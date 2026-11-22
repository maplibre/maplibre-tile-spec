//! Buildings placed on the tiles of each zoom, simplified to its pixels, merged into blocks at coarse zooms, and written as `Meshes` layers.

use std::collections::{BTreeMap, HashSet};
use std::f64::consts::PI;
use std::ops::RangeInclusive;
use std::path::Path;

use anyhow::{Context as _, Result as AnyResult};
use martin_tile_utils::EARTH_CIRCUMFERENCE;
use mlt_core::geo_types::{Geometry, MultiPoint, Point};
use mlt_core::{HeightMode, MValue, MValueScope, PropKind, PropValue, TileLayer, ZAxis, ZStep};
use rayon::prelude::*;

use super::merge::{Footprint, merge};
use super::mesh::{MAX_GROUND_SLOPE_METRES, Mesh, block, drop_ground_caps};
use super::model::{Building, Lods, Part, Surface};
use super::simplify::simplify;
use crate::convert::from_geojson::{mercator_unit, tile_index};
use crate::convert::{Reencoder, tiled};

/// The property holding a building's id, which a block of merged buildings lacks.
const GML_ID: &str = "gml_id";
/// The property counting the buildings a block holds.
const BUILDINGS: &str = "buildings";
/// The finest grid unit the extent of a tile aims for, in metres.
const UNIT_METRES: f64 = 0.01;
/// The largest extent a v2 layer header can name.
const MAX_EXTENT: u32 = 1 << 21;
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

/// What every tile's layer shares.
pub struct LayerSpec {
    pub name: String,
    pub z_step: ZStep,
}

/// Writes the `buildings` as the layer of `spec` into the tiles of `zooms` under `output`, returning how many tiles it wrote.
/// A tile that outgrows the default decoder budget is left out with a warning.
pub fn tile(
    buildings: &[Building],
    spec: &LayerSpec,
    zooms: RangeInclusive<u8>,
    output: &Path,
    reencoder: &Reencoder,
) -> AnyResult<usize> {
    let max_zoom = *zooms.end();
    // One zoom at a time, so that only its placed meshes are held at once.
    let mut written = 0;
    for zoom in zooms {
        let placed: Vec<Option<(Tile, Placed)>> = buildings
            .par_iter()
            .map(|building| place(building, zoom, max_zoom, spec.z_step))
            .collect();
        let mut tiles: BTreeMap<Tile, Vec<(u64, &Building, Placed)>> = BTreeMap::new();
        for ((id, building), placed) in (0..).zip(buildings).zip(placed) {
            if let Some((tile, placed)) = placed {
                tiles.entry(tile).or_default().push((id, building, placed));
            }
        }
        written += tiles
            .into_par_iter()
            .map(|(tile, placed)| -> AnyResult<usize> {
                let Some(layer) = layer(spec, tile, placed)? else {
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
    Ok(written)
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
        building: &'a Building,
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
    spec: &LayerSpec,
    tile: Tile,
    placed: Vec<(u64, &Building, Placed)>,
) -> AnyResult<Option<TileLayer>> {
    let z_step = spec.z_step;
    let mut meshes = Vec::new();
    let mut taken = HashSet::new();
    let (mut ids, mut footprints) = (Vec::new(), Vec::new());
    for (id, building, placed) in placed {
        match placed {
            Placed::Mesh(mesh) => {
                taken.insert(id);
                meshes.push(TileMesh::Building { id, building, mesh });
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

    let mut builder = TileLayer::builder(spec.name.clone(), extent)?;
    builder.set_meshes(ZAxis {
        step: z_step,
        mode: HeightMode::RelativeToGround,
    })?;
    let gml_id = builder.add_property(GML_ID, PropKind::Str)?;
    let buildings = meshes
        .iter()
        .any(|m| matches!(m, TileMesh::Block { .. }))
        .then(|| builder.add_property(BUILDINGS, PropKind::U32))
        .transpose()?;
    let part = builder.add_scoped_m_value("part", PropKind::Str, MValueScope::Triangle)?;
    for tile_mesh in meshes {
        let (id, building, count, mesh) = match tile_mesh {
            TileMesh::Building { id, building, mesh } => (Some(id), Some(building), None, mesh),
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
        let name = building.and_then(|b| b.id.as_deref());
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
