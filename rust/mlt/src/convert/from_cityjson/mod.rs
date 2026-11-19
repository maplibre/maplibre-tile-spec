//! `mlt convert <file>.city.json <dir>`: tile the surfaces of a WGS84 `CityJSON` file into a v2 `Meshes` layer.

mod city;

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context as _, Result as AnyResult, bail};
use martin_tile_utils::EARTH_CIRCUMFERENCE;
use mlt_core::geo_types::{Geometry, MultiPoint, Point};
use mlt_core::geojson::PropertySchema;
use mlt_core::{HeightMode, MValue, MValueScope, PropKind, PropValue, TileLayer, ZAxis, ZStep};
use rayon::prelude::*;
use serde_json::Value;

use super::from_geojson::clip::clip_ring;
use super::from_geojson::project::{Vertex, grid_z, mercator_unit};
use super::from_geojson::{EXTENT, tile_rect};
use super::orient::orient_outward;
use super::triangulate::{Triangulator, has_area};
use super::{ConvertArgs, Reencoder};

/// Whether `path` names a `CityJSON` file.
pub(super) fn is_cityjson(path: &Path) -> bool {
    path.file_name()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|name| name.ends_with(".city.json"))
}

/// What every tile's layer shares.
struct LayerSpec {
    name: String,
    schema: PropertySchema,
    z_axis: ZAxis,
    /// Every semantic surface type, which a triangle's label indexes.
    surfaces: Vec<String>,
}

/// A city object's triangles in the Web Mercator unit square, z in grid units.
struct Source {
    triangles: Arc<Vec<[Vertex; 3]>>,
    /// Each triangle's index into [`LayerSpec::surfaces`], if its face has a surface type.
    labels: Arc<Vec<Option<usize>>>,
    bbox: [f64; 4],
    props: Arc<Vec<(usize, PropValue)>>,
}

/// What the strict mesh rules made the tiler change, summed over every mesh of every tile.
#[derive(Clone, Copy, Default)]
struct Cleanup {
    /// Triangles dropped for repeating a corner once rounded onto the grid.
    degenerate: usize,
    /// Meshes with a closed shell whose triangles faced inward, turned over.
    flipped: usize,
}

#[derive(Clone, Copy, Default)]
struct Written {
    tiles: usize,
    meshes: usize,
    cleanup: Cleanup,
}

impl Written {
    fn add(&mut self, other: Self) {
        self.tiles += other.tiles;
        self.meshes += other.meshes;
        self.cleanup.degenerate += other.cleanup.degenerate;
        self.cleanup.flipped += other.cleanup.flipped;
    }
}

