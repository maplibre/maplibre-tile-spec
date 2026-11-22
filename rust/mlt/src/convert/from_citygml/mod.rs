//! `mlt convert <citygml> <dir>`: tile the buildings of `CityGML` files into a `z/x/y.mlt` tree of v2 `Meshes` layers.

mod crs;
mod merge;
mod parser;
mod simplify;
mod triangulate;

use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::f64::consts::PI;
use std::fs::File;
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
use self::simplify::simplify;
use super::from_geojson::{mercator_unit, tile_index};
use super::{ConvertArgs, Reencoder, tiled};

/// The z grid without `--z-step`: 1 cm.
const DEFAULT_Z_STEP: i8 = -2;
/// The finest grid unit the extent of a tile aims for, in metres.
const UNIT_METRES: f64 = 0.01;
/// The largest extent a v2 layer header can name.
const MAX_EXTENT: u32 = 1 << 21;
/// The up component of a unit normal above which a surface without semantics is a roof.
const ROOF_MIN_UP: f64 = 0.34;
/// How far, in degrees, a file must lie from the other files in the CRS it names to be read in the CRS most files name.
const MISLABELLED_MIN_DEGREES: f64 = 1.0;
/// The pixels across a tile that the detail of zooms below the highest is measured in.
const TILE_PIXELS: f64 = 512.0;
/// The cell, in pixels, that merged blocks rasterise their footprints in.
const BLOCK_CELL_PIXELS: f64 = 1.0;
/// The height in metres from which a building keeps its 3D mesh two zooms below the highest.
const COARSE_3D_MIN_METRES: f64 = 14.0;
/// The bounding-box side in pixels from which a building keeps its 3D mesh two zooms below the highest.
const COARSE_3D_MIN_PIXELS: f64 = 6.0;
/// The height in metres from which a building is never merged and keeps its 3D mesh.
const LANDMARK_MIN_METRES: f64 = 40.0;
/// The height, in pixels, under which merged blocks have no wall between them or to the ground.
const WALL_MIN_PIXELS: f64 = 1.0;
/// The height above `z = 0` up to which downward ground triangles are dropped below the highest zoom.
const HIDDEN_GROUND_CAP_METRES: f64 = 0.5;
/// How far above the lowest ground surface a ground vertex may lie and still count as the building's ground.
const MAX_GROUND_SLOPE_METRES: f64 = 15.0;

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
    terrain: Vec<[f64; 3]>,
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
                let Some(layer) = layer(&name, tile, z_step, placed)? else {
                    return Ok(0);
                };
                if tiled::write_tile(output, reencoder, tile.zoom, tile.x, tile.y, layer)? {
                    Ok(1)
                } else {
                    eprintln!(
                        "warning: skipping tile {}/{}/{}: exceeds default decoder memory budget",
                        tile.zoom, tile.x, tile.y
                    );
                    Ok(0)
                }
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

/// The `CityGML` file `input`, or the `.gml` and `.xml` files under the directory `input`, sorted.
fn gml_files(input: &Path) -> AnyResult<Vec<PathBuf>> {
    if input.is_file() {
        return Ok(vec![input.to_owned()]);
    }
    files_under(input)
        .filter(|file| !matches!(file, Ok(f) if !is_gml(f)))
        .collect()
}

