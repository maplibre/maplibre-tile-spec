//! WebAssembly bindings for the `MapLibre` Tile (MLT) format.
//!
//! # Design
//!
//! A single [`MltTile`] struct owns all decoded columnar data for every layer
//! in the tile.  No per-layer or per-feature WASM objects are created; instead,
//! every accessor takes explicit `(layer_idx, feature_idx)` arguments so the
//! JavaScript side can keep plain numeric indices rather than heap-allocated
//! wrapper objects.
//!
//! ## Geometry encoding
//!
//! [`MltTile::feature_geometry`] returns a flat [`js_sys::Int32Array`]:
//!
//! ```text
//! [ numRings,
//!   ring0_numPoints, x0, y0, x1, y1, …,
//!   ring1_numPoints, x0, y0, …,
//!   … ]
//! ```
//!
//! Rings are **open** (no repeated closing vertex), matching the contract of
//! `loadGeometry()` in `@mapbox/vector-tile`.
//!
//! ## ID encoding
//!
//! [`MltTile::feature_id`] returns `f64`.  Absent IDs are represented as
//! `f64::NAN` (≡ `NaN` in JS), which the TS wrapper converts to `undefined`.
//! IDs above `Number.MAX_SAFE_INTEGER` lose precision.

use std::cell::RefCell;
use std::f64;

use js_sys::{Float64Array, Int32Array, Object, Reflect, Uint8Array, Uint32Array};
use mlt_core::borrowme::{Borrow, ToOwned};
use mlt_core::v01::{
    DecodedId, GeometryType, Id, OwnedEncodedId, OwnedGeometry, OwnedProperty, PropValue,
};
use mlt_core::{MltError, parse_layers};
use wasm_bindgen::prelude::*;

/// All decoded geometry arrays for a single layer, fetched in one WASM call.
///
/// JS indexes into these typed arrays directly inside `loadGeometry()`, so
/// there are **zero** per-feature WASM boundary crossings for geometry.
///
/// ## Array semantics (mirrors `DecodedGeometry`)
///
/// All offset arrays are cumulative: `offsets[i]` is the start index and
/// `offsets[i+1]` is the exclusive end for feature/part/ring `i`.
/// Vertex indices count whole vertices (pairs), so vertex `n` lives at
/// `vertices[n*2]`, `vertices[n*2+1]`.
///
/// | Getter             | Present for                                         |
/// |--------------------|-----------------------------------------------------|
/// | `geometry_offsets` | MultiPoint, MultiLineString, MultiPolygon           |
/// | `part_offsets`     | LineString, Polygon, MultiLineString, MultiPolygon  |
/// | `ring_offsets`     | Polygon, MultiPolygon (+ LineString when mixed)     |
/// | `vertices`         | always                                              |
///
/// Absent offset arrays are returned as zero-length `Uint32Array`s so JS can
/// always branch on `.length` without a null-check.
#[wasm_bindgen]
pub struct LayerGeometry {
    geometry_offsets: Uint32Array,
    part_offsets: Uint32Array,
    ring_offsets: Uint32Array,
    vertices: Int32Array,
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
    /// for LineString layers without rings).
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

enum IdState {
    Absent,
    Encoded(OwnedEncodedId),
    Decoded(Vec<f64>),
}

struct DecodedLayer {
    name: String,
    extent: u32,
    vector_types: Vec<GeometryType>,
    geometry: RefCell<OwnedGeometry>,
    ids: RefCell<IdState>,
    /// Property columns
    props: RefCell<Vec<OwnedProperty>>,
    /// Cached JS string handles for property keys — built once on first
    /// `feature_properties` call and reused for every subsequent feature.
    /// Each entry corresponds 1-to-1 with the flattened key set produced by
    /// iterating `props` in the same order as `feature_properties` does.
    prop_keys: RefCell<Vec<JsValue>>,
}

/// A fully-decoded MLT tile.
///
/// Construct one via [`decode_tile`], then use the index-based accessors to
/// read layer metadata and per-feature data.  Layer metadata and geometry types
/// are decoded eagerly; ID, full geometry, and property columns are decoded
/// lazily on first access.
#[wasm_bindgen]
pub struct MltTile {
    layers: Vec<DecodedLayer>,
}

#[wasm_bindgen]
impl MltTile {
    /// Number of layers in this tile.
    #[must_use]
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// Name of layer `layer_idx`.
    #[must_use]
    pub fn layer_name(&self, layer_idx: usize) -> String {
        self.layers[layer_idx].name.clone()
    }

