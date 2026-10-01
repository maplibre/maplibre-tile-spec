//! `mlt from-geojson <file>.geojson <dir>`: tile a WGS84 `GeoJSON` file into an
//! MLT `z/x/y.mlt` tree (XYZ scheme, `y = 0` at the north).
//!
//! `GeoJSON` has no layer or tiling concept, so the whole file becomes one named
//! layer, re-projected and clipped into every tile of every requested zoom.
//! The property schema and the vertical grid are decided once, globally, so all
//! tiles share one schema: an encoder layer holds one kind per column and one
//! [`ZStep`] for all its vertices.
//!
//! Altitudes become MLT z coordinates on a `--z-step` grid. Only the v2 wire
//! format holds them, so this command always writes v2 and exists only in an
//! `mlt` built with `unstable-v2`.

mod clip;
mod project;
mod properties;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context as _, Result as AnyResult, bail};
use clap::Args;
use geojson::{Feature, FeatureCollection, GeoJson};
use martin_tile_utils::MAX_ZOOM;
use mlt_core::ZStep;
use mlt_core::encoder::{EncoderConfig, WireVersion};
use mlt_core::geo_types::{
    Coord, Geometry, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
};
use mlt_core::{PropValue, TileLayer};
use rayon::prelude::*;

use self::clip::Rect;
use self::project::{Dims, Geom, Projector, Vertex};
use self::properties::Schema;
use crate::convert::ContainerFormat;

#[derive(Args)]
pub struct FromGeoJsonArgs {
    /// Input: a WGS84 `GeoJSON` file holding a `FeatureCollection`
    input: PathBuf,
    /// Output: a directory for the `z/x/y.mlt` tile tree
    output: PathBuf,
    /// Lowest zoom level to tile into
    #[clap(long, value_name = "ZOOM", default_value_t = 0)]
    min_zoom: u8,
    /// Highest zoom level to tile into
    #[clap(long, value_name = "ZOOM")]
    max_zoom: u8,
    /// MLT layer name (default: the input file stem)
    #[clap(long, value_name = "NAME")]
    layer: Option<String>,
    /// Power of ten of the z grid's step in metres (-3..=4) for positions with an altitude
    ///
    /// Required when the positions carry an altitude, and rejected when they do not.
    #[clap(long, value_name = "EXPONENT", allow_hyphen_values = true)]
    z_step: Option<i8>,
}

/// The number of grid units across a tile, the value every MVT/MLT consumer expects.
const EXTENT: u32 = 4096;

/// Lines and polygons are kept this far past each tile edge, in extent units, so
/// strokes and fills meet seamlessly at tile seams.
const BUFFER: u32 = 64;

/// What every tile's layer shares.
struct LayerSpec {
    name: String,
    schema: Schema,
    /// The vertical grid of the layer: a step when the input has altitudes, or none.
    z_step: Option<ZStep>,
}

/// A feature in the Web Mercator unit square, clipped to the tile being visited.
/// The geometry is shared with the parent tile when the clip left it untouched.
struct Source {
    id: Option<u64>,
    geom: Arc<Geom>,
    /// `[min_x, min_y, max_x, max_y]` of `geom`, so a child tile the feature
    /// misses or wholly contains it in is settled without clipping.
    bbox: [f64; 4],
    /// Non-null property values as `(column, value)`, shared by every tile the feature lands in.
    props: Arc<Vec<(usize, PropValue)>>,
}

/// One feature rounded onto a tile's grid.
struct Entry {
    id: Option<u64>,
    geometry: Geometry<i32>,
    /// The z of each stored vertex, in the order the layer counts them; unused on a flat layer.
    z: Vec<i32>,
    props: Arc<Vec<(usize, PropValue)>>,
}