/// The buildings of the `CityGML` file at `path`, triangulated and reprojected from `epsg` or, without it, from the file's `srsName`.
/// `None` if the file holds no position.
fn read(path: &Path, epsg: Option<u16>) -> AnyResult<Option<SourceFile>> {
    let reader = BufReader::new(File::open(path)?);
    let document = parser::parse_reader(reader)?;
    if document.city_objects.is_empty() {
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
        .city_objects
        .iter()
        .flat_map(|b| &b.polygons)
        .find_map(|p| p.points.first().copied())
    else {
        return Ok(None);
    };
    let reprojector = Reprojector::new(epsg)?;
    let mut triangulator = triangulate::Triangulator::default();
    let mut buildings = Vec::with_capacity(document.city_objects.len());
    for city_object in document.city_objects {
        let mut surfaces = Vec::with_capacity(city_object.polygons.len());
        for polygon in city_object.polygons {
            let mut triangles = Vec::new();
            let Some(normal) = triangulator.polygon(
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
            let mut terrain = city_object.terrain_points;
            if !terrain.is_empty() {
                reprojector.to_geodetic(&mut terrain)?;
            }
            buildings.push(Building {
                gml_id: city_object.gml_id,
                surfaces,
                terrain,
            });
        }
    }
    Ok(Some(SourceFile {
        epsg,
        first,
        buildings,
    }))
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

    /// `[x, y]` on this tile's grid of `extent` of the Web Mercator position `[x, y]`, rounded half to even.
    /// `None` if it does not fit an `i32`.
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

/// A building on a tile's grid: its vertices, the triangles between them that have three distinct corners, and the part of each triangle.
#[derive(Debug, PartialEq)]
struct Mesh {
    xyz: Vec<[i32; 3]>,
    triangles: Vec<[u32; 3]>,
    parts: Vec<Part>,
}

impl Mesh {
    /// Welds `surfaces`, `points` holding each one's vertices on the grid, so that surfaces share the vertices they meet at.
    /// Vertices keep their order, and those no triangle names are dropped.
    /// `None` if they are more than a `u32` indexes.
    fn weld<'a>(
        surfaces: impl IntoIterator<Item = &'a Surface>,
        points: &[Vec<[i32; 3]>],
    ) -> Option<Self> {
        let mut ids: HashMap<[i32; 3], u32> = HashMap::new();
        let mut welded = Vec::new();
        let mut triangles = Vec::new();
        let mut parts = Vec::new();
        let mut seen = HashSet::new();
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
                if a != b && b != c && c != a && seen.insert((rotated([a, b, c]), surface.part)) {
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
        let mut mesh = Self {
            xyz,
            triangles,
            parts,
        };
        mesh.orient_outward();
        Some(mesh)
    }

    /// Turns each closed shell whose triangles face inward outward, so that its triangles wind counter-clockwise in `(x, -y, z)` seen from outside.
    /// Open shells keep their winding.
    fn orient_outward(&mut self) {
        let count = self.triangles.len();
        if count < 4 {
            return;
        }
        let mut parent: Vec<usize> = (0..count).collect();
        let find = |parent: &mut [usize], mut x: usize| {
            while parent[x] != x {
                parent[x] = parent[parent[x]];
                x = parent[x];
            }
            x
        };
        let mut by_edge: Vec<(u64, usize)> = Vec::with_capacity(count * 3);
        for (i, &[a, b, c]) in self.triangles.iter().enumerate() {
            for (u, v) in [(a, b), (b, c), (c, a)] {
                let (lo, hi) = (u.min(v), u.max(v));
                by_edge.push(((u64::from(lo) << 32) | u64::from(hi), i));
            }
        }
        by_edge.sort_unstable_by_key(|&(edge, _)| edge);
        for pair in by_edge.windows(2) {
            if pair[0].0 == pair[1].0 {
                let (ra, rb) = (find(&mut parent, pair[0].1), find(&mut parent, pair[1].1));
                if ra != rb {
                    parent[rb] = ra;
                }
            }
        }
        let mut components: HashMap<usize, Vec<usize>> = HashMap::new();
        for i in 0..count {
            components.entry(find(&mut parent, i)).or_default().push(i);
        }
        let mut shell = Vec::new();
        for indices in components.into_values() {
            if indices.len() < 4 {
                continue;
            }
            shell.clear();
            shell.extend(indices.iter().map(|&i| self.triangles[i]));
            if is_closed_shell(&shell) && signed_volume_x6(&self.xyz, &shell) < 0 {
                for i in indices {
                    self.triangles[i].swap(1, 2);
                }
            }
        }
    }
}

/// Whether every directed edge `(a, b)` of `triangles` is paired with `(b, a)`.
fn is_closed_shell(triangles: &[[u32; 3]]) -> bool {
    let mut forward = Vec::with_capacity(triangles.len() * 3);
    let mut reverse = Vec::with_capacity(triangles.len() * 3);
    for &[a, b, c] in triangles {
        for (u, v) in [(a, b), (b, c), (c, a)] {
            forward.push((u64::from(u) << 32) | u64::from(v));
            reverse.push((u64::from(v) << 32) | u64::from(u));
        }
    }
    forward.sort_unstable();
    reverse.sort_unstable();
    forward == reverse
}

/// Six times the signed volume enclosed by `triangles` in `(x, -y, z)`, positive when they wind counter-clockwise seen from outside.
fn signed_volume_x6(xyz: &[[i32; 3]], triangles: &[[u32; 3]]) -> i128 {
    let Some(&[x0, y0, z0]) = xyz.first() else {
        return 0;
    };
    let at = |v: u32| {
        let [x, y, z] = xyz[v as usize];
        [
            i128::from(x) - i128::from(x0),
            -(i128::from(y) - i128::from(y0)),
            i128::from(z) - i128::from(z0),
        ]
    };
    triangles
        .iter()
        .map(|&[a, b, c]| {
            let ([ax, ay, az], [bx, by, bz], [cx, cy, cz]) = (at(a), at(b), at(c));
            ax * (by * cz - bz * cy) - ay * (bx * cz - bz * cx) + az * (bx * cy - by * cx)
        })
        .sum()
}

/// A building on a tile: its mesh, or its footprint, to be merged with those of the tile's others.
enum Placed {
    Mesh(Mesh),
    Footprint(Footprint),
}

/// How many zooms below the highest a zoom lies.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tier {
    /// The highest zoom, of the highest LOD.
    Highest,
    /// One zoom below, of LOD2.
    Lod2,
    /// Two zooms below, where small buildings become blocks.
    Coarse,
    /// Three or more zooms below, where buildings under [`LANDMARK_MIN_METRES`] are merged.
    Merged,
}

/// What a building keeps at a tier.
struct Detail {
    lods: Lods,
    /// The bounding-box side, in pixels, under which a building is left out.
    min_pixels: f64,
    /// The simplification error in metres, added to that of `error_pixels`.
    error_metres: f64,
    error_pixels: f64,
    /// Whether simplifying removes small disconnected pieces.
    prune: bool,
    /// Whether downward ground triangles at `z = 0` are kept.
    ground_caps: bool,
}

impl Tier {
    fn of(zoom: u8, max_zoom: u8) -> Self {
        match max_zoom - zoom {
            0 => Self::Highest,
            1 => Self::Lod2,
            2 => Self::Coarse,
            _ => Self::Merged,
        }
    }

    fn detail(self) -> Detail {
        match self {
            Self::Highest => Detail {
                lods: Lods::Highest,
                min_pixels: 0.0,
                error_metres: 0.05,
                error_pixels: 0.0,
                prune: true,
                ground_caps: true,
            },
            Self::Lod2 => Detail {
                lods: Lods::Lod2,
                min_pixels: 1.5,
                error_metres: 0.0,
                error_pixels: 0.45,
                prune: false,
                ground_caps: false,
            },
            Self::Coarse | Self::Merged => Detail {
                lods: Lods::Lod2,
                min_pixels: 2.5,
                error_metres: 0.0,
                error_pixels: 0.45,
                prune: false,
                ground_caps: false,
            },
        }
    }
}

/// `building` at `zoom` on the tile that holds the centre of its bounding box, simplified to the pixels of that zoom.
/// `None` if it is too small for the zoom or leaves no triangle.
fn place(building: &Building, zoom: u8, max_zoom: u8, z_step: ZStep) -> Option<(Tile, Placed)> {
    let tier = Tier::of(zoom, max_zoom);
    let detail = tier.detail();
    let lod = LodSurfaces::new(
        building
            .surfaces
            .iter()
            .filter(|s| s.lods.includes(detail.lods)),
        &building.terrain,
    )?;
    let tile = Tile::of(lod.centre, zoom);
    let extent = tile.extent();
    let pixel = f64::from(extent) / TILE_PIXELS;
    let landmark = lod.height >= LANDMARK_MIN_METRES;
    if tier == Tier::Merged && !landmark {
        let points = lod.points_with_datum(tile, extent, z_step, 0.0)?;
        let footprint = Footprint::new(&lod.surfaces, &points)?;
        return Some((tile, Placed::Footprint(footprint)));
    }
    let points = lod.points(tile, extent, z_step)?;
    let (min_x, max_x, min_y, max_y) = points.iter().flatten().fold(
        (i32::MAX, i32::MIN, i32::MAX, i32::MIN),
        |(x0, x1, y0, y1), &[x, y, _]| (x0.min(x), x1.max(x), y0.min(y), y1.max(y)),
    );
    let span = (f64::from(max_x) - f64::from(min_x)).max(f64::from(max_y) - f64::from(min_y));
    if !landmark && span < detail.min_pixels * pixel {
        return None;
    }
    let prominent =
        landmark || lod.height >= COARSE_3D_MIN_METRES || span >= COARSE_3D_MIN_PIXELS * pixel;
    let mut mesh = if tier == Tier::Coarse && !prominent {
        block(&lod.surfaces, &points, z_step)?
    } else {
        Mesh::weld(lod.surfaces.iter().copied(), &points)?
    };
    if !detail.ground_caps {
        drop_ground_caps(&mut mesh, z_step);
    }
    let metres = tile.width() / f64::from(extent);
    let error = detail.error_metres + detail.error_pixels * tile.width() / TILE_PIXELS;
    let mut mesh = simplify(
        &mesh,
        [metres, metres, z_step.metres()],
        error,
        detail.prune,
    )?;
    mesh.orient_outward();
    (!mesh.triangles.is_empty()).then_some((tile, Placed::Mesh(mesh)))
}

/// Drops the downward ground triangles of `mesh` no higher than [`HIDDEN_GROUND_CAP_METRES`] above `z = 0`.
fn drop_ground_caps(mesh: &mut Mesh, z_step: ZStep) {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "0.5 m in z_step units of at least 1 mm fits in i32"
    )]
    let cap = (HIDDEN_GROUND_CAP_METRES / z_step.metres()).round() as i32;
    (mesh.triangles, mesh.parts) = mesh
        .triangles
        .iter()
        .zip(&mesh.parts)
        .filter(|&(&t, &part)| {
            part != Part::Ground || t.iter().any(|&v| mesh.xyz[v as usize][2] > cap)
        })
        .map(|(&t, &p)| (t, p))
        .unzip();
}