    /// Extent of layer `layer_idx` in tile coordinates (typically 4096).
    #[must_use]
    pub fn layer_extent(&self, layer_idx: usize) -> u32 {
        self.layers[layer_idx].extent
    }

    /// Number of features in layer `layer_idx`.
    #[must_use]
    pub fn feature_count(&self, layer_idx: usize) -> usize {
        self.layers[layer_idx].vector_types.len()
    }

    /// All geometry types for every feature in `layer_idx` as a `Uint8Array`.
    ///
    /// One byte per feature, using the same mapping as [`feature_type`]:
    /// `1` = Point, `2` = LineString, `3` = Polygon, `0` = Unknown.
    ///
    /// Fetching this once per layer (2 WASM calls total) and indexing into it
    /// in JS is far cheaper than calling `feature_type` once per feature.
    #[must_use]
    pub fn layer_types(&self, layer_idx: usize) -> Uint8Array {
        let types: Vec<u8> = self.layers[layer_idx]
            .vector_types
            .iter()
            .map(|t| match t {
                GeometryType::Point | GeometryType::MultiPoint => 1,
                GeometryType::LineString | GeometryType::MultiLineString => 2,
                GeometryType::Polygon | GeometryType::MultiPolygon => 3,
                #[allow(unreachable_patterns)]
                _ => 0,
            })
            .collect();
        Uint8Array::from(types.as_slice())
    }

    /// All feature IDs for every feature in `layer_idx` as a `Float64Array`.
    ///
    /// One `f64` per feature. Absent IDs are represented as `NaN`, matching
    /// the contract of [`feature_id`]. Fetching this once per layer and
    /// indexing into it in JS eliminates per-feature WASM boundary crossings.
    pub fn layer_ids(&self, layer_idx: usize) -> Result<Float64Array, JsError> {
        self.ensure_ids_decoded(layer_idx)?;
        let guard = self.layers[layer_idx].ids.borrow();
        let ids: &[f64] = match &*guard {
            IdState::Decoded(ids) => ids.as_slice(),
            _ => &[],
        };
        Ok(Float64Array::from(ids))
    }

    /// All decoded geometry arrays for layer `layer_idx`, fetched in one call.
    ///
    /// Returns a [`LayerGeometry`] whose typed-array getters expose the raw
    /// offset and vertex buffers so that JS can implement `loadGeometry(i)`
    /// with zero WASM boundary crossings per feature.
    ///
    /// The layer's geometry is decoded lazily on first access and cached, so
    /// subsequent calls for the same layer are cheap views over existing data.
    pub fn layer_geometry(&self, layer_idx: usize) -> Result<LayerGeometry, JsError> {
        self.ensure_geometry_decoded(layer_idx)?;

        let layer = &self.layers[layer_idx];
        let guard = layer.geometry.borrow();
        let d = match &*guard {
            OwnedGeometry::Decoded(d) => d,
            _ => unreachable!(),
        };

        let geometry_offsets = d
            .geometry_offsets
            .as_deref()
            .map(Uint32Array::from)
            .unwrap_or_else(|| Uint32Array::new_with_length(0));

        let part_offsets = d
            .part_offsets
            .as_deref()
            .map(Uint32Array::from)
            .unwrap_or_else(|| Uint32Array::new_with_length(0));

        let ring_offsets = d
            .ring_offsets
            .as_deref()
            .map(Uint32Array::from)
            .unwrap_or_else(|| Uint32Array::new_with_length(0));

        // vertices is Vec<i32>; reinterpret as &[i32] slice for Int32Array.
        let vertices = d
            .vertices
            .as_deref()
            .map(Int32Array::from)
            .unwrap_or_else(|| Int32Array::new_with_length(0));

        Ok(LayerGeometry {
            geometry_offsets,
            part_offsets,
            ring_offsets,
            vertices,
        })
    }

