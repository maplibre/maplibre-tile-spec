//! `mlt convert <citygml> <dir>`: tile the buildings of `CityGML` files into an MLT `z/x/y.mlt`
//! tree of v2 `Meshes` layers (XYZ scheme, `y = 0` at the north).
//!
//! Each building, bridge or tunnel becomes one mesh per zoom, reprojected from its file's
//! `srsName`. As `docs/specification/v2.md` asks, a mesh is stored whole in the tile of each zoom
//! that holds the centre of its bounding box, and its z values are heights above its lowest
//! vertex. Its vertices are welded on the tile's grid, and each triangle carries the `part`
//! m-value of the semantic surface it lies in: wall, roof, ground, window or door. A surface
//! without semantics is a roof where it faces up and a wall otherwise.
//!
//! The highest zoom holds the highest LOD of each part of a building. The zoom below holds its
//! LOD2: the highest LOD of 2 or less of each part, or else its highest LOD without windows, doors,
//! installations and holes, its vertices clustered in cells of half a pixel. Lower zooms hold a
//! block: the outline of its ground, or its bounding box without one, clustered in cells of a
//! pixel and raised from 0 to its top as walls and a roof. A building under 2 pixels across has
//! no block. A pixel is a 512th of a tile. From three zooms below the highest, the buildings of a
//! tile lower than 40 m are merged where they touch: the cells of a pixel their grounds cover at
//! least half of are joined with their neighbours in the same 10 m band of tops, and each group is
//! one block at its mean top, with a `buildings` property counting the buildings it holds. Its
//! outline is simplified to within a pixel, and it has no wall where it stands less than a pixel
//! above its neighbour or the ground. Higher buildings stay blocks of their own there, whatever
//! their size.
//!
//! Every mesh but the merged blocks is then simplified by meshoptimizer: within 5 cm at the highest
//! zoom and a third of a pixel below, its pieces smaller than that removed.
//!
//! A file whose coordinates lie far from the input's other files in the CRS it declares, but near
//! them in the CRS most files declare, is read in the latter: some Hamburg files declare
//! EPSG:25833 for UTM zone 32 coordinates.

mod crs;
mod merge;
mod parser;
mod simplify;
mod triangulate;

use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::f64::consts::PI;
use std::fs::{self, File as FsFile};
use std::io::BufReader;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result as AnyResult};
use martin_tile_utils::EARTH_CIRCUMFERENCE;
use mlt_core::geo_types::{Geometry, MultiPoint, Point};
use mlt_core::{HeightMode, MValue, MValueScope, PropKind, PropValue, TileLayer, ZAxis, ZStep};
use rayon::prelude::*;
use walkdir::WalkDir;

use self::crs::{Reprojector, epsg_of_srs_name};
use self::merge::{Footprint, merge};
use self::parser::{Lods, Part};
use self::simplify::{Parts, simplify};
use super::from_geojson::{decode_with_default_budget, is_memory_limit, mercator_unit, tile_index};
use super::{ConvertArgs, Reencoder};

/// The z grid without `--z-step`: 1 cm.
const DEFAULT_Z_STEP: i8 = -2;
/// The finest grid unit the extent of a tile aims for, in metres.
const UNIT_METRES: f64 = 0.01;
/// The largest extent a v2 layer header can name.
const MAX_EXTENT: u32 = 1 << 21;
/// The up component of a unit normal above which a surface without semantics is a roof, about
/// 70 degrees from vertical.
const ROOF_MIN_UP: f64 = 0.34;
/// How far, in degrees, a file must lie from the input's other files in the CRS it declares to
/// be read in the CRS most files declare.
const MISLABELLED_MIN_DEGREES: f64 = 1.0;
/// The pixels across a tile that the detail of zooms below the highest is measured in.
const TILE_PIXELS: f64 = 512.0;
/// The cell, in pixels, the LOD2 mesh one zoom below the highest clusters its vertices in.
const LOD2_CELL_PIXELS: f64 = 0.5;
/// The cell, in pixels, a block clusters the corners of its footprint in.
const BLOCK_CELL_PIXELS: f64 = 1.0;
/// The longer side of its bounding box, in pixels, under which a building has no block.
const BLOCK_MIN_PIXELS: f64 = 2.0;
/// How many zooms below the highest the blocks of a tile's buildings start to be merged.
const MERGE_BELOW: u8 = 3;
/// The top, in metres above its ground, from which a building is never merged.
const LANDMARK_MIN_METRES: f64 = 40.0;
/// The height, in pixels, under which merged blocks have no wall between them or to the ground.
const WALL_MIN_PIXELS: f64 = 1.0;
/// The error, in metres, the meshes of the highest zoom are simplified within: close to lossless.
const HIGHEST_ERROR_METRES: f64 = 0.05;
/// The error, in pixels, the meshes and blocks of lower zooms are simplified within.
const ERROR_PIXELS: f64 = 1.0 / 3.0;