/// The surfaces of one LOD of a building, the centre of their Web Mercator bounding box in units of [`unit()`], their ground at that centre, and their height above it.
struct LodSurfaces<'a> {
    surfaces: Vec<&'a Surface>,
    units: Vec<Vec<[f64; 2]>>,
    centre: [f64; 2],
    ground: f64,
    height: f64,
}

impl<'a> LodSurfaces<'a> {
    /// The ground is the terrain intersection nearest the centre, or else the ground surface vertex nearest it, or else the lowest vertex.
    /// Terrain and ground vertices over [`MAX_GROUND_SLOPE_METRES`] above the lowest of them do not count.
    /// `None` without a surface.
    fn new(surfaces: impl Iterator<Item = &'a Surface>, terrain: &[[f64; 3]]) -> Option<Self> {
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
        let nearest_z = |pts: &mut dyn Iterator<Item = ([f64; 2], f64)>| -> Option<f64> {
            pts.min_by(|(ua, _), (ub, _)| {
                let da = (ua[0] - centre[0]).hypot(ua[1] - centre[1]);
                let db = (ub[0] - centre[0]).hypot(ub[1] - centre[1]);
                da.total_cmp(&db)
            })
            .map(|(_, z)| z)
        };
        let terrain_ground = (!terrain.is_empty()).then(|| {
            let min_t = terrain.iter().map(|p| p[2]).fold(f64::INFINITY, f64::min);
            nearest_z(
                &mut terrain
                    .iter()
                    .filter(|p| p[2] <= min_t + MAX_GROUND_SLOPE_METRES)
                    .map(|p| (unit(p[0], p[1]), p[2])),
            )
        });
        let surface_ground = || {
            let min_g = surfaces
                .iter()
                .filter(|s| s.part == Part::Ground)
                .flat_map(|s| &s.positions)
                .map(|p| p[2])
                .fold(f64::INFINITY, f64::min);
            min_g.is_finite().then(|| {
                nearest_z(
                    &mut surfaces
                        .iter()
                        .zip(&units)
                        .filter(|(s, _)| s.part == Part::Ground)
                        .flat_map(|(s, u)| s.positions.iter().zip(u))
                        .filter(|(p, _)| p[2] <= min_g + MAX_GROUND_SLOPE_METRES)
                        .map(|(p, &u)| (u, p[2])),
                )
            })
        };
        let (min_z, max_z) = surfaces
            .iter()
            .flat_map(|s| &s.positions)
            .map(|p| p[2])
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), z| {
                (lo.min(z), hi.max(z))
            });
        let ground = terrain_ground
            .flatten()
            .or_else(|| surface_ground().flatten())
            .unwrap_or(min_z);
        Some(Self {
            surfaces,
            units,
            centre,
            ground,
            height: (max_z - ground).max(0.0),
        })
    }

    /// The vertices of each surface on `tile`'s grid of `extent`, z above `self.ground`, or `None` if one does not fit.
    fn points(&self, tile: Tile, extent: u32, z_step: ZStep) -> Option<Vec<Vec<[i32; 3]>>> {
        self.points_with_datum(tile, extent, z_step, self.ground)
    }

    /// The vertices of each surface on `tile`'s grid of `extent`, z above `datum`, or `None` if one does not fit.
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

