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
mod feature_id;
mod project;
mod round;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context as _, Result as AnyResult, bail};
use clap::Args;
use geojson::{Feature, FeatureCollection, GeoJson};
use martin_tile_utils::MAX_ZOOM;
use mlt_core::encoder::{EncoderConfig, WireVersion};
use mlt_core::geo_types::Geometry;
use mlt_core::geojson::PropertySchema;
use mlt_core::{Decoder, MltError, Parser, PropValue, TileLayer, ZStep};
use rayon::prelude::*;

use self::clip::Rect;
use self::feature_id::feature_id;
use self::project::{Dims, Geom, Projector, Vertex};
use self::round::to_tile;
use crate::convert::ContainerFormat;

#[derive(Args)]
pub struct FromGeoJsonArgs {
    /// Input: a WGS84 `GeoJSON` file holding a `FeatureCollection`
    input: PathBuf,
    /// Output: a directory for the `z/x/y.mlt` tile tree, which must not exist yet
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
    schema: PropertySchema,
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
    if output.exists() {
        bail!(
            "Output {} already exists; refusing to append. \
             Delete it first or choose a different path.",
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
    let schema = PropertySchema::infer(features.iter().map(|f| f.properties.iter().flatten()))?;
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
            id: feature_id(feature)
                .with_context(|| format!("reading the id of feature {index}"))?,
            geom: Arc::new(geom),
            bbox,
            props: Arc::new(match &feature.properties {
                Some(properties) => schema.values(properties)?,
                None => Vec::new(),
            }),
        });
    }
    // The parsed tree is no longer needed; it would otherwise double peak memory while tiling.
    drop(features);
    check_dimensions(projector.dims(), z_step)?;

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
    let written = tiler.tile(0, 0, 0, &sources)?;
    let (tiles_written, features_written) = (written.tiles, written.features);

    // TODO: simplify geometries at low zooms, as other tilers do, so the tiles there
    // stay light enough to decode, rather than failing and asking for a higher --min-zoom.
    if let Some((zoom, col, row)) = written.too_heavy {
        let fits = zoom + 1;
        let max_zoom = if fits > args.max_zoom {
            format!(" and a --max-zoom of at least {fits}")
        } else {
            String::new()
        };
        bail!(
            "tile {zoom}/{col}/{row} needs more memory to decode than mlt-core's default \
             decoder budget allows, so it and any such tile at a lower zoom were not written; \
             pass --min-zoom {fits}{max_zoom}"
        );
    }

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

/// `--z-step` must match whether the positions carry an altitude. `dims` is
/// `None` when the file has no positions.
fn check_dimensions(dims: Option<Dims>, z_step: Option<ZStep>) -> AnyResult<()> {
    match (dims == Some(Dims::Xyz), z_step) {
        (true, None) => bail!(
            "the GeoJSON positions carry an altitude, so --z-step is required \
             (the power of ten of the z grid's step in metres, -3..=4)"
        ),
        (false, Some(_)) => bail!(
            "--z-step needs [lon, lat, alt] positions, but the GeoJSON positions have no altitude"
        ),
        (true, Some(_)) | (false, None) => Ok(()),
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

/// What the tiles of one subtree of the walk came to.
#[derive(Clone, Copy, Default)]
struct Written {
    tiles: usize,
    features: usize,
    /// `(zoom, col, row)` of the highest-zoom tile left unwritten for needing more
    /// than the default decoder budget, if any.
    too_heavy: Option<(u8, u32, u32)>,
}

impl Written {
    fn add(&mut self, other: Self) {
        self.tiles += other.tiles;
        self.features += other.features;
        // `None` sorts first and the tuples by zoom first, so this keeps the highest zoom.
        self.too_heavy = self.too_heavy.max(other.too_heavy);
    }
}

impl Tiler<'_> {
    /// Write tile `(zoom, col, row)` from `features` when the zoom is wanted, then
    /// its four children in parallel. Returns what the tiles below here came to.
    fn tile(&self, zoom: u8, col: u32, row: u32, features: &[Source]) -> AnyResult<Written> {
        let mut written = Written::default();
        if features.is_empty() {
            return Ok(written);
        }
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
                let count = entries.len();
                if self.write_tile(zoom, col, row, entries)? {
                    written.tiles = 1;
                    written.features = count;
                } else {
                    written.too_heavy = Some((zoom, col, row));
                }
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
        for child in below {
            written.add(child);
        }
        Ok(written)
    }

    /// Encode and write tile `(zoom, col, row)`. Returns `false`, writing nothing,
    /// when the tile needs more than the default decoder budget.
    fn write_tile(&self, zoom: u8, col: u32, row: u32, entries: Vec<Entry>) -> AnyResult<bool> {
        let bytes = encode_tile(self.layer, entries)
            .with_context(|| format!("encoding tile {zoom}/{col}/{row}"))?;
        if !fits_default_decoder(&bytes)
            .with_context(|| format!("decoding tile {zoom}/{col}/{row} back"))?
        {
            return Ok(false);
        }
        let dir = self.output.join(zoom.to_string()).join(col.to_string());
        fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        let path = dir.join(format!("{row}.mlt"));
        fs::write(&path, bytes).with_context(|| format!("writing {}", path.display()))?;
        Ok(true)
    }
}

/// Whether `bytes` decode into rows within the default memory budget, as a
/// renderer reading the tile would decode them.
fn fits_default_decoder(bytes: &[u8]) -> AnyResult<bool> {
    let decoded = Parser::default().parse_layers(bytes).and_then(|layers| {
        let mut dec = Decoder::default();
        layers
            .into_iter()
            .try_for_each(|layer| layer.into_tile(&mut dec).map(drop))
    });
    match decoded {
        Ok(()) => Ok(true),
        Err(MltError::MemoryLimitExceeded { .. }) => Ok(false),
        Err(e) => Err(e.into()),
    }
}

fn encode_tile(layer: &LayerSpec, entries: Vec<Entry>) -> AnyResult<Vec<u8>> {
    let mut builder = TileLayer::builder(layer.name.clone(), EXTENT)?;
    if let Some(step) = layer.z_step {
        builder.set_z_step(step)?;
    }
    let keys = layer
        .schema
        .names()
        .iter()
        .zip(layer.schema.kinds())
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

    #[test]
    fn tile_index_floors_and_clamps() {
        assert_eq!(tile_index(0.0, 4), 0);
        assert_eq!(tile_index(0.2499, 4), 0);
        assert_eq!(tile_index(0.25, 4), 1);
        assert_eq!(tile_index(-0.1, 4), 0);
        assert_eq!(tile_index(1.0, 4), 3);
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
}