/// Whether `path` is a `CityGML` file, or a directory whose first `.gml` file comes before any tile.
pub(super) fn is_citygml(path: &Path) -> AnyResult<bool> {
    if path.is_file() {
        return Ok(is_gml(path));
    }
    if !path.is_dir() {
        return Ok(false);
    }
    for entry in WalkDir::new(path).sort_by_file_name() {
        let entry = entry.with_context(|| format!("searching {}", path.display()))?;
        if !entry.file_type().is_file() {
            continue;
        }
        match entry.path().extension().and_then(|x| x.to_str()) {
            Some("mlt" | "mvt" | "pbf") => return Ok(false),
            _ if is_gml(entry.path()) => return Ok(true),
            _ => {}
        }
    }
    Ok(false)
}

fn is_gml(path: &Path) -> bool {
    path.extension()
        .is_some_and(|x| x.eq_ignore_ascii_case("gml") || x.eq_ignore_ascii_case("xml"))
}

/// A surface of one part, with geodetic positions: longitude and latitude in radians, and metres up.
struct Surface {
    part: Part,
    lods: Lods,
    positions: Vec<[f64; 3]>,
    triangles: Vec<[u32; 3]>,
}

struct Building {
    gml_id: Option<String>,
    surfaces: Vec<Surface>,
}

/// The buildings of a file, the CRS they were read in and the file's first position in it.
struct File {
    epsg: u16,
    first: [f64; 3],
    buildings: Vec<Building>,
}