pub fn from_geojson(args: &FromGeoJsonArgs) -> AnyResult<()> {
    let (input, output) = (args.input.as_path(), args.output.as_path());
    if args.min_zoom > args.max_zoom {
        bail!(
            "--min-zoom ({}) must be <= --max-zoom ({})",
            args.min_zoom,
            args.max_zoom
        );
    }
    if args.max_zoom > MAX_ZOOM {
        bail!("--max-zoom ({}) must be <= {MAX_ZOOM}", args.max_zoom);
    }
    if ContainerFormat::from_path(output) != ContainerFormat::Files {
        bail!(
            "from-geojson writes a directory of z/x/y.mlt tiles; archive output is not supported yet, got: {}",
            output.display()
        );
    }
    let name = match &args.layer {
        Some(name) => name.clone(),
        None => input
            .file_stem()
            .and_then(|s| s.to_str())
            .map(str::to_owned)
            .with_context(|| {
                format!(
                    "cannot derive a layer name from {}; pass --layer",
                    input.display()
                )
            })?,
    };

    let features = read_features(input)?;
    let schema = Schema::infer(&features)?;
    let z_step = args.z_step.map(ZStep::new).transpose()?;
    let mut projector = Projector::new(z_step);
    let mut sources = Vec::with_capacity(features.len());
    for (index, feature) in features.iter().enumerate() {
        let Some(geometry) = feature.geometry.as_ref() else {
            continue;
        };
        let geom = projector
            .project(&geometry.value)
            .with_context(|| format!("projecting feature {index}"))?;
        // An empty geometry has nothing to tile.
        let Some(bbox) = geom.bbox() else {
            continue;
        };
        sources.push(Source {
            id: properties::feature_id(feature)
                .with_context(|| format!("reading the id of feature {index}"))?,
            geom: Arc::new(geom),
            bbox,
            props: Arc::new(schema.values(feature)?),
        });
    }
    // The parsed tree is no longer needed; it would otherwise double peak memory while tiling.
    drop(features);
    check_dimensions(projector.dims, z_step)?;

    let layer = LayerSpec {
        name,
        schema,
        z_step,
    };
    let tiler = Tiler {
        output,
        layer: &layer,
        min_zoom: args.min_zoom,
        max_zoom: args.max_zoom,
    };
    let (tiles_written, features_written) = tiler.tile(0, 0, 0, &sources)?;

    if sources.is_empty() {
        eprintln!("No features with a geometry to tile in {}", input.display());
    } else if tiles_written == 0 {
        eprintln!(
            "{}: none of its {} feature(s) spans a grid unit at zooms {}..={}, so no tile was written; raise --max-zoom",
            input.display(),
            sources.len(),
            args.min_zoom,
            args.max_zoom,
        );
    } else {
        eprintln!(
            "{} -> {}: wrote {tiles_written} tile(s) holding {features_written} feature(s) at zooms {}..={}",
            input.display(),
            output.display(),
            args.min_zoom,
            args.max_zoom,
        );
    }
    Ok(())
}

/// Parse the file, which must hold a `FeatureCollection`, into its features.
fn read_features(input: &Path) -> AnyResult<Vec<Feature>> {
    let text = fs::read_to_string(input).with_context(|| format!("reading {}", input.display()))?;
    let parsed: GeoJson = text
        .parse()
        .with_context(|| format!("parsing GeoJSON from {}", input.display()))?;
    let collection = FeatureCollection::try_from(parsed)
        .with_context(|| format!("reading {}", input.display()))?;
    Ok(collection.features)
}

/// The file must be all flat or all 3D, and `--z-step` must match which.
fn check_dimensions(dims: Dims, z_step: Option<ZStep>) -> AnyResult<()> {
    match (dims.flat > 0, dims.with_z > 0, z_step) {
        (true, true, _) => bail!(
            "the GeoJSON mixes {} positions without an altitude and {} with one; \
             an MLT layer is either flat or 3D, so make them all [lon, lat] or all [lon, lat, alt]",
            dims.flat,
            dims.with_z
        ),
        (_, true, None) => bail!(
            "the GeoJSON positions carry an altitude, so --z-step is required \
             (the power of ten of the z grid's step in metres, -3..=4)"
        ),
        (_, false, Some(_)) => bail!(
            "--z-step needs [lon, lat, alt] positions, but the GeoJSON positions have no altitude"
        ),
        (_, true, Some(_)) | (_, false, None) => Ok(()),
    }
}

/// Which of the `n` tiles along one axis a unit coordinate falls in. The world's
/// far edge belongs to the last tile, so every point has exactly one tile.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the product is floored, clamped to 0.., and capped at n - 1 < 2^30"
)]
fn tile_index(unit: f64, n: u32) -> u32 {
    ((unit * f64::from(n)).floor().max(0.0) as u32).min(n - 1)
}

