//! `mlt convert <city model> <dir>`: tile the buildings of `CityGML` and `CityJSON` files into a `z/x/y.mlt` tree of v2 `Meshes` layers.

mod citygml;
mod cityjson;
mod crs;
mod lods;
mod merge;
mod mesh;
mod model;
mod orient;
mod simplify;
mod tiler;
mod triangulate;

use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result as AnyResult, bail};
use mlt_core::ZStep;
use mlt_core::geojson::PropertySchema;
use rayon::prelude::*;
use serde_json::Value;
use walkdir::WalkDir;

use self::crs::{Reprojector, epsg_of_srs_name};
use self::model::{Building, Lod, SourceDocument};
use self::tiler::{BUILDINGS, GML_ID, LayerSpec};
use self::triangulate::Triangulator;
use super::{ConvertArgs, Reencoder};

/// The z grid without `--z-step`: 1 cm.
const DEFAULT_Z_STEP: i8 = -2;
/// How far, in degrees, a file must lie from the other files in the CRS it names to be read in the CRS most files name.
const MISLABELLED_MIN_DEGREES: f64 = 1.0;

/// The format of a city model file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Format {
    CityGml,
    CityJson,
}

impl Format {
    /// The format of the file `path` by its name: `.gml` or `.xml` for `CityGML`, `.city.json` for `CityJSON`.
    fn of_file(path: &Path) -> Option<Self> {
        let name = path.file_name()?.to_str()?;
        let ends_with = |suffix: &str| {
            name.len()
                .checked_sub(suffix.len())
                .and_then(|at| name.get(at..))
                .is_some_and(|end| end.eq_ignore_ascii_case(suffix))
        };
        if ends_with(".city.json") {
            Some(Self::CityJson)
        } else if ends_with(".gml") || ends_with(".xml") {
            Some(Self::CityGml)
        } else {
            None
        }
    }

    pub(super) fn name(self) -> &'static str {
        match self {
            Self::CityGml => "CityGML",
            Self::CityJson => "CityJSON",
        }
    }

    /// The member that names the CRS of a file.
    fn crs_member(self) -> &'static str {
        match self {
            Self::CityGml => "srsName",
            Self::CityJson => "referenceSystem",
        }
    }

    fn parse(self, path: &Path) -> AnyResult<SourceDocument> {
        match self {
            Self::CityGml => citygml::parse(BufReader::new(File::open(path)?)),
            Self::CityJson => cityjson::parse(&fs::read(path)?),
        }
    }
}

/// The format of the city model file `path`, or of the first city model file under the directory `path` if it comes before any tile.
pub(super) fn input_format(path: &Path) -> AnyResult<Option<Format>> {
    if path.is_file() {
        return Ok(Format::of_file(path));
    }
    if !path.is_dir() {
        return Ok(None);
    }
    for file in files_under(path) {
        let file = file?;
        if let Some(format) = Format::of_file(&file) {
            return Ok(Some(format));
        }
        if matches!(
            file.extension().and_then(|x| x.to_str()),
            Some("mlt" | "mvt" | "pbf")
        ) {
            return Ok(None);
        }
    }
    Ok(None)
}

/// The files under the directory `dir`, sorted by name.
fn files_under(dir: &Path) -> impl Iterator<Item = AnyResult<PathBuf>> + '_ {
    WalkDir::new(dir)
        .sort_by_file_name()
        .into_iter()
        .filter_map(move |entry| match entry {
            Ok(entry) => entry.file_type().is_file().then(|| Ok(entry.into_path())),
            Err(e) => Some(Err(e).with_context(|| format!("searching {}", dir.display()))),
        })
}

/// The city model file `input`, or the city model files under the directory `input`, sorted, each with its format.
fn city_files(input: &Path) -> AnyResult<Vec<(PathBuf, Format)>> {
    if input.is_file() {
        let format = Format::of_file(input).context("the input is no city model file")?;
        return Ok(vec![(input.to_owned(), format)]);
    }
    files_under(input)
        .filter_map(|file| match file {
            Ok(file) => Format::of_file(&file).map(|format| Ok((file, format))),
            Err(e) => Some(Err(e)),
        })
        .collect()
}