/// Tile the `CityGML` file or directory `args.input` into the directory `args.output`.
/// [`super::convert`] has already checked the flags and that the output does not exist.
pub(super) fn convert(args: &ConvertArgs, reencoder: &Reencoder) -> AnyResult<()> {
    let (input, output) = (args.input.as_path(), args.output.as_path());
    let (min_zoom, max_zoom) = args.tiled_zooms()?;
    let name = args.tiled_layer_name()?;
    let z_step = ZStep::new(args.z_step.unwrap_or(DEFAULT_Z_STEP))?;

    let files = files(input)?;
    let mut parsed: Vec<Option<File>> = files
        .par_iter()
        .map(|path| read(path, None).with_context(|| format!("reading {}", path.display())))
        .collect::<AnyResult<_>>()?;
    if args.fix_mislabelled_crs {
        let reread: Vec<(usize, Option<File>)> = mislabelled(&parsed)?
            .into_par_iter()
            .map(|(i, declared, epsg)| {
                let path = &files[i];
                eprintln!(
                    "warning: {} names EPSG:{declared}, but its coordinates lie near the other files in EPSG:{epsg}, so it is read in that",
                    path.display(),
                );
                let file = read(path, Some(epsg))
                    .with_context(|| format!("reading {}", path.display()))?;
                Ok((i, file))
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

    // One zoom at a time, so that only its placed meshes are held at once.
    let mut written = 0;
    for zoom in min_zoom..=max_zoom {
        let placed: Vec<Option<(Tile, Placed)>> = buildings
            .par_iter()
            .map(|building| place(building, zoom, max_zoom, z_step))
            .collect();
        let mut tiles: BTreeMap<Tile, Vec<(u64, &Building, Placed)>> = BTreeMap::new();
        for ((id, building), placed) in (0..).zip(&buildings).zip(placed) {
            if let Some((tile, placed)) = placed {
                tiles.entry(tile).or_default().push((id, building, placed));
            }
        }
        written += tiles
            .into_par_iter()
            .map(|(tile, placed)| -> AnyResult<usize> {
                let at = format!("{}/{}/{}", tile.zoom, tile.x, tile.y);
                let Some(layer) = layer(&name, tile, z_step, placed)? else {
                    return Ok(0);
                };
                let checked = reencoder
                    .encode(layer)
                    .with_context(|| format!("encoding tile {at}"))
                    .and_then(|bytes| {
                        if !reencoder.verify && !bytes.is_empty() {
                            decode_with_default_budget(&bytes)
                                .with_context(|| format!("decoding tile {at} back"))?;
                        }
                        Ok(bytes)
                    });
                let bytes = match checked {
                    Ok(bytes) => bytes,
                    Err(e) if is_memory_limit(&e) => {
                        eprintln!(
                            "warning: skipping tile {at}: exceeds default decoder memory budget"
                        );
                        return Ok(0);
                    }
                    Err(e) => return Err(e),
                };
                if bytes.is_empty() {
                    return Ok(0);
                }
                let dir = output.join(tile.zoom.to_string()).join(tile.x.to_string());
                fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
                let path = dir.join(format!("{}.mlt", tile.y));
                fs::write(&path, bytes).with_context(|| format!("writing {}", path.display()))?;
                Ok(1)
            })
            .try_reduce(|| 0, |a, b| Ok(a + b))?;
    }
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

/// The `CityGML` file `input`, or the `.gml` files under the directory `input`, sorted.
fn files(input: &Path) -> AnyResult<Vec<PathBuf>> {
    if input.is_file() {
        return Ok(vec![input.to_owned()]);
    }
    let mut files = Vec::new();
    for entry in WalkDir::new(input).sort_by_file_name() {
        let entry = entry.with_context(|| format!("searching {}", input.display()))?;
        if entry.file_type().is_file() && is_gml(entry.path()) {
            files.push(entry.into_path());
        }
    }
    Ok(files)
}

/// The buildings of the `CityGML` file at `path`, triangulated and reprojected from `epsg` or,
/// without it, from the file's `srsName`. `None` if the file holds no position.
fn read(path: &Path, epsg: Option<u16>) -> AnyResult<Option<File>> {
    let reader = BufReader::new(FsFile::open(path)?);
    let document = parser::parse_reader(reader)?;
    if document.buildings.is_empty() {
        return Ok(None);
    }
    let epsg = if let Some(epsg) = epsg {
        epsg
    } else {
        let srs_name = document
            .srs_name
            .context("the file names no CRS in an srsName")?;
        epsg_of_srs_name(&srs_name)
            .with_context(|| format!("the srsName {srs_name:?} names no EPSG code"))?
    };
    let Some(first) = document
        .buildings
        .iter()
        .flat_map(|b| &b.polygons)
        .find_map(|p| p.points.first().copied())
    else {
        return Ok(None);
    };
    let reprojector = Reprojector::new(epsg)?;
    let mut buildings = Vec::with_capacity(document.buildings.len());
    for building in document.buildings {
        let mut surfaces = Vec::with_capacity(building.polygons.len());
        for polygon in building.polygons {
            let mut triangles = Vec::new();
            let Some(normal) = triangulate::polygon(
                &polygon.points,
                &polygon.ends,
                reprojector.geographic,
                &mut triangles,
            ) else {
                continue;
            };
            let part = polygon.part.unwrap_or_else(|| {
                let up = normal[2] / triangulate::dot(normal, normal).sqrt();
                if up > ROOF_MIN_UP {
                    Part::Roof
                } else if up < -ROOF_MIN_UP {
                    Part::Ground
                } else {
                    Part::Wall
                }
            });
            let mut positions = polygon.points;
            reprojector.to_geodetic(&mut positions)?;
            surfaces.push(Surface {
                part,
                lods: polygon.lods,
                positions,
                triangles,
            });
        }
        if !surfaces.is_empty() {
            buildings.push(Building {
                gml_id: building.gml_id,
                surfaces,
            });
        }
    }
    Ok(Some(File {
        epsg,
        first,
        buildings,
    }))
}

/// The index of each of `files` whose first position lies more than [`MISLABELLED_MIN_DEGREES`]
/// from the median of those of the files in the CRS most of them declare, but less than a quarter
/// as far when read in that CRS, with the CRS it declares and that CRS.
fn mislabelled(files: &[Option<File>]) -> AnyResult<Vec<(usize, u16, u16)>> {
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
        .filter_map(|(i, file)| {
            let file = file.as_ref().filter(|f| f.epsg != common)?;
            let off = distance(degrees(file.epsg, file.first));
            let near = distance(degrees(common, file.first));
            (off > MISLABELLED_MIN_DEGREES && near < off / 4.0).then_some((i, file.epsg, common))
        })
        .collect())
}

fn median(values: &mut [f64]) -> Option<f64> {
    values.sort_unstable_by(f64::total_cmp);
    values.get(values.len() / 2).copied()
}

/// A tile of the XYZ scheme, `y = 0` at the north.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Tile {
    zoom: u8,
    x: u32,
    y: u32,
}

impl Tile {
    /// The tile of `zoom` holding the Web Mercator position `[x, y]` of [`unit()`].
    fn of([x, y]: [f64; 2], zoom: u8) -> Self {
        let n = 1 << zoom;
        let (x, y) = (tile_index(x, n), tile_index(y, n));
        Self { zoom, x, y }
    }

    /// The metres across the middle of the tile.
    fn width(self) -> f64 {
        let n = f64::from(1_u32 << self.zoom);
        let lat = (PI * (1.0 - 2.0 * (f64::from(self.y) + 0.5) / n))
            .sinh()
            .atan();
        EARTH_CIRCUMFERENCE * lat.cos() / n
    }

    /// The extent whose unit is between 1 and 2 cm across the middle of the tile.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the exponent is clamped to 6..=21"
    )]
    fn extent(self) -> u32 {
        let exponent = (self.width() / UNIT_METRES).log2().floor().clamp(6.0, 21.0) as u32;
        (1 << exponent).min(MAX_EXTENT)
    }

    /// `[x, y]` on this tile's grid of `extent` of the Web Mercator position `[x, y]`, rounded
    /// half to even, or `None` if it does not fit an `i32`.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the rounded values are checked to fit"
    )]
    fn grid(self, [ux, uy]: [f64; 2], extent: u32) -> Option<[i32; 2]> {
        let n = f64::from(1_u32 << self.zoom);
        let extent = f64::from(extent);
        let x = ((ux * n - f64::from(self.x)) * extent).round_ties_even();
        let y = ((uy * n - f64::from(self.y)) * extent).round_ties_even();
        let fits = |v: f64| (f64::from(i32::MIN)..=f64::from(i32::MAX)).contains(&v);
        (fits(x) && fits(y)).then_some([x as i32, y as i32])
    }
}