/// The buffered rectangle of tile `(zoom, col, row)` in unit coordinates.
fn tile_rect(zoom: u8, col: u32, row: u32) -> Rect {
    let n = f64::from(1_u32 << zoom);
    let size = 1.0 / n;
    let buffer = f64::from(BUFFER) / (f64::from(EXTENT) * n);
    Rect {
        min_x: f64::from(col) * size - buffer,
        min_y: f64::from(row) * size - buffer,
        max_x: f64::from(col + 1) * size + buffer,
        max_y: f64::from(row + 1) * size + buffer,
    }
}

/// Round a vertex of the unit square onto a tile's integer grid: `scale` is the
/// grid's size across the whole world at the tile's zoom and `origin` the tile's
/// corner on it. The z is already in grid units.
#[expect(
    clippy::cast_possible_truncation,
    reason = "clipped x/y lie within -buffer..=extent+buffer of the tile origin, and z was range-checked at projection; a cut point lies between its endpoints"
)]
fn round_vertex(v: &Vertex, scale: f64, origin: (f64, f64)) -> (Coord<i32>, i32) {
    let coord = Coord {
        x: (v.x * scale - origin.0).round() as i32,
        y: (v.y * scale - origin.1).round() as i32,
    };
    (coord, v.z.round() as i32)
}

/// Round a run of vertices, dropping any that lands on the previous one.
fn round_run(vertices: &[Vertex], scale: f64, origin: (f64, f64)) -> (Vec<Coord<i32>>, Vec<i32>) {
    let mut coords: Vec<Coord<i32>> = Vec::with_capacity(vertices.len());
    let mut z = Vec::with_capacity(vertices.len());
    for v in vertices {
        let (c, vz) = round_vertex(v, scale, origin);
        if coords.last() != Some(&c) {
            coords.push(c);
            z.push(vz);
        }
    }
    (coords, z)
}

/// Round a line, or `None` when fewer than two distinct vertices remain.
fn round_line(
    line: &[Vertex],
    scale: f64,
    origin: (f64, f64),
) -> Option<(LineString<i32>, Vec<i32>)> {
    let (coords, z) = round_run(line, scale, origin);
    (coords.len() >= 2).then(|| (LineString::new(coords), z))
}

/// Round an open ring and close it, or `None` when fewer than three distinct
/// vertices remain. The z values cover the stored vertices only.
fn round_ring(
    open: &[Vertex],
    scale: f64,
    origin: (f64, f64),
) -> Option<(LineString<i32>, Vec<i32>)> {
    let (mut coords, mut z) = round_run(open, scale, origin);
    if coords.len() > 1 && coords.first() == coords.last() {
        coords.pop();
        z.pop();
    }
    if coords.len() < 3 {
        return None;
    }
    coords.push(coords[0]);
    Some((LineString::new(coords), z))
}

/// Round a polygon's rings, or `None` when its exterior collapses. Holes that
/// collapse are dropped.
fn round_polygon(
    rings: &[Vec<Vertex>],
    scale: f64,
    origin: (f64, f64),
) -> Option<(Polygon<i32>, Vec<i32>)> {
    let mut rings = rings.iter();
    let (exterior, mut z) = round_ring(rings.next()?, scale, origin)?;
    let (interiors, holes_z) =
        gather(rings.map(|r| round_ring(r, scale, origin))).unwrap_or_default();
    z.extend(holes_z);
    Some((Polygon::new(exterior, interiors), z))
}

/// Collect the parts that survive rounding, with their z in order; `None` when none do.
fn gather<T>(parts: impl Iterator<Item = Option<(T, Vec<i32>)>>) -> Option<(Vec<T>, Vec<i32>)> {
    let mut out = Vec::new();
    let mut z = Vec::new();
    for (part, part_z) in parts.flatten() {
        out.push(part);
        z.extend(part_z);
    }
    (!out.is_empty()).then_some((out, z))
}