/// The triangle turned to start at its least vertex, alike for the triangles that wind alike through the same vertices.
fn rotated([a, b, c]: [u32; 3]) -> [u32; 3] {
    if b < a && b < c {
        [b, c, a]
    } else if c < a && c < b {
        [c, a, b]
    } else {
        [a, b, c]
    }
}

/// The block of `surfaces` at `points`: the outline of their ground or, without one, their bounding box, raised from 0 to their top.
/// Its walls and roof face out, and it has no bottom.
/// `None` if it has no height, or if its vertices are more than a `u32` indexes.
fn block(surfaces: &[&Surface], points: &[Vec<[i32; 3]>], z_step: ZStep) -> Option<Mesh> {
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
    // Foundations below the ground at the centre reach down to it.
    let base = min[2].min(0);
    if max[2] <= base {
        return None;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "15 m in z_step units of at least 1 mm fits in i32"
    )]
    let slope_cap = (MAX_GROUND_SLOPE_METRES / z_step.metres()).round() as i32;
    let min_ground_z = surfaces
        .iter()
        .zip(points)
        .filter(|(s, _)| s.part == Part::Ground)
        .flat_map(|(_, p)| p.iter().map(|v| v[2]))
        .min()
        .unwrap_or(0);
    let max_collar_z = min_ground_z.saturating_add(slope_cap);
    let (grounds, flat): (Vec<&Surface>, Vec<Vec<[i32; 3]>>) = surfaces
        .iter()
        .zip(points)
        .filter(|(s, p)| s.part == Part::Ground && p.iter().all(|v| v[2] <= max_collar_z))
        .map(|(&s, p)| (s, p.iter().map(|&[x, y, _]| [x, y, base]).collect()))
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
    // Grounds stacked onto each other would count their edges twice, leaving out their walls.
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