/// Web Mercator `[x, y]` of `[lon, lat]` in radians, the world spanning `[0, 1]`.
fn unit(lon: f64, lat: f64) -> [f64; 2] {
    let (x, y) = mercator_unit(lon.to_degrees(), lat.to_degrees());
    [x, y]
}

/// A building on a tile's grid: its vertices, the triangles between them that still have three
/// distinct corners, and the part of each triangle.
#[derive(Debug, PartialEq)]
struct Mesh {
    xyz: Vec<[i32; 3]>,
    triangles: Vec<[u32; 3]>,
    parts: Vec<Part>,
}

impl Mesh {
    /// Welds `surfaces`, `points` holding each one's vertices on the grid, so that surfaces share
    /// the vertices they meet at. Vertices keep their order, and those no triangle names are dropped.
    /// `None` if they are more than a `u32` indexes.
    fn weld<'a>(
        surfaces: impl IntoIterator<Item = &'a Surface>,
        points: &[Vec<[i32; 3]>],
    ) -> Option<Self> {
        let mut ids: HashMap<[i32; 3], u32> = HashMap::new();
        let mut welded = Vec::new();
        let mut triangles = Vec::new();
        let mut parts = Vec::new();
        for (surface, points) in surfaces.into_iter().zip(points) {
            let local: Vec<u32> = points
                .iter()
                .map(|&p| {
                    let next = u32::try_from(welded.len()).ok()?;
                    Some(*ids.entry(p).or_insert_with(|| {
                        welded.push(p);
                        next
                    }))
                })
                .collect::<Option<_>>()?;
            for t in &surface.triangles {
                let [a, b, c] = t.map(|i| local[i as usize]);
                if a != b && b != c && c != a {
                    triangles.push([a, b, c]);
                    parts.push(surface.part);
                }
            }
        }
        let mut used = vec![false; welded.len()];
        for &i in triangles.iter().flatten() {
            used[i as usize] = true;
        }
        let mut renumbered = vec![0; welded.len()];
        let mut xyz = Vec::new();
        for (i, p) in welded.into_iter().enumerate() {
            if used[i] {
                renumbered[i] = u32::try_from(xyz.len()).ok()?;
                xyz.push(p);
            }
        }
        for t in &mut triangles {
            *t = t.map(|i| renumbered[i as usize]);
        }
        Some(Self {
            xyz,
            triangles,
            parts,
        })
    }
}

/// A building on a tile: its mesh, or its footprint, to be merged with those of the tile's others.
enum Placed {
    Mesh(Mesh),
    Footprint(Footprint),
}

/// `building` at `zoom` on the tile that holds the centre of its bounding box: at its highest LOD
/// at `max_zoom`, at LOD2 one zoom below and as a block below that, or from [`MERGE_BELOW`] zooms
/// below as its footprint unless it is a landmark.
/// Nothing is placed where the building has no triangle, does not fit the tile's grid, or has
/// more vertices or triangle corners than a `u32` indexes.
fn place(building: &Building, zoom: u8, max_zoom: u8, z_step: ZStep) -> Option<(Tile, Placed)> {
    let below = max_zoom - zoom;
    let lods = if below == 0 {
        Lods::Lod2
    } else {
        Lods::Highest
    };
    let view = View::new(building.surfaces.iter().filter(|s| s.lods != lods))?;
    let tile = Tile::of(view.centre, zoom);
    let extent = tile.extent();
    let pixel = f64::from(extent) / TILE_PIXELS;
    if below >= MERGE_BELOW {
        let abs_points = view.points_with_datum(tile, extent, z_step, 0.0)?;
        let footprint = Footprint::new(&view.surfaces, &abs_points)?;
        if f64::from(footprint.height()) * z_step.metres() < LANDMARK_MIN_METRES {
            return Some((tile, Placed::Footprint(footprint)));
        }
    }
    let points = view.points(tile, extent, z_step)?;
    let mesh = match below {
        0 => Mesh::weld(view.surfaces.iter().copied(), &points),
        1 => {
            let z = tile.width() / TILE_PIXELS / z_step.metres();
            let cell = [pixel, pixel, z].map(|c| c * LOD2_CELL_PIXELS);
            cluster(&view.surfaces, &points, cell)
        }
        _ if below < MERGE_BELOW => block(&view.surfaces, &points, pixel, BLOCK_MIN_PIXELS),
        _ => block(&view.surfaces, &points, pixel, 0.0),
    };
    let metres = tile.width() / f64::from(extent);
    let (error, parts) = match below {
        0 => (HIGHEST_ERROR_METRES, Parts::Seamed),
        1 => (ERROR_PIXELS * tile.width() / TILE_PIXELS, Parts::Seamed),
        _ => (ERROR_PIXELS * tile.width() / TILE_PIXELS, Parts::Faced),
    };
    let mesh = simplify(&mesh?, [metres, metres, z_step.metres()], error, parts)?;
    (!mesh.triangles.is_empty()).then_some((tile, Placed::Mesh(mesh)))
}

