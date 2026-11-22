//! `mlt convert <citygml> <dir>`: tile the buildings of `CityGML` files into a `z/x/y.mlt` tree of v2 `Meshes` layers.

mod citygml;
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
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result as AnyResult};
use mlt_core::ZStep;
use rayon::prelude::*;
use walkdir::WalkDir;

use self::crs::{Reprojector, epsg_of_srs_name};
use self::model::Building;
use self::tiler::LayerSpec;
use self::triangulate::Triangulator;
use super::{ConvertArgs, Reencoder};

/// The z grid without `--z-step`: 1 cm.
const DEFAULT_Z_STEP: i8 = -2;
/// How far, in degrees, a file must lie from the other files in the CRS it names to be read in the CRS most files name.
const MISLABELLED_MIN_DEGREES: f64 = 1.0;

/// Whether `path` is a `CityGML` file, or a directory whose first `.gml` or `.xml` file comes before any tile.
pub(super) fn is_citygml(path: &Path) -> AnyResult<bool> {
    if path.is_file() {
        return Ok(is_gml(path));
    }
    if !path.is_dir() {
        return Ok(false);
    }
    for file in files_under(path) {
        let file = file?;
        match file.extension().and_then(|x| x.to_str()) {
            Some("mlt" | "mvt" | "pbf") => return Ok(false),
            _ if is_gml(&file) => return Ok(true),
            _ => {}
        }
    }
    Ok(false)
}

fn is_gml(path: &Path) -> bool {
    path.extension()
        .is_some_and(|x| x.eq_ignore_ascii_case("gml") || x.eq_ignore_ascii_case("xml"))
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

/// The `CityGML` file `input`, or the `.gml` and `.xml` files under the directory `input`, sorted.
fn gml_files(input: &Path) -> AnyResult<Vec<PathBuf>> {
    if input.is_file() {
        return Ok(vec![input.to_owned()]);
    }
    files_under(input)
        .filter(|file| !matches!(file, Ok(f) if !is_gml(f)))
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

/// The buildings of the `CityGML` file at `path`, triangulated and reprojected from `epsg` or, without it, from the file's `srsName`.
/// `None` if the file holds no position.
fn read(path: &Path, epsg: Option<u16>) -> AnyResult<Option<SourceFile>> {
    let document = citygml::parse(BufReader::new(File::open(path)?))?;
    if document.city_objects.is_empty() {
        return Ok(None);
    }
    let epsg = if let Some(epsg) = epsg {
        epsg
    } else {
        let crs = document
            .crs
            .as_deref()
            .context("the file names no CRS in its srsName")?;
        epsg_of_srs_name(crs).with_context(|| format!("the srsName {crs:?} names no EPSG code"))?
    };
    let reprojector = Reprojector::new(epsg)?;
    let Some(first) = document.first() else {
        return Ok(None);
    };
    let mut triangulator = Triangulator::default();
    let mut buildings = Vec::with_capacity(document.city_objects.len());
    for object in document.city_objects {
        if let Some(building) = Building::new(object, &reprojector, &mut triangulator)? {
            buildings.push(building);
        }
    }
    Ok(Some(SourceFile {
        epsg,
        first,
        buildings,
    }))
}

/// Tile the `CityGML` file or directory `args.input` into the directory `args.output`.
/// [`super::convert`] has already checked the flags and that the output does not exist.
pub(super) fn convert(args: &ConvertArgs, reencoder: &Reencoder) -> AnyResult<()> {
    let (input, output) = (args.input.as_path(), args.output.as_path());
    let (min_zoom, max_zoom) = args.tiled_zooms()?;
    let name = args.tiled_layer_name()?;
    let z_step = ZStep::new(args.z_step.unwrap_or(DEFAULT_Z_STEP))?;

    let files = gml_files(input)?;
    let mut parsed: Vec<Option<SourceFile>> = files
        .par_iter()
        .map(|path| read(path, None).with_context(|| format!("reading {}", path.display())))
        .collect::<AnyResult<_>>()?;
    if args.fix_mislabelled_crs {
        let reread: Vec<(usize, Option<SourceFile>)> = mislabelled(&parsed)?
            .into_par_iter()
            .map(|m| {
                let path = &files[m.index];
                eprintln!(
                    "warning: {} names EPSG:{}, but its coordinates lie near the other files in EPSG:{}, so it is read in that",
                    path.display(),
                    m.declared,
                    m.epsg,
                );
                let file = read(path, Some(m.epsg))
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

    let spec = LayerSpec { name, z_step };
    let written = tiler::tile(&buildings, &spec, min_zoom..=max_zoom, output, reencoder)?;
    if written == 0 {
        eprintln!(
            "note: no tiles were written (the input held no 3D buildings, or none reached the minimum pixel size at zooms {min_zoom}..={max_zoom})"
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