/// The 16-bit Hilbert curve index of `[x, y]` over `[-extent, 2 * extent)`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "clamped to 0..=65535"
)]
fn hilbert_key([x, y]: [i32; 2], extent: u32) -> u32 {
    let span = (i64::from(extent) * 3).max(1);
    let norm =
        |c: i32| (((i64::from(c) + i64::from(extent)).clamp(0, span - 1) * 65535) / span) as u32;
    let (mut x, mut y) = (norm(x), norm(y));
    let mut d = 0_u32;
    let mut s = 1_u32 << 15;
    while s > 0 {
        let rx = u32::from((x & s) > 0);
        let ry = u32::from((y & s) > 0);
        d += s * s * ((3 * rx) ^ ry);
        if ry == 0 {
            if rx == 1 {
                x = 65535 - x;
                y = 65535 - y;
            }
            std::mem::swap(&mut x, &mut y);
        }
        s >>= 1;
    }
    d
}

/// A feature of a tile's layer: one building, or a block of merged buildings.
enum TileMesh<'a> {
    Building {
        id: u64,
        gml_id: Option<&'a str>,
        mesh: Mesh,
    },
    Block {
        id: Option<u64>,
        buildings: u32,
        mesh: Mesh,
    },
}

impl TileMesh<'_> {
    fn mesh(&self) -> &Mesh {
        match self {
            Self::Building { mesh, .. } | Self::Block { mesh, .. } => mesh,
        }
    }
}