/// Some surfaces of a building, the centre of their Web Mercator `(x, y)` bounding box in units
/// of [`unit()`], and their ground elevation at that centre.
struct View<'a> {
    surfaces: Vec<&'a Surface>,
    units: Vec<Vec<[f64; 2]>>,
    centre: [f64; 2],
    ground: f64,
}

impl<'a> View<'a> {
    /// `None` without a surface.
    fn new(surfaces: impl Iterator<Item = &'a Surface>) -> Option<Self> {
        let surfaces: Vec<&Surface> = surfaces.collect();
        if surfaces.is_empty() {
            return None;
        }
        let units: Vec<Vec<[f64; 2]>> = surfaces
            .iter()
            .map(|s| s.positions.iter().map(|p| unit(p[0], p[1])).collect())
            .collect();
        let all_units = || units.iter().flatten();
        let fold_u =
            |f: fn(f64, f64) -> f64, i: usize, init: f64| all_units().map(|u| u[i]).fold(init, f);
        let [ux0, uy0] = [0, 1].map(|i| fold_u(f64::min, i, f64::INFINITY));
        let [ux1, uy1] = [0, 1].map(|i| fold_u(f64::max, i, f64::NEG_INFINITY));
        let centre = [f64::midpoint(ux0, ux1), f64::midpoint(uy0, uy1)];

        // Prefer the GroundSurface vertex nearest the Web Mercator bbox midpoint as `z = 0`,
        // so underground basements stay at `z < 0` and sloped buildings match the spec's
        // centre-ground datum; fall back to the lowest vertex when no GroundSurface is given.
        let ground_at_centre = surfaces
            .iter()
            .zip(&units)
            .filter(|(s, _)| s.part == Part::Ground)
            .flat_map(|(s, u)| s.positions.iter().zip(u))
            .min_by(|(_, ua), (_, ub)| {
                let da = (ua[0] - centre[0]).hypot(ua[1] - centre[1]);
                let db = (ub[0] - centre[0]).hypot(ub[1] - centre[1]);
                da.total_cmp(&db)
            })
            .map(|(p, _)| p[2]);
        let ground = ground_at_centre.unwrap_or_else(|| {
            surfaces
                .iter()
                .flat_map(|s| &s.positions)
                .map(|p| p[2])
                .fold(f64::INFINITY, f64::min)
        });
        Some(Self {
            surfaces,
            units,
            centre,
            ground,
        })
    }

    /// The vertices of each surface on `tile`'s grid of `extent`, z above `self.ground`, or `None`
    /// if one does not fit.
    fn points(&self, tile: Tile, extent: u32, z_step: ZStep) -> Option<Vec<Vec<[i32; 3]>>> {
        self.points_with_datum(tile, extent, z_step, self.ground)
    }

    fn points_with_datum(
        &self,
        tile: Tile,
        extent: u32,
        z_step: ZStep,
        datum: f64,
    ) -> Option<Vec<Vec<[i32; 3]>>> {
        self.surfaces
            .iter()
            .zip(&self.units)
            .map(|(surface, units)| {
                surface
                    .positions
                    .iter()
                    .zip(units)
                    .map(|(p, &u)| {
                        let [x, y] = tile.grid(u, extent)?;
                        Some([x, y, z_step.z(p[2] - datum)?])
                    })
                    .collect()
            })
            .collect()
    }
}

/// `surfaces` with each of their `points` moved to the nearest multiple of `cell` in x, y and z,
/// welded, keeping one of the triangles that wind alike through the same vertices. `None` as
/// [`Mesh::weld`] says.
fn cluster(surfaces: &[&Surface], points: &[Vec<[i32; 3]>], cell: [f64; 3]) -> Option<Mesh> {
    let snapped: Vec<Vec<[i32; 3]>> = points
        .iter()
        .map(|p| {
            p.iter()
                .map(|v| [0, 1, 2].map(|i| snap(v[i], cell[i])))
                .collect()
        })
        .collect();
    let mut mesh = Mesh::weld(surfaces.iter().copied(), &snapped)?;
    let mut seen = HashSet::new();
    (mesh.triangles, mesh.parts) = mesh
        .triangles
        .iter()
        .zip(&mesh.parts)
        .filter(|&(t, _)| seen.insert(rotated(*t)))
        .map(|(&t, &p)| (t, p))
        .unzip();
    Some(mesh)
}

/// The triangle turned to start at its least vertex, alike for the triangles that wind alike
/// through the same vertices.
fn rotated([a, b, c]: [u32; 3]) -> [u32; 3] {
    if b < a && b < c {
        [b, c, a]
    } else if c < a && c < b {
        [c, a, b]
    } else {
        [a, b, c]
    }
}