    /// Geometry for a single feature as a flat `Int32Array`.
    ///
    /// Encoding:
    /// ```text
    /// [ numRings,
    ///   ring0_numPoints, x0, y0, x1, y1, …,
    ///   ring1_numPoints, x0, y0, …, … ]
    /// ```
    ///
    /// The TS wrapper decodes this into `Point[][]` by iterating the prefix
    /// counts and constructing `new Point(x, y)` for each coordinate pair.
    pub fn feature_geometry(
        &self,
        layer_idx: usize,
        feature_idx: usize,
    ) -> Result<Int32Array, JsError> {
        self.ensure_geometry_decoded(layer_idx)?;

        let layer = &self.layers[layer_idx];
        let geom_guard = layer.geometry.borrow();

        let rings = match &*geom_guard {
            OwnedGeometry::Decoded(d) => d.to_mvt_rings(feature_idx).map_err(|e| to_js_err(&e))?,
            _ => unreachable!(),
        };

        // Pre-compute exact capacity: 1 (numRings) + per ring: 1 (len) + 2*points
        let cap = 1 + rings.iter().map(|r| 1 + r.len() * 2).sum::<usize>();
        let mut buf: Vec<i32> = Vec::with_capacity(cap);

        buf.push(i32::try_from(rings.len()).map_err(|_| JsError::new("ring count overflows i32"))?);
        for ring in rings {
            buf.push(
                i32::try_from(ring.len()).map_err(|_| JsError::new("ring length overflows i32"))?,
            );
            for [x, y] in ring {
                buf.push(x);
                buf.push(y);
            }
        }

        Ok(Int32Array::from(buf.as_slice()))
    }

    /// Properties for a single feature as a plain JS object.
    ///
    /// Only present (`Some`) values are included.  Null/absent optional property
    /// values are omitted from the object entirely, matching the behaviour of
    /// `@mapbox/vector-tile` which also skips null properties.
    #[must_use]
    pub fn feature_properties(&self, layer_idx: usize, feature_idx: usize) -> Object {
        let layer = &self.layers[layer_idx];

        // Decode all columns in-place on first access: take the vec out, decode
        // every Encoded element into Decoded, then put it back.
        {
            let mut guard = layer.props.borrow_mut();
            if guard.iter().any(|p| matches!(p, OwnedProperty::Encoded(_))) {
                let taken = std::mem::take(&mut *guard);
                *guard = taken
                    .into_iter()
                    .filter_map(|p| p.borrow().decode().ok().map(OwnedProperty::Decoded))
                    .collect();
            }
        }

        // Build the cached key vec on the very first feature access, then reuse
        // it for every subsequent feature.  This eliminates one `JsValue::from_str`
        // allocation per property key per feature — a measurable win when there
        // are many features and a fixed set of column names.
        {
            let mut keys = layer.prop_keys.borrow_mut();
            if keys.is_empty() {
                let guard = layer.props.borrow();
                for p in &*guard {
                    if let OwnedProperty::Decoded(prop) = p {
                        match &prop.values {
                            PropValue::SharedDict(items) => {
                                for item in items {
                                    let key = format!("{}{}", prop.name, item.suffix);
                                    keys.push(JsValue::from_str(&key));
                                }
                            }
                            _ => {
                                keys.push(JsValue::from_str(&prop.name));
                            }
                        }
                    }
                }
            }
        }

        let obj = Object::new();
        let guard = layer.props.borrow();
        let keys = layer.prop_keys.borrow();
        let mut key_idx = 0usize;
        for p in &*guard {
            if let OwnedProperty::Decoded(prop) = p {
                match &prop.values {
                    PropValue::SharedDict(items) => {
                        for item in items {
                            if let Some(val) = &item.values[feature_idx] {
                                let _ = Reflect::set(&obj, &keys[key_idx], &JsValue::from_str(val));
                            }
                            key_idx += 1;
                        }
                    }
                    _ => {
                        if let Some(val) = prop_to_js(&prop.values, feature_idx) {
                            let _ = Reflect::set(&obj, &keys[key_idx], &val);
                        }
                        key_idx += 1;
                    }
                }
            }
        }
        obj
    }

    fn ensure_geometry_decoded(&self, layer_idx: usize) -> Result<(), JsError> {
        let layer = &self.layers[layer_idx];
        let mut geom = layer.geometry.borrow_mut();
        if let OwnedGeometry::Encoded(encoded) = &*geom {
            let decoded = mlt_core::v01::Geometry::Encoded(encoded.borrow())
                .decode()
                .map_err(|e| to_js_err(&e))?;
            *geom = OwnedGeometry::Decoded(decoded);
        }
        Ok(())
    }

