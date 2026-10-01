use js_sys::{Int32Array, Uint32Array};
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
