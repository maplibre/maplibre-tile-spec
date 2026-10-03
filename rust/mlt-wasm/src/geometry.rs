use js_sys::{Int32Array, Uint32Array};
use mlt_core::wire::GeoLayout;
use mlt_core::{GeometryValues, ZStep};
use wasm_bindgen::prelude::*;

/// All decoded geometry arrays for a single layer, fetched in one WASM call.
///
/// JS indexes into these typed arrays directly inside `loadGeometry()`, so
/// there are **zero** per-feature WASM boundary crossings for geometry.
///
/// ## Array semantics (mirrors `GeometryValues`)
///
/// All offset arrays are cumulative: `offsets[i]` is the start index and
/// `offsets[i+1]` is the exclusive end for feature/part/ring `i`.
/// Vertex indices count whole vertices (pairs), so vertex `n` lives at
/// `vertices[n*2]`, `vertices[n*2+1]`.
///
/// | Getter             | Present for                                               |
/// |--------------------|-----------------------------------------------------------|
/// | `geometry_offsets` | `MultiPoint`, `MultiLineString`, `MultiPolygon`           |
/// | `part_offsets`     | `LineString`, `Polygon`, `MultiLineString`, `MultiPolygon`|
/// | `ring_offsets`     | `Polygon`, `MultiPolygon` (+ `LineString` when mixed)     |
/// | `vertices`         | always                                                    |
/// | `z`                | a layer whose vertices carry z coordinates                |
///
/// Absent offset arrays are returned as zero-length `Uint32Array`s so JS can
/// always branch on `.length` without a null-check.
#[wasm_bindgen]
pub struct LayerGeometry {
    pub(crate) geometry_offsets: Uint32Array,
    pub(crate) part_offsets: Uint32Array,
    pub(crate) ring_offsets: Uint32Array,
    pub(crate) vertices: Int32Array,
    pub(crate) z: Int32Array,
    pub(crate) z_step: Option<i8>,
}

#[wasm_bindgen]
impl LayerGeometry {
    /// Cumulative offsets into `part_offsets` for multi-geometry types.
    /// Zero-length when no multi-geometry features are present.
    #[must_use]
    pub fn geometry_offsets(&self) -> Uint32Array {
        self.geometry_offsets.clone()
    }

    /// Cumulative offsets into `ring_offsets` (or directly into `vertices`
    /// for `LineString` layers without rings).
    /// Zero-length for pure Point layers.
    #[must_use]
    pub fn part_offsets(&self) -> Uint32Array {
        self.part_offsets.clone()
    }

    /// Cumulative offsets into the vertex buffer (counting whole vertices).
    /// Zero-length when no ring-level indirection is needed.
    #[must_use]
    pub fn ring_offsets(&self) -> Uint32Array {
        self.ring_offsets.clone()
    }

    /// Flat vertex buffer: `[x0, y0, x1, y1, …]` in tile coordinates.
    #[must_use]
    pub fn vertices(&self) -> Int32Array {
        self.vertices.clone()
    }

    /// One z per vertex, parallel to [`Self::vertices`], or zero-length when the layer has none.
    #[must_use]
    pub fn z(&self) -> Int32Array {
        self.z.clone()
    }

    /// The z grid as the power of ten of its step in metres, or `undefined` when the layer has none.
    #[must_use]
    #[wasm_bindgen(js_name = zStep)]
    pub fn z_step(&self) -> Option<i8> {
        self.z_step
    }
}

impl LayerGeometry {
    /// Build a [`LayerGeometry`] from a decoded [`GeometryValues`].
    pub(crate) fn from_values(geom: &GeometryValues) -> Self {
        let geometry_offsets = geom
            .geometry_offsets()
            .map_or_else(|| Uint32Array::new_with_length(0), Uint32Array::from);

        let part_offsets = geom
            .part_offsets()
            .map_or_else(|| Uint32Array::new_with_length(0), Uint32Array::from);

        let ring_offsets = geom
            .ring_offsets()
            .map_or_else(|| Uint32Array::new_with_length(0), Uint32Array::from);

        // JS reads the vertices as pairs, so a layer's z travel in an array of their own.
        let words = geom.vertices().unwrap_or_default();
        let (vertices, z) = if geom.z_step().is_some() {
            let triples = words.as_chunks::<3>().0;
            let xy: Vec<i32> = triples.iter().flat_map(|&[x, y, _]| [x, y]).collect();
            let z: Vec<i32> = triples.iter().map(|&[_, _, z]| z).collect();
            (Int32Array::from(&xy[..]), Int32Array::from(&z[..]))
        } else {
            (Int32Array::from(words), Int32Array::new_with_length(0))
        };

        Self {
            geometry_offsets,
            part_offsets,
            ring_offsets,
            vertices,
            z,
            z_step: geom.z_step().map(ZStep::exponent),
        }
    }
}

/// The z grid of a layer `decodeTile3D` can read, or why it cannot.
///
/// It needs z coordinates, and rings to walk: a tessellated layout is rejected,
/// with or without its outlines, since `decodeTile3D` does not support tessellation.
pub(crate) fn z_step_3d(geom: &GeometryValues, layout: Option<GeoLayout>) -> Result<ZStep, String> {
    let Some(step) = geom.z_step() else {
        return Err("has no z coordinates; decode the tile with decodeTile() instead".into());
    };
    match layout {
        Some(layout @ (GeoLayout::TessPolygons | GeoLayout::TessPolygonsWithOutlines)) => {
            let name: &str = layout.into();
            Err(format!(
                "uses the {name} geometry layout, which decodeTile3D does not support"
            ))
        }
        _ => Ok(step),
    }
}