/// The layer of a tile's buildings, the footprints merged into blocks.
/// A block takes the id of the first building it holds that no other feature has, and counts them in a `buildings` property.
/// `None` if no building leaves a triangle.
fn layer(
    name: &str,
    tile: Tile,
    z_step: ZStep,
    placed: Vec<(u64, &Building, Placed)>,
) -> AnyResult<Option<TileLayer>> {
    let mut meshes = Vec::new();
    let mut taken = HashSet::new();
    let (mut ids, mut footprints) = (Vec::new(), Vec::new());
    for (id, building, placed) in placed {
        match placed {
            Placed::Mesh(mesh) => {
                taken.insert(id);
                meshes.push(TileMesh::Building {
                    id,
                    gml_id: building.gml_id.as_deref(),
                    mesh,
                });
            }
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
    for block in merge(&footprints, cell, z_step, min_wall) {
        let id = block
            .members
            .iter()
            .map(|&m| ids[m])
            .find(|&id| taken.insert(id));
        let buildings = u32::try_from(block.members.len())
            .context("a block holds more buildings than a u32 counts")?;
        meshes.push(TileMesh::Block {
            id,
            buildings,
            mesh: block.mesh,
        });
    }
    meshes.retain(|m| !m.mesh().triangles.is_empty());
    if meshes.is_empty() {
        return Ok(None);
    }
    // Blocks follow the buildings, so that the `buildings` column is present in one run.
    let extent = tile.extent();
    meshes.sort_by_key(|m| {
        let [x, y, _] = m.mesh().xyz.first().copied().unwrap_or_default();
        (
            matches!(m, TileMesh::Block { .. }),
            hilbert_key([x, y], extent),
        )
    });

    let mut builder = TileLayer::builder(name, extent)?;
    builder.set_meshes(ZAxis {
        step: z_step,
        mode: HeightMode::RelativeToGround,
    })?;
    let gml_id = builder.add_property("gml_id", PropKind::Str)?;
    let buildings = meshes
        .iter()
        .any(|m| matches!(m, TileMesh::Block { .. }))
        .then(|| builder.add_property("buildings", PropKind::U32))
        .transpose()?;
    let part = builder.add_scoped_m_value("part", PropKind::Str, MValueScope::Triangle)?;
    for tile_mesh in meshes {
        let (id, name, count, mesh) = match tile_mesh {
            TileMesh::Building { id, gml_id, mesh } => (Some(id), gml_id, None, mesh),
            TileMesh::Block {
                id,
                buildings,
                mesh,
            } => (id, None, Some(buildings), mesh),
        };
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