/// Round a geometry already clipped to tile `(zoom, col, row)` onto that tile's
/// integer grid, with the z of each stored vertex. `None` when nothing survives
/// rounding. Multi-part geometries keep their type; a line that was split by
/// clipping arrives here as a `MultiLineString` already.
fn to_tile(geom: &Geom, zoom: u8, col: u32, row: u32) -> Option<(Geometry<i32>, Vec<i32>)> {
    let extent = f64::from(EXTENT);
    let scale = f64::from(1_u32 << zoom) * extent;
    let origin = (f64::from(col) * extent, f64::from(row) * extent);
    Some(match geom {
        Geom::Point(v) => {
            let (c, z) = round_vertex(v, scale, origin);
            (Geometry::Point(Point(c)), vec![z])
        }
        Geom::MultiPoint(points) => {
            let (coords, z): (Vec<Point<i32>>, Vec<i32>) = points
                .iter()
                .map(|v| {
                    let (c, z) = round_vertex(v, scale, origin);
                    (Point(c), z)
                })
                .unzip();
            (Geometry::MultiPoint(MultiPoint::new(coords)), z)
        }
        Geom::LineString(line) => {
            let (line, z) = round_line(line, scale, origin)?;
            (Geometry::LineString(line), z)
        }
        Geom::MultiLineString(parts) => {
            let (lines, z) = gather(parts.iter().map(|p| round_line(p, scale, origin)))?;
            (Geometry::MultiLineString(MultiLineString::new(lines)), z)
        }
        Geom::Polygon(rings) => {
            let (polygon, z) = round_polygon(rings, scale, origin)?;
            (Geometry::Polygon(polygon), z)
        }
        Geom::MultiPolygon(polygons) => {
            let (polygons, z) = gather(polygons.iter().map(|p| round_polygon(p, scale, origin)))?;
            (Geometry::MultiPolygon(MultiPolygon::new(polygons)), z)
        }
    })
}

/// The part of a source that reaches tile `(zoom, col, row)`. A feature whose
/// bounding box misses the tile's buffered rectangle is skipped, one lying
/// wholly inside it shares its geometry unclipped, and the rest are clipped.
/// A point belongs to the one tile its [`tile_index`] names.
fn clip_to_tile(src: &Source, zoom: u8, col: u32, row: u32) -> Option<Source> {
    let rect = tile_rect(zoom, col, row);
    let [x0, y0, x1, y1] = src.bbox;
    if x1 < rect.min_x || x0 > rect.max_x || y1 < rect.min_y || y0 > rect.max_y {
        return None;
    }
    let is_points = matches!(*src.geom, Geom::Point(_) | Geom::MultiPoint(_));
    let inside = x0 >= rect.min_x && x1 <= rect.max_x && y0 >= rect.min_y && y1 <= rect.max_y;
    let (geom, bbox) = if inside && !is_points {
        (Arc::clone(&src.geom), src.bbox)
    } else {
        let n = 1_u32 << zoom;
        let owns = |v: &Vertex| tile_index(v.x, n) == col && tile_index(v.y, n) == row;
        let geom = clip::clip_geom(&src.geom, &rect, owns)?;
        let bbox = geom.bbox()?;
        (Arc::new(geom), bbox)
    };
    Some(Source {
        id: src.id,
        geom,
        bbox,
        props: Arc::clone(&src.props),
    })
}

/// Walks the tile quadtree depth first, writing each tile from its features and
/// descending only into children those features reach. Each level clips the
/// parent's already-clipped geometry, so the work is proportional to the tiles
/// the data touches rather than to its bounding box.
struct Tiler<'a> {
    output: &'a Path,
    layer: &'a LayerSpec,
    min_zoom: u8,
    max_zoom: u8,
}