/// Tile the `CityJSON` file `args.input` into the directory `args.output`.
pub(super) fn convert(args: &ConvertArgs, reencoder: &Reencoder) -> AnyResult<()> {
    let (input, output) = (args.input.as_path(), args.output.as_path());
    let (min_zoom, max_zoom) = args.tiled_zooms()?;
    let Some(z_step) = args.z_step else {
        bail!(
            "a CityJSON input is 3D, so --z-step is required (the power of ten of the z grid's step in metres, -3..=4)"
        );
    };
    let step = ZStep::new(z_step)?;
    let mode: HeightMode = args.height_mode.unwrap_or_default().into();
    let ground = match (mode, args.ground_attribute.as_deref()) {
        (HeightMode::RelativeToGround, Some(name)) => Some(name),
        (HeightMode::RelativeToGround, None) => bail!(
            "--height-mode relative-to-ground needs --ground-attribute, naming the attribute that holds each object's ground height"
        ),
        (HeightMode::Absolute, Some(_)) => bail!(
            "--ground-attribute is subtracted from heights relative to the ground, so it needs --height-mode relative-to-ground"
        ),
        (HeightMode::Absolute, None) => None,
    };
    let name = args.tiled_layer_name()?;

    let text = fs::read_to_string(input).with_context(|| format!("reading {}", input.display()))?;
    let city = city::parse(&text, args.lod.as_deref())
        .with_context(|| format!("reading {}", input.display()))?;
    drop(text);

    let mut attributes = Vec::with_capacity(city.objects.len());
    for object in &city.objects {
        let mut map = object.attributes.clone();
        if map
            .insert("id".to_owned(), Value::String(object.id.clone()))
            .is_some()
        {
            bail!(
                "object {} has an attribute named id, which convert stores the object's key in",
                object.id
            );
        }
        attributes.push(map);
    }
    let schema = PropertySchema::infer(attributes.iter().map(|m| m.iter()))?;

    let mut surfaces: Vec<String> = Vec::new();
    let mut sources = Vec::with_capacity(city.objects.len());
    let mut triangulator = Triangulator::default();
    let mut face_triangles = Vec::new();
    for (object, attributes) in city.objects.iter().zip(&attributes) {
        let base = match ground {
            Some(name) => attributes
                .get(name)
                .and_then(Value::as_f64)
                .with_context(|| format!("object {} has no numeric {name} attribute", object.id))?,
            None => 0.0,
        };
        let lat0 = {
            let lats: Vec<f64> = object
                .faces
                .iter()
                .flat_map(|f| f.rings.iter().flatten())
                .map(|v| v[1])
                .collect();
            #[expect(
                clippy::cast_precision_loss,
                reason = "a vertex count is far below 2^53"
            )]
            let mean = lats.iter().sum::<f64>() / lats.len() as f64;
            mean
        };
        let metres_per_unit = EARTH_CIRCUMFERENCE * lat0.to_radians().cos();
        let mut triangles = Vec::new();
        let mut labels = Vec::new();
        for face in &object.faces {
            let mut projected = Vec::new();
            let mut metric = Vec::new();
            let mut ends = Vec::new();
            for (r, ring) in face.rings.iter().enumerate() {
                let mut p = Vec::with_capacity(ring.len());
                let mut m = Vec::with_capacity(ring.len());
                for &[lon, lat, height] in ring {
                    let metres = height - base;
                    if step.z(metres).is_none() {
                        bail!(
                            "object {} has a height of {metres} m, outside the z grid of 10^{z_step} m steps",
                            object.id
                        );
                    }
                    let (x, y) = mercator_unit(lon, lat);
                    p.push(Vertex {
                        x,
                        y,
                        z: grid_z(step, metres),
                    });
                    m.push([x * metres_per_unit, -y * metres_per_unit, metres]);
                }
                // `earcut` can loop forever on a hole without area.
                if r > 0 && !has_area(&m) {
                    continue;
                }
                projected.extend(p);
                metric.extend(m);
                ends.push(metric.len());
            }
            let label = face.surface.as_ref().map(|s| {
                surfaces
                    .iter()
                    .position(|known| known == s)
                    .unwrap_or_else(|| {
                        surfaces.push(s.clone());
                        surfaces.len() - 1
                    })
            });
            triangulator.polygon(&metric, &ends, false, &mut face_triangles);
            for triangle in face_triangles.drain(..) {
                triangles.push(triangle.map(|i| projected[i as usize]));
                labels.push(label);
            }
        }
        let Some(bbox) = bbox(&triangles) else {
            continue;
        };
        sources.push(Source {
            triangles: Arc::new(triangles),
            labels: Arc::new(labels),
            bbox,
            props: Arc::new(schema.values(attributes)?),
        });
    }
    drop(attributes);

    let layer = LayerSpec {
        name,
        schema,
        z_axis: ZAxis { step, mode },
        surfaces,
    };
    let tiler = Tiler {
        output,
        layer: &layer,
        reencoder,
        min_zoom,
        max_zoom,
    };
    let written = tiler.tile(0, 0, 0, &sources)?;
    eprintln!(
        "{} -> {}: wrote {} tile(s) holding {} mesh(es) of LoD {} at zooms {min_zoom}..={max_zoom}",
        input.display(),
        output.display(),
        written.tiles,
        written.meshes,
        city.lod,
    );
    if city.skipped > 0 {
        eprintln!(
            "skipped {} object(s) with no surfaces at LoD {}",
            city.skipped, city.lod
        );
    }
    let Cleanup {
        degenerate,
        flipped,
    } = written.cleanup;
    eprintln!(
        "strict mesh rules: dropped {degenerate} degenerate triangle(s), flipped {flipped} inside-out mesh(es)"
    );
    Ok(())
}