/// The buildings of a file, the CRS they were read in, and the file's first position in it.
struct SourceFile {
    epsg: u16,
    first: [f64; 3],
    buildings: Vec<Building>,
}

/// A file that names `declared` as its CRS, but whose coordinates fit `epsg`, the CRS most files name.
struct Mislabelled {
    index: usize,
    declared: u16,
    epsg: u16,
}

/// How files are read into buildings.
struct Reader<'a> {
    /// The only LOD read, if `--lod` names one.
    lod: Option<Lod>,
    /// The attribute holding each building's ground height, if `--ground-attribute` names one.
    ground_attribute: Option<&'a str>,
}

impl Reader<'_> {
    /// The buildings of the file at `path`, triangulated and reprojected from `epsg` or, without it, from the CRS the file names.
    /// `None` if the file holds no position.
    fn read(
        &self,
        path: &Path,
        format: Format,
        epsg: Option<u16>,
    ) -> AnyResult<Option<SourceFile>> {
        let mut document = format.parse(path)?;
        if document.city_objects.is_empty() {
            return Ok(None);
        }
        let epsg = if let Some(epsg) = epsg {
            epsg
        } else {
            let member = format.crs_member();
            let crs = document
                .crs
                .as_deref()
                .with_context(|| format!("the file names no CRS in its {member}"))?;
            epsg_of_srs_name(crs)
                .with_context(|| format!("the {member} {crs:?} names no EPSG code"))?
        };
        let reprojector = Reprojector::new(epsg)?;
        if reprojector.geographic && document.lon_first {
            document.swap_axes();
        }
        let Some(first) = document.first() else {
            return Ok(None);
        };
        let mut triangulator = Triangulator::default();
        let mut buildings = Vec::with_capacity(document.city_objects.len());
        for object in document.city_objects {
            let Some(mut building) =
                Building::new(object, self.lod, &reprojector, &mut triangulator)?
            else {
                continue;
            };
            if let Some(name) = self.ground_attribute {
                let ground = building.attributes.get(name).and_then(Value::as_f64);
                building.ground = Some(ground.with_context(|| {
                    format!(
                        "object {} has no numeric {name} attribute",
                        building.id.as_deref().unwrap_or("without an id")
                    )
                })?);
            }
            buildings.push(building);
        }
        Ok(Some(SourceFile {
            epsg,
            first,
            buildings,
        }))
    }
}

/// Tile the city model file or directory `args.input` into the directory `args.output`.
/// [`super::convert`] has already checked the flags and that the output does not exist.
pub(super) fn convert(args: &ConvertArgs, reencoder: &Reencoder) -> AnyResult<()> {
    let (input, output) = (args.input.as_path(), args.output.as_path());
    let (min_zoom, max_zoom) = args.tiled_zooms()?;
    let name = args.tiled_layer_name()?;
    let z_step = ZStep::new(args.z_step.unwrap_or(DEFAULT_Z_STEP))?;
    let lod = args
        .lod
        .as_deref()
        .map(str::parse::<Lod>)
        .transpose()
        .context("reading --lod")?;
    let reader = Reader {
        lod,
        ground_attribute: args.ground_attribute.as_deref(),
    };

    let files = city_files(input)?;
    let mut parsed: Vec<Option<SourceFile>> = files
        .par_iter()
        .map(|(path, format)| {
            reader
                .read(path, *format, None)
                .with_context(|| format!("reading {}", path.display()))
        })
        .collect::<AnyResult<_>>()?;
    if args.fix_mislabelled_crs {
        let reread: Vec<(usize, Option<SourceFile>)> = mislabelled(&parsed)?
            .into_par_iter()
            .map(|m| {
                let (path, format) = &files[m.index];
                eprintln!(
                    "warning: {} names EPSG:{}, but its coordinates lie near the other files in EPSG:{}, so it is read in that",
                    path.display(),
                    m.declared,
                    m.epsg,
                );
                let file = reader
                    .read(path, *format, Some(m.epsg))
                    .with_context(|| format!("reading {}", path.display()))?;
                Ok((m.index, file))
            })
            .collect::<AnyResult<_>>()?;
        for (i, file) in reread {
            parsed[i] = file;
        }
    }
    let buildings: Vec<Building> = parsed
        .into_iter()
        .flatten()
        .flat_map(|f| f.buildings)
        .collect();

    let spec = LayerSpec {
        name,
        z_step,
        schema: schema(&buildings)?,
    };
    let written = tiler::tile(&buildings, &spec, min_zoom..=max_zoom, output, reencoder)?;
    if written == 0 {
        let at_lod = lod.map_or_else(String::new, |lod| format!(" at LoD {lod}"));
        eprintln!(
            "note: no tiles were written (the input held no 3D buildings{at_lod}, or none reached the minimum pixel size at zooms {min_zoom}..={max_zoom})"
        );
    }
    eprintln!(
        "{} -> {}: wrote {written} tile(s) holding {} building(s) of {} file(s) at zooms {min_zoom}..={max_zoom}",
        input.display(),
        output.display(),
        buildings.len(),
        files.len(),
    );
    Ok(())
}