impl Tiler<'_> {
    /// Write tile `(zoom, col, row)` from `features` when the zoom is wanted, then
    /// its four children in parallel. Returns `(tiles, features)` written below here.
    fn tile(&self, zoom: u8, col: u32, row: u32, features: &[Source]) -> AnyResult<(usize, usize)> {
        if features.is_empty() {
            return Ok((0, 0));
        }
        let mut written = (0, 0);
        if zoom >= self.min_zoom {
            let entries: Vec<Entry> = features
                .iter()
                .filter_map(|f| {
                    to_tile(&f.geom, zoom, col, row).map(|(geometry, z)| Entry {
                        id: f.id,
                        geometry,
                        z,
                        props: Arc::clone(&f.props),
                    })
                })
                .collect();
            if !entries.is_empty() {
                written = (1, entries.len());
                self.write_tile(zoom, col, row, entries)?;
            }
        }
        if zoom == self.max_zoom {
            return Ok(written);
        }

        let child_zoom = zoom + 1;
        let children =
            [(0, 0), (1, 0), (0, 1), (1, 1)].map(|(dx, dy)| (col * 2 + dx, row * 2 + dy));
        let below = children
            .into_par_iter()
            .map(|(col, row)| {
                let reached: Vec<Source> = features
                    .iter()
                    .filter_map(|f| clip_to_tile(f, child_zoom, col, row))
                    .collect();
                self.tile(child_zoom, col, row, &reached)
            })
            .collect::<AnyResult<Vec<_>>>()?;
        for (tiles, features) in below {
            written.0 += tiles;
            written.1 += features;
        }
        Ok(written)
    }

    fn write_tile(&self, zoom: u8, col: u32, row: u32, entries: Vec<Entry>) -> AnyResult<()> {
        let bytes = encode_tile(self.layer, entries)
            .with_context(|| format!("encoding tile {zoom}/{col}/{row}"))?;
        let dir = self.output.join(zoom.to_string()).join(col.to_string());
        fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        let path = dir.join(format!("{row}.mlt"));
        fs::write(&path, bytes).with_context(|| format!("writing {}", path.display()))
    }
}