fn bbox(triangles: &[[Vertex; 3]]) -> Option<[f64; 4]> {
    triangles.iter().flatten().fold(None, |acc, v| {
        let [x0, y0, x1, y1] = acc.unwrap_or([v.x, v.y, v.x, v.y]);
        Some([x0.min(v.x), y0.min(v.y), x1.max(v.x), y1.max(v.y)])
    })
}

/// The part of a source that reaches tile `(zoom, col, row)`.
/// A source wholly inside the buffered tile is shared, and the rest have each triangle cut to it.
fn clip_to_tile(src: &Source, zoom: u8, col: u32, row: u32) -> Option<Source> {
    let rect = tile_rect(zoom, col, row);
    let [x0, y0, x1, y1] = src.bbox;
    if x1 < rect.min_x || x0 > rect.max_x || y1 < rect.min_y || y0 > rect.max_y {
        return None;
    }
    if x0 >= rect.min_x && x1 <= rect.max_x && y0 >= rect.min_y && y1 <= rect.max_y {
        return Some(Source {
            triangles: Arc::clone(&src.triangles),
            labels: Arc::clone(&src.labels),
            bbox: src.bbox,
            props: Arc::clone(&src.props),
        });
    }
    let mut triangles = Vec::new();
    let mut labels = Vec::new();
    for (triangle, &label) in src.triangles.iter().zip(src.labels.iter()) {
        // A triangle cut by a rectangle is convex, so a fan from its first corner keeps its winding.
        let cut = clip_ring(triangle, &rect);
        for pair in cut.windows(2).skip(1) {
            triangles.push([cut[0], pair[0], pair[1]]);
            labels.push(label);
        }
    }
    Some(Source {
        bbox: bbox(&triangles)?,
        triangles: Arc::new(triangles),
        labels: Arc::new(labels),
        props: Arc::clone(&src.props),
    })
}

/// One source's triangles rounded onto a tile's grid and cleaned for the strict mesh rules.
struct Rounded {
    vertices: Vec<[i32; 3]>,
    triangles: Vec<[u32; 3]>,
    labels: Vec<Option<usize>>,
}

/// Round a source onto tile `(zoom, col, row)`.
/// Corners on one grid position share a vertex, and a triangle left repeating a corner is dropped.
#[expect(
    clippy::cast_possible_truncation,
    reason = "x/y lie within the buffered tile, and z was range-checked when projected"
)]
fn round(src: &Source, zoom: u8, col: u32, row: u32, cleanup: &mut Cleanup) -> Option<Rounded> {
    let scale = f64::from(EXTENT) * f64::from(1_u32 << zoom);
    let origin = (
        f64::from(col) * f64::from(EXTENT),
        f64::from(row) * f64::from(EXTENT),
    );
    let grid = |v: &Vertex| {
        [
            (v.x.mul_add(scale, -origin.0)).round() as i32,
            (v.y.mul_add(scale, -origin.1)).round() as i32,
            v.z.round() as i32,
        ]
    };
    let mut index = HashMap::<[i32; 3], u32>::new();
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    let mut labels = Vec::new();
    for (triangle, &label) in src.triangles.iter().zip(src.labels.iter()) {
        let corners = triangle.map(|v| grid(&v));
        if corners[0] == corners[1] || corners[1] == corners[2] || corners[2] == corners[0] {
            cleanup.degenerate += 1;
            continue;
        }
        let corners = corners.map(|corner| {
            *index.entry(corner).or_insert_with(|| {
                vertices.push(corner);
                u32::try_from(vertices.len() - 1)
                    .expect("a tile's mesh has fewer than 2^32 vertices")
            })
        });
        triangles.push(corners);
        labels.push(label);
    }
    (!triangles.is_empty()).then_some(Rounded {
        vertices,
        triangles,
        labels,
    })
}