    fn ensure_ids_decoded(&self, layer_idx: usize) -> Result<(), JsError> {
        let layer = &self.layers[layer_idx];
        let mut ids = layer.ids.borrow_mut();
        if let IdState::Encoded(encoded) = &*ids {
            let decoded = Id::Encoded(encoded.borrow())
                .decode()
                .map_err(|e| to_js_err(&e))?;

            let converted = match decoded.0 {
                Some(v) => v
                    .into_iter()
                    .map(|f| {
                        #[expect(clippy::cast_precision_loss)]
                        f.map_or(f64::NAN, |f| f as f64)
                    })
                    .collect(),
                None => Vec::new(),
            };
            *ids = IdState::Decoded(converted);
        }
        Ok(())
    }
}

/// Decode a raw MLT tile blob and return an [`MltTile`].
///
/// Layer metadata and geometry types are decoded eagerly; ID, full geometry,
/// and property columns are decoded lazily on first access.
#[wasm_bindgen]
pub fn decode_tile(data: &[u8]) -> Result<MltTile, JsError> {
    let raw_layers = parse_layers(data).map_err(|e| to_js_err(&e))?;
    let mut layers = Vec::with_capacity(raw_layers.len());

    for raw_layer in raw_layers {
        // Skip non-Tag01 layers.
        let mlt_core::Layer::Tag01(layer01) = raw_layer else {
            continue;
        };

        let name = layer01.name.to_string();
        let extent = layer01.extent;

        // Decode ONLY geometry types to get the feature count and allow feature_type
        // to stay fast without full geometry decode.
        let vector_types = match &layer01.geometry {
            mlt_core::v01::Geometry::Encoded(e) => e
                .meta
                .clone()
                .decode_bits_u32()
                .map_err(|e| to_js_err(&e))?
                .decode_u32()
                .map_err(|e| to_js_err(&e))?
                .into_iter()
                .map::<Result<GeometryType, JsError>, _>(|v| {
                    Ok(u8::try_from(v)
                        .map_err(|_| JsError::new("invalid geometry type"))?
                        .try_into()
                        .map_err(|_| JsError::new("invalid geometry type"))?)
                })
                .collect::<Result<Vec<_>, _>>()?,
            mlt_core::v01::Geometry::Decoded(d) => d.vector_types.clone(),
        };

        let geometry = RefCell::new(ToOwned::to_owned(&layer01.geometry));

        let ids = RefCell::new(match layer01.id {
            Id::None => IdState::Absent,
            Id::Encoded(e) => IdState::Encoded(ToOwned::to_owned(&e)),
            Id::Decoded(DecodedId(Some(v))) => IdState::Decoded(
                v.into_iter()
                    .map(|f| {
                        #[expect(clippy::cast_precision_loss)]
                        f.map_or(f64::NAN, |f| f as f64)
                    })
                    .collect(),
            ),
            Id::Decoded(DecodedId(None)) => IdState::Absent,
        });

        let props = RefCell::new(layer01.properties.iter().map(ToOwned::to_owned).collect());

        layers.push(DecodedLayer {
            name,
            extent,
            vector_types,
            geometry,
            ids,
            props,
            prop_keys: RefCell::new(Vec::new()),
        });
    }

    Ok(MltTile { layers })
}

fn to_js_err(e: &MltError) -> JsError {
    JsError::new(&e.to_string())
}

/// Convert a single column value at row `i` to a JS primitive.
///
/// Returns `None` for absent optional values and for unsupported column types
/// (e.g. `SharedDict`), which causes the property to be omitted from the
/// output object rather than set to `null`.
#[allow(clippy::cast_precision_loss)]
fn prop_to_js(pv: &PropValue, i: usize) -> Option<JsValue> {
    match pv {
        PropValue::Bool(v) => v[i].map(JsValue::from_bool),
        PropValue::I8(v) => v[i].map(|n| JsValue::from_f64(f64::from(n))),
        PropValue::U8(v) => v[i].map(|n| JsValue::from_f64(f64::from(n))),
        PropValue::I32(v) => v[i].map(|n| JsValue::from_f64(f64::from(n))),
        PropValue::U32(v) => v[i].map(|n| JsValue::from_f64(f64::from(n))),
        // i64/u64 may lose precision for values beyond 2^53; this matches the
        // existing TS decoder behaviour and the VectorTileFeatureLike contract
        // which types properties as `number | string | boolean`.
        PropValue::I64(v) => v[i].map(|n| JsValue::from_f64(n as f64)),
        PropValue::U64(v) => v[i].map(|n| JsValue::from_f64(n as f64)),
        PropValue::F32(v) => v[i].map(|n| JsValue::from_f64(f64::from(n))),
        PropValue::F64(v) => v[i].map(JsValue::from_f64),
        PropValue::Str(v) => v[i].as_ref().map(|s| JsValue::from_str(s)),
        PropValue::SharedDict(_) => None,
    }
}