/// The block of `surfaces` at `points` on a grid of `pixel` units per pixel: the outline of
/// their ground or, without one, their bounding box, its corners clustered, raised from 0 to
/// their top. Its walls and roof face out, and it has no bottom.
/// `None` if the longer side of their bounding box is under `min_pixels`, or if its vertices are
/// more than a `u32` indexes.
fn block(
    surfaces: &[&Surface],
    points: &[Vec<[i32; 3]>],
    pixel: f64,
    min_pixels: f64,
) -> Option<Mesh> {
    let (min, max) =
        points
            .iter()
            .flatten()
            .fold(([i32::MAX; 3], [i32::MIN; 3]), |(min, max), p| {
                (
                    [0, 1, 2].map(|i| min[i].min(p[i])),
                    [0, 1, 2].map(|i| max[i].max(p[i])),
                )
            });
    let side = |i: usize| f64::from(max[i]) - f64::from(min[i]);
    if side(0).max(side(1)) < min_pixels * pixel {
        return None;
    }
    let cell = BLOCK_CELL_PIXELS * pixel;
    // Extend foundations below `z = 0` when downhill ground vertices lie below the centre datum;
    // otherwise start the block at `z = 0`.
    let base = min[2].min(0);
    if max[2] <= base {
        return None;
    }
    let (grounds, flat): (Vec<&Surface>, Vec<Vec<[i32; 3]>>) = surfaces
        .iter()
        .zip(points)
        .filter(|(s, _)| s.part == Part::Ground)
        .map(|(&s, p)| {
            let flat = p
                .iter()
                .map(|&[x, y, _]| [snap(x, cell), snap(y, cell), base]);
            (s, flat.collect())
        })
        .unzip();
    let ground = Mesh::weld(grounds, &flat)?;
    let mut xyz = ground.xyz;
    let mut triangles = facing_down(&xyz, &ground.triangles);
    if triangles.is_empty() {
        xyz = vec![
            [min[0], min[1], base],
            [max[0], min[1], base],
            [max[0], max[1], base],
            [min[0], max[1], base],
        ];
        triangles = facing_down(&xyz, &[[0, 1, 2], [0, 2, 3]]);
    }
    // Grounds snapped onto each other would count their edges twice, leaving out their walls.
    let mut seen = HashSet::new();
    triangles.retain(|&t| seen.insert(rotated(t)));
    let edges = |&[a, b, c]: &[u32; 3]| [[a, b], [b, c], [c, a]];
    let mut uses: HashMap<[u32; 2], u32> = HashMap::new();
    for [a, b] in triangles.iter().flat_map(edges) {
        *uses.entry([a.min(b), a.max(b)]).or_default() += 1;
    }
    // The roof's vertices follow the ground's, so twice their count must fit.
    let n = u32::try_from(xyz.len())
        .ok()
        .filter(|&n| n <= u32::MAX / 2)?;
    let surface = |part, triangles| Surface {
        part,
        lods: Lods::Both,
        positions: Vec::new(),
        triangles,
    };
    let walls = triangles
        .iter()
        .flat_map(edges)
        .filter(|&[a, b]| uses.get(&[a.min(b), a.max(b)]) == Some(&1))
        .flat_map(|[a, b]| [[b, a, a + n], [b, a + n, b + n]])
        .collect();
    let roof = triangles.iter().map(|&[a, b, c]| [a + n, c + n, b + n]);
    let mut raised = xyz.clone();
    raised.extend(xyz.iter().map(|&[x, y, _]| [x, y, max[2]]));
    Mesh::weld(
        &[
            surface(Part::Wall, walls),
            surface(Part::Roof, roof.collect()),
        ],
        &[raised.clone(), raised],
    )
}

/// The `triangles` between flat `xyz` that have an area, each turned to face down.
fn facing_down(xyz: &[[i32; 3]], triangles: &[[u32; 3]]) -> Vec<[u32; 3]> {
    // The grid's y points south, so a triangle facing down turns from +x towards +y.
    triangles
        .iter()
        .filter_map(|&[a, b, c]| {
            let [pa, pb, pc] = [a, b, c].map(|i| xyz[i as usize].map(i128::from));
            match ((pb[0] - pa[0]) * (pc[1] - pa[1]) - (pb[1] - pa[1]) * (pc[0] - pa[0])).signum() {
                1 => Some([a, b, c]),
                -1 => Some([a, c, b]),
                _ => None,
            }
        })
        .collect()
}

/// The multiple of `cell` nearest `v`, which is `v` for a cell under 1.
#[expect(
    clippy::cast_possible_truncation,
    reason = "the multiple is within a cell of an i32, and saturates past it"
)]
fn snap(v: i32, cell: f64) -> i32 {
    ((f64::from(v) / cell).round_ties_even() * cell).round() as i32
}