fn encode_tile(layer: &LayerSpec, entries: Vec<Entry>) -> AnyResult<Vec<u8>> {
    let mut builder = TileLayer::builder(layer.name.clone(), EXTENT)?;
    if let Some(step) = layer.z_step {
        builder.set_z_step(step)?;
    }
    let keys = layer
        .schema
        .columns
        .iter()
        .map(|(name, kind)| builder.add_property(name.clone(), *kind))
        .collect::<Result<Vec<_>, _>>()?;

    for entry in entries {
        let mut feature = builder.feature(entry.geometry);
        feature.id(entry.id);
        if layer.z_step.is_some() {
            feature.z(entry.z)?;
        }
        for (idx, value) in entry.props.iter() {
            feature.property(keys[*idx], value.clone())?;
        }
        feature.finish()?;
    }
    let cfg = EncoderConfig::default().with_wire_version(WireVersion::V02);
    Ok(builder.finish().encode(cfg)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f64, y: f64, z: f64) -> Vertex {
        Vertex { x, y, z }
    }

    const ORIGIN: (f64, f64) = (0.0, 0.0);

    #[test]
    fn tile_index_floors_and_clamps() {
        assert_eq!(tile_index(0.0, 4), 0);
        assert_eq!(tile_index(0.2499, 4), 0);
        assert_eq!(tile_index(0.25, 4), 1);
        assert_eq!(tile_index(-0.1, 4), 0);
        assert_eq!(tile_index(1.0, 4), 3);
    }

    #[test]
    fn a_ring_keeps_its_stored_vertices_and_their_z() {
        let open = [
            v(0.2, 0.0, 1.0),
            v(10.0, 0.0, 2.0),
            v(10.0, 10.0, 3.0),
            v(0.0, 10.0, 4.4),
        ];
        let (ring, z) = round_ring(&open, 1.0, ORIGIN).unwrap();
        assert_eq!(ring.0.len(), 5);
        assert_eq!(ring.0[0], ring.0[4]);
        assert_eq!(z, [1, 2, 3, 4]);
    }

    #[test]
    fn a_ring_collapsing_under_rounding_is_dropped() {
        let open = [v(0.1, 0.1, 0.0), v(0.2, 0.2, 0.0), v(0.3, 0.1, 0.0)];
        assert!(round_ring(&open, 1.0, ORIGIN).is_none());
    }

    #[test]
    fn a_closed_input_ring_is_not_double_closed() {
        let open = [
            v(0.0, 0.0, 0.0),
            v(10.0, 0.0, 0.0),
            v(10.0, 10.0, 0.0),
            v(0.0, 0.0, 0.0),
        ];
        let (ring, z) = round_ring(&open, 1.0, ORIGIN).unwrap();
        assert_eq!(ring.0.len(), 4);
        assert_eq!(z.len(), 3);
    }

    fn source(geom: Geom) -> Source {
        Source {
            id: None,
            bbox: geom.bbox().unwrap(),
            geom: Arc::new(geom),
            props: Arc::new(Vec::new()),
        }
    }

    #[test]
    fn a_point_reaches_exactly_one_child() {
        let src = source(Geom::Point(v(0.5, 0.5, 7.0)));
        let reached: Vec<(u32, u32)> = [(0, 0), (1, 0), (0, 1), (1, 1)]
            .into_iter()
            .filter(|&(c, r)| clip_to_tile(&src, 1, c, r).is_some())
            .collect();
        assert_eq!(reached, [(1, 1)]);
    }

    #[test]
    fn a_multi_point_is_split_between_the_children_its_points_fall_in() {
        let src = source(Geom::MultiPoint(vec![v(0.1, 0.1, 1.0), v(0.9, 0.1, 2.0)]));
        let child = clip_to_tile(&src, 1, 1, 0).expect("a multi point in tile 1/1/0");
        assert_eq!(*child.geom, Geom::MultiPoint(vec![v(0.9, 0.1, 2.0)]));
        assert!(clip_to_tile(&src, 1, 1, 1).is_none());
    }

    #[test]
    fn a_feature_missing_a_child_is_skipped_and_one_inside_it_is_shared_unclipped() {
        let src = source(Geom::LineString(vec![v(0.1, 0.1, 0.0), v(0.2, 0.2, 0.0)]));
        assert!(clip_to_tile(&src, 1, 1, 1).is_none());
        let child = clip_to_tile(&src, 1, 0, 0).expect("a line in tile 1/0/0");
        assert!(Arc::ptr_eq(&child.geom, &src.geom));
        assert_eq!(child.bbox, src.bbox);
    }

    #[test]
    fn a_feature_straddling_a_child_edge_is_clipped_with_a_fresh_bbox() {
        let src = source(Geom::LineString(vec![
            v(0.25, 0.5, 0.0),
            v(0.75, 0.5, 100.0),
        ]));
        let child = clip_to_tile(&src, 1, 0, 0).expect("a line in tile 1/0/0");
        assert!(!Arc::ptr_eq(&child.geom, &src.geom));
        let east = 0.5 + 64.0 / 8192.0;
        assert!((child.bbox[2] - east).abs() < 1e-12, "{:?}", child.bbox);
    }

    #[test]
    fn to_tile_rounds_onto_the_tile_grid() {
        let geom = Geom::LineString(vec![v(0.25, 0.5, 0.4), v(0.5, 0.5, 100.6)]);
        let (geometry, z) = to_tile(&geom, 1, 0, 0).unwrap();
        let Geometry::LineString(line) = geometry else {
            panic!("a line string, got {geometry:?}");
        };
        assert_eq!(
            line.0,
            [Coord { x: 2048, y: 4096 }, Coord { x: 4096, y: 4096 }]
        );
        assert_eq!(z, [0, 101]);
    }

    #[test]
    fn a_multi_line_string_losing_every_part_to_rounding_is_dropped() {
        let geom = Geom::MultiLineString(vec![vec![v(0.5, 0.5, 0.0), v(0.500_000_01, 0.5, 1.0)]]);
        assert!(to_tile(&geom, 0, 0, 0).is_none());
    }

    #[test]
    fn a_polygon_whose_hole_collapses_keeps_only_the_shell() {
        let rings = vec![
            vec![
                v(0.0, 0.0, 1.0),
                v(10.0, 0.0, 1.0),
                v(10.0, 10.0, 1.0),
                v(0.0, 10.0, 1.0),
            ],
            vec![v(5.0, 5.0, 2.0), v(5.1, 5.0, 2.0), v(5.1, 5.1, 2.0)],
        ];
        let (polygon, z) = round_polygon(&rings, 1.0, ORIGIN).unwrap();
        assert_eq!(polygon.interiors(), &[] as &[LineString<i32>]);
        assert_eq!(z, [1; 4]);
    }
}
