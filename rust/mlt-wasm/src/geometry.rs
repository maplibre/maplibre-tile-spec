use js_sys::{Int32Array, Uint32Array};
use mlt_core::v01::GeometryValues;
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
///
/// Absent offset arrays are returned as zero-length `Uint32Array`s so JS can
/// always branch on `.length` without a null-check.
#[wasm_bindgen]
pub struct LayerGeometry {
    pub(crate) geometry_offsets: Uint32Array,
    pub(crate) part_offsets: Uint32Array,
    pub(crate) ring_offsets: Uint32Array,
    pub(crate) vertices: Int32Array,
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
}

impl LayerGeometry {
    /// Build a [`LayerGeometry`] from a decoded [`GeometryValues`].
    pub(crate) fn from_values(geom: &GeometryValues) -> Self {
        let geometry_offsets = geom
            .geometry_offsets
            .as_deref()
            .map_or_else(|| Uint32Array::new_with_length(0), Uint32Array::from);

        let part_offsets = geom
            .part_offsets
            .as_deref()
            .map_or_else(|| Uint32Array::new_with_length(0), Uint32Array::from);

        let ring_offsets = geom
            .ring_offsets
            .as_deref()
            .map_or_else(|| Uint32Array::new_with_length(0), Uint32Array::from);

        let vertices = geom
            .vertices
            .as_deref()
            .map_or_else(|| Int32Array::new_with_length(0), Int32Array::from);

        Self {
            geometry_offsets,
            part_offsets,
            ring_offsets,
            vertices,
        }
    }
}