/// The property columns of the attributes of `buildings`, none of which may take the name of a column of the tiler's own.
fn schema(buildings: &[Building]) -> AnyResult<PropertySchema> {
    for building in buildings {
        if let Some(reserved) = [GML_ID, BUILDINGS]
            .into_iter()
            .find(|&name| building.attributes.contains_key(name))
        {
            bail!(
                "object {} has an attribute named {reserved}, which convert writes a column of its own under",
                building.id.as_deref().unwrap_or("without an id")
            );
        }
    }
    Ok(PropertySchema::infer(
        buildings.iter().map(|b| b.attributes.iter()),
    )?)
}

/// The `files` whose first position lies over [`MISLABELLED_MIN_DEGREES`] from the median of those of the files in the CRS most of them name, but less than a quarter as far when read in that CRS.
fn mislabelled(files: &[Option<SourceFile>]) -> AnyResult<Vec<Mislabelled>> {
    let mut counts: BTreeMap<u16, usize> = BTreeMap::new();
    for file in files.iter().flatten() {
        *counts.entry(file.epsg).or_default() += 1;
    }
    let Some((&common, _)) = counts.iter().max_by_key(|&(&epsg, &n)| (n, Reverse(epsg))) else {
        return Ok(Vec::new());
    };
    if counts.len() < 2 {
        return Ok(Vec::new());
    }
    let reprojectors: BTreeMap<u16, Reprojector> = counts
        .keys()
        .map(|&epsg| Ok((epsg, Reprojector::new(epsg)?)))
        .collect::<AnyResult<_>>()?;
    let degrees = |epsg: u16, p: [f64; 3]| {
        let mut points = [p];
        reprojectors[&epsg].to_geodetic(&mut points).ok()?;
        Some([points[0][0].to_degrees(), points[0][1].to_degrees()])
    };
    let (mut lons, mut lats): (Vec<f64>, Vec<f64>) = files
        .iter()
        .flatten()
        .filter(|f| f.epsg == common)
        .filter_map(|f| degrees(common, f.first))
        .map(|[lon, lat]| (lon, lat))
        .unzip();
    let (Some(lon), Some(lat)) = (median(&mut lons), median(&mut lats)) else {
        return Ok(Vec::new());
    };
    let distance = |p: Option<[f64; 2]>| {
        p.map_or(f64::INFINITY, |[x, y]| {
            ((x - lon) * lat.to_radians().cos()).hypot(y - lat)
        })
    };
    Ok(files
        .iter()
        .enumerate()
        .filter_map(|(index, file)| {
            let file = file.as_ref().filter(|f| f.epsg != common)?;
            let off = distance(degrees(file.epsg, file.first));
            let near = distance(degrees(common, file.first));
            (off > MISLABELLED_MIN_DEGREES && near < off / 4.0).then_some(Mislabelled {
                index,
                declared: file.epsg,
                epsg: common,
            })
        })
        .collect())
}

fn median(values: &mut [f64]) -> Option<f64> {
    values.sort_unstable_by(f64::total_cmp);
    values.get(values.len() / 2).copied()
}