/// Each vertex's z as an elevation in metres, or none in a layer without z.
pub(crate) fn elevations(geom: &GeometryValues) -> Vec<f64> {
    let Some(step) = geom.z_step() else {
        return Vec::new();
    };
    let words = geom.vertices().unwrap_or_default();
    let triples = words.as_chunks::<3>().0;
    triples.iter().map(|&[_, _, z]| step.elevation(z)).collect()
}

#[cfg(test)]
mod tests {
    use mlt_core::encoder::{EncoderConfig, WireVersion};
    use mlt_core::geo_types::{Coord, Geometry, LineString, Polygon};
    use mlt_core::wire::GeoLayout;
    use mlt_core::{Decoder, GeometryValues, ParsedLayer, Parser, TileLayer, ZStep};

    use super::{elevations, z_step_3d};

    fn coords(pts: &[(i32, i32)]) -> LineString<i32> {
        LineString(pts.iter().map(|&(x, y)| Coord { x, y }).collect())
    }

    fn line() -> Geometry<i32> {
        Geometry::LineString(coords(&[(1, 2), (3, 4)]))
    }

    /// A square, which stores 4 vertices.
    fn square() -> Geometry<i32> {
        Geometry::Polygon(Polygon::new(
            coords(&[(0, 0), (10, 0), (10, 10), (0, 10), (0, 0)]),
            vec![],
        ))
    }

    fn v2() -> EncoderConfig {
        EncoderConfig::default().with_wire_version(WireVersion::V02)
    }

    /// Encode `geom` as a v2 layer with `cfg`, its vertices on `z` when given, and decode it
    /// back into the geometry and layout `decode_tile` holds.
    fn decoded(
        geom: Geometry<i32>,
        z: Option<(ZStep, Vec<i32>)>,
        cfg: EncoderConfig,
    ) -> (GeometryValues, GeoLayout) {
        let mut builder = TileLayer::builder("layer", 4096).unwrap();
        if let Some((step, _)) = &z {
            builder.set_z_step(*step).unwrap();
        }
        let mut feature = builder.feature(geom);
        if let Some((_, z)) = z {
            feature.z(z).unwrap();
        }
        feature.finish().unwrap();
        let bytes = builder.finish().encode(cfg).unwrap();
        let layer = Parser::default().parse_layers(&bytes).unwrap().remove(0);
        let ParsedLayer::Tag02(layer) = layer.decode_all(&mut Decoder::default()).unwrap() else {
            panic!("expected a v2 layer")
        };
        (
            layer.layer().geometry_values().clone(),
            layer.layout().geometry,
        )
    }

    fn step() -> ZStep {
        ZStep::new(-1).unwrap()
    }

    /// A 3D square on [`step`], encoded with `cfg`.
    fn square_3d(cfg: EncoderConfig) -> (GeometryValues, GeoLayout) {
        decoded(square(), Some((step(), vec![100_000; 4])), cfg)
    }

    #[test]
    fn elevations_are_metres_on_the_z_step_grid() {
        // -10000 m + 100_120 dm = 12 m; -10000 m + 99_995 dm = -0.5 m
        let (geom, _) = decoded(line(), Some((step(), vec![100_120, 99_995])), v2());
        assert_eq!(elevations(&geom), [12.0, -0.5]);
    }

    #[test]
    fn a_flat_layer_has_no_elevations() {
        let (geom, _) = decoded(line(), None, v2());
        assert_eq!(elevations(&geom), [] as [f64; 0]);
    }

    #[test]
    fn a_3d_layer_of_rings_reads_in_3d() {
        let (geom, layout) = square_3d(v2());
        assert_eq!(layout, GeoLayout::Polygons);
        assert_eq!(z_step_3d(&geom, Some(layout)), Ok(step()));
    }

    #[test]
    fn a_flat_layer_does_not_read_in_3d() {
        let (geom, layout) = decoded(line(), None, v2());
        let err = z_step_3d(&geom, Some(layout)).unwrap_err();
        assert!(err.contains("no z coordinates"), "{err}");
    }

    #[test]
    fn a_tessellated_3d_layer_with_outlines_is_rejected() {
        let (geom, layout) = square_3d(v2().with_tessellation(true));
        assert_eq!(layout, GeoLayout::TessPolygonsWithOutlines);
        let err = z_step_3d(&geom, Some(layout)).unwrap_err();
        assert!(err.contains("TessPolygonsWithOutlines"), "{err}");
    }

    #[test]
    fn a_triangles_only_3d_layer_is_rejected() {
        let cfg = v2().with_tessellation(true).with_triangles_only(true);
        let (geom, layout) = square_3d(cfg);
        assert_eq!(layout, GeoLayout::TessPolygons);
        let err = z_step_3d(&geom, Some(layout)).unwrap_err();
        assert!(err.contains("the TessPolygons geometry layout"), "{err}");
    }
}