/// The layer of a tile's buildings, each with its feature id, the footprints merged into blocks.
/// A block takes the id of the first building it holds that no other feature has, and counts
/// them in a `buildings` property. `None` if no building leaves a triangle.
fn layer(
    name: &str,
    tile: Tile,
    z_step: ZStep,
    placed: Vec<(u64, &Building, Placed)>,
) -> AnyResult<Option<TileLayer>> {
    let mut meshes = Vec::new();
    let (mut ids, mut footprints) = (Vec::new(), Vec::new());
    for (id, building, placed) in placed {
        match placed {
            Placed::Mesh(mesh) => meshes.push((Some(id), building.gml_id.as_deref(), None, mesh)),
            Placed::Footprint(footprint) => {
                ids.push(id);
                footprints.push(footprint);
            }
        }
    }
    let cell = BLOCK_CELL_PIXELS * f64::from(tile.extent()) / TILE_PIXELS;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a pixel of a tile of at most a few hundred km in z units of at least a millimetre"
    )]
    let min_wall = (WALL_MIN_PIXELS * tile.width() / TILE_PIXELS / z_step.metres()).round() as i32;
    let mut taken: HashSet<u64> = meshes.iter().filter_map(|m| m.0).collect();
    for block in merge(&footprints, cell, z_step, min_wall) {
        let id = block
            .members
            .iter()
            .map(|&m| ids[m])
            .find(|&id| taken.insert(id));
        let count = u32::try_from(block.members.len())
            .context("a block holds more buildings than a u32 counts")?;
        meshes.push((id, None, Some(count), block.mesh));
    }
    meshes.retain(|m| !m.3.triangles.is_empty());
    if meshes.is_empty() {
        return Ok(None);
    }

    let mut builder = TileLayer::builder(name, tile.extent())?;
    builder.set_z_axis(ZAxis {
        step: z_step,
        mode: HeightMode::RelativeToGround,
    })?;
    let gml_id = builder.add_property("gml_id", PropKind::Str)?;
    let buildings = meshes
        .iter()
        .any(|m| m.2.is_some())
        .then(|| builder.add_property("buildings", PropKind::U32))
        .transpose()?;
    let part = builder.add_scoped_m_value("part", PropKind::Str, MValueScope::Triangle)?;
    for (id, name, count, mesh) in meshes {
        let points = mesh.xyz.iter().map(|&[x, y, _]| Point::new(x, y)).collect();
        let mut feature = builder.feature(Geometry::MultiPoint(MultiPoint(points)));
        feature.id(id);
        feature.z(mesh.xyz.iter().map(|v| v[2]).collect())?;
        feature.triangles(mesh.triangles)?;
        feature.property(gml_id, PropValue::Str(name.map(Into::into)))?;
        if let Some(buildings) = buildings {
            feature.property(buildings, PropValue::U32(count))?;
        }
        let parts = mesh.parts.iter().map(|p| p.as_str().into()).collect();
        feature.m_value(part, MValue::Str(Some(parts)))?;
        feature.finish()?;
    }
    Ok(Some(builder.finish()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tile_grid_unit_is_between_one_and_two_centimetres() {
        let [lon, lat] = [2.331_716_682_733_388_f64, 48.872_017_348_646_814].map(f64::to_radians);
        let extents = [15, 16, 17].map(|zoom| Tile::of(unit(lon, lat), zoom).extent());
        assert_eq!(extents, [65536, 32768, 16384]);
    }

    fn surface(part: Part, triangles: Vec<[u32; 3]>) -> Surface {
        Surface {
            part,
            lods: Lods::Both,
            positions: Vec::new(),
            triangles,
        }
    }

    fn count(mesh: &Mesh, part: Part) -> usize {
        mesh.parts.iter().filter(|&&p| p == part).count()
    }

    #[test]
    fn surfaces_share_their_welded_vertices_and_each_triangle_keeps_its_part() {
        let surfaces = [
            surface(Part::Wall, vec![[0, 1, 2]]),
            surface(Part::Roof, vec![[0, 2, 1], [0, 0, 1], [3, 3, 3]]),
        ];
        let points = [
            vec![[0, 0, 0], [5, 0, 0], [0, 5, 0]],
            vec![[5, 0, 0], [5, 5, 0], [0, 5, 0], [9, 9, 9]],
        ];
        let mesh = Mesh::weld(&surfaces, &points).unwrap();
        assert_eq!(mesh.xyz, [[0, 0, 0], [5, 0, 0], [0, 5, 0], [5, 5, 0]]);
        assert_eq!(mesh.triangles, [[0, 1, 2], [1, 2, 3]]);
        assert_eq!(mesh.parts, [Part::Wall, Part::Roof]);
    }

    #[test]
    fn clustering_collapses_edges_under_a_cell_and_keeps_one_of_triangles_winding_alike() {
        let wall = surface(Part::Wall, vec![[0, 1, 2], [1, 3, 2], [1, 2, 0], [2, 1, 0]]);
        let points = [vec![[0, 0, 0], [100, 0, 0], [0, 0, 100], [104, 0, 3]]];
        let mesh = cluster(&[&wall], &points, [10.0, 10.0, 10.0]).unwrap();
        assert_eq!(mesh.xyz, [[0, 0, 0], [100, 0, 0], [0, 0, 100]]);
        assert_eq!(mesh.triangles, [[0, 1, 2], [2, 1, 0]]);
    }

    #[test]
    fn grounds_on_top_of_each_other_make_one_block_with_its_walls() {
        let triangles = vec![[0, 1, 4], [1, 2, 4], [2, 3, 4], [3, 0, 4]];
        let reversed = triangles.iter().map(|&[a, b, c]| [a, c, b]).collect();
        let (ground, again) = (
            surface(Part::Ground, triangles),
            surface(Part::Ground, reversed),
        );
        let square = vec![
            [0, 0, 7],
            [100, 0, 7],
            [100, 100, 7],
            [0, 100, 7],
            [50, 50, 7],
        ];
        let points = [square.clone(), square];
        let mesh = block(&[&ground, &again], &points, 1.0, BLOCK_MIN_PIXELS).unwrap();
        assert_eq!([count(&mesh, Part::Wall), count(&mesh, Part::Roof)], [8, 4]);
    }

    #[test]
    fn a_block_raises_the_outline_of_its_ground_closed_but_for_its_bottom_and_facing_out() {
        // A square facing down in four triangles around its centre, under a roof 7 units up.
        let ground = surface(
            Part::Ground,
            vec![[0, 1, 4], [1, 2, 4], [2, 3, 4], [3, 0, 4]],
        );
        let roof = surface(Part::Roof, vec![[0, 1, 2]]);
        let points = [
            vec![
                [0, 0, 0],
                [100, 0, 0],
                [100, 100, 0],
                [0, 100, 0],
                [50, 50, 0],
            ],
            vec![[0, 0, 7], [100, 0, 7], [0, 100, 7]],
        ];
        let mesh = block(&[&ground, &roof], &points, 1.0, BLOCK_MIN_PIXELS).unwrap();
        assert_eq!(mesh.xyz.len(), 9);
        assert_eq!([count(&mesh, Part::Wall), count(&mesh, Part::Roof)], [8, 4]);

        let edges: Vec<[u32; 2]> = mesh
            .triangles
            .iter()
            .flat_map(|&[a, b, c]| [[a, b], [b, c], [c, a]])
            .collect();
        let uses = |edge: [u32; 2]| edges.iter().filter(|&&e| e == edge).count();
        for &[a, b] in &edges {
            let bottom = mesh.xyz[a as usize][2] == 0 && mesh.xyz[b as usize][2] == 0;
            assert_eq!(uses([b, a]), usize::from(!bottom), "{a} {b}");
            assert_eq!(uses([a, b]), 1, "{a} {b}");
        }

        // In (x, -y, z), each triangle winds counter-clockwise seen from outside.
        let xyz: Vec<[i64; 3]> = mesh
            .xyz
            .iter()
            .map(|&[x, y, z]| [x, -y, z].map(i64::from))
            .collect();
        let len = i64::try_from(xyz.len()).unwrap();
        let centre = [0, 1, 2].map(|i| xyz.iter().map(|p| p[i]).sum::<i64>() / len);
        for triangle in &mesh.triangles {
            let [first, second, third] = triangle.map(|i| xyz[i as usize]);
            let [along, across] = [second, third].map(|p| [0, 1, 2].map(|i| p[i] - first[i]));
            let normal = [
                along[1] * across[2] - along[2] * across[1],
                along[2] * across[0] - along[0] * across[2],
                along[0] * across[1] - along[1] * across[0],
            ];
            let outwards: i64 = (0..3).map(|i| normal[i] * (first[i] - centre[i])).sum();
            assert!(outwards > 0, "{triangle:?} faces inwards");
        }
    }

    #[test]
    fn a_block_without_ground_area_is_its_bounding_box_and_none_under_two_pixels() {
        let roof = surface(Part::Roof, vec![[0, 1, 2]]);
        let line = surface(Part::Ground, vec![[0, 1, 2]]);
        let points = [
            vec![[0, 0, 5], [30, 0, 5], [0, 20, 9]],
            vec![[0, 0, 5], [30, 0, 5], [15, 0, 5]],
        ];
        let mesh = block(&[&roof, &line], &points, 10.0, BLOCK_MIN_PIXELS).unwrap();
        let mut xyz = mesh.xyz.clone();
        xyz.sort_unstable();
        assert_eq!(
            xyz,
            [
                [0, 0, 0],
                [0, 0, 9],
                [0, 20, 0],
                [0, 20, 9],
                [30, 0, 0],
                [30, 0, 9],
                [30, 20, 0],
                [30, 20, 9]
            ]
        );
        assert_eq!([count(&mesh, Part::Wall), count(&mesh, Part::Roof)], [8, 2]);
        assert!(block(&[&roof, &line], &points, 16.0, BLOCK_MIN_PIXELS).is_none());
    }

    #[test]
    fn only_a_file_whose_coordinates_fit_the_common_crs_is_mislabelled() {
        let file = |epsg, x, y| {
            Some(File {
                epsg,
                first: [x, y, 0.0],
                buildings: Vec::new(),
            })
        };
        let hamburg = [566_100.0, 5_934_100.0];
        let files = [
            file(25832, hamburg[0], hamburg[1]),
            file(25832, 567_000.0, 5_935_000.0),
            None,
            file(25833, hamburg[0], hamburg[1]),
            // Berlin, in zone 33 indeed
            file(25833, 390_000.0, 5_820_000.0),
        ];
        assert_eq!(mislabelled(&files).unwrap(), [(3, 25833, 25832)]);
        assert_eq!(
            mislabelled(&files[3..]).unwrap(),
            [] as [(usize, u16, u16); 0]
        );
    }
}