struct Tiler<'a> {
    output: &'a Path,
    layer: &'a LayerSpec,
    reencoder: &'a Reencoder,
    min_zoom: u8,
    max_zoom: u8,
}

impl Tiler<'_> {
    fn tile(&self, zoom: u8, col: u32, row: u32, sources: &[Source]) -> AnyResult<Written> {
        let mut written = Written::default();
        if sources.is_empty() {
            return Ok(written);
        }
        if zoom >= self.min_zoom {
            written = self
                .write_tile(zoom, col, row, sources)
                .with_context(|| format!("writing tile {zoom}/{col}/{row}"))?;
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
                let reached: Vec<Source> = sources
                    .iter()
                    .filter_map(|s| clip_to_tile(s, child_zoom, col, row))
                    .collect();
                self.tile(child_zoom, col, row, &reached)
            })
            .collect::<AnyResult<Vec<_>>>()?;
        for child in below {
            written.add(child);
        }
        Ok(written)
    }

    fn write_tile(&self, zoom: u8, col: u32, row: u32, sources: &[Source]) -> AnyResult<Written> {
        let spec = self.layer;
        let mut written = Written::default();
        let mut meshes = Vec::new();
        for src in sources {
            let Some(mut mesh) = round(src, zoom, col, row, &mut written.cleanup) else {
                continue;
            };
            if orient_outward(&mesh.vertices, &mut mesh.triangles) > 0 {
                written.cleanup.flipped += 1;
            }
            meshes.push((src, mesh));
        }
        if meshes.is_empty() {
            return Ok(written);
        }
        let mut builder = TileLayer::builder(spec.name.clone(), EXTENT)?;
        builder.set_meshes(spec.z_axis)?;
        let keys = spec
            .schema
            .names()
            .iter()
            .zip(spec.schema.kinds())
            .map(|(name, kind)| builder.add_property(name.clone(), *kind))
            .collect::<Result<Vec<_>, _>>()?;
        let surface = (!spec.surfaces.is_empty())
            .then(|| builder.add_scoped_m_value("surface", PropKind::Str, MValueScope::Triangle))
            .transpose()?;
        for (src, mesh) in meshes {
            let points = mesh.vertices.iter().map(|&[x, y, _]| Point::new(x, y));
            let mut feature = builder.feature(Geometry::MultiPoint(MultiPoint(points.collect())));
            feature.z(mesh.vertices.iter().map(|v| v[2]).collect())?;
            feature.triangles(mesh.triangles)?;
            for (idx, value) in src.props.iter() {
                feature.property(keys[*idx], value.clone())?;
            }
            if let Some(key) = surface
                && mesh.labels.iter().any(Option::is_some)
            {
                let values = mesh
                    .labels
                    .iter()
                    .map(|l| l.map_or("", |l| spec.surfaces[l].as_str()).into());
                feature.m_value(key, MValue::Str(Some(values.collect())))?;
            }
            feature.finish()?;
            written.meshes += 1;
        }
        let bytes = self.reencoder.encode(builder.finish())?;
        let dir = self.output.join(zoom.to_string()).join(col.to_string());
        fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        let path = dir.join(format!("{row}.mlt"));
        fs::write(&path, bytes).with_context(|| format!("writing {}", path.display()))?;
        written.tiles = 1;
        Ok(written)
    }
}
