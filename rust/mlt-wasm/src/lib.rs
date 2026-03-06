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

use js_sys::{Int32Array, Object, Reflect};
use mlt_core::borrowme::{Borrow as _, ToOwned as _};
use mlt_core::v01::{
    DecodedGeometry, DecodedId, DecodedProperty, GeometryType, Id, OwnedProperty, PropValue,
    Property,
};
use mlt_core::{MltError, parse_layers};
use wasm_bindgen::prelude::*;

struct DecodedLayer {
    name: String,
    extent: u32,
    geometry: DecodedGeometry,
    /// `None` when the layer carries no ID column at all.
    ids: Option<Vec<Option<u64>>>,
    /// Property
    raw_props: RefCell<Vec<OwnedProperty>>,
    /// Decoded property columns, populated lazily.  `None` until first access.
    props: RefCell<Option<Vec<DecodedProperty>>>,
}

/// A fully-decoded MLT tile.
///
/// Construct one via [`decode_tile`], then use the index-based accessors to
/// read layer metadata and per-feature data.  Geometry and IDs are decoded
/// eagerly; property columns are decoded lazily on first access.
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
        self.layers[layer_idx].geometry.vector_types.len()
    }

    /// MVT geometry type for a feature.
    ///
    /// | Return | Meaning    |
    /// |--------|------------|
    /// | `1`    | Point      |
    /// | `2`    | LineString |
    /// | `3`    | Polygon    |
    /// | `0`    | Unknown    |
    ///
    /// Multi-part geometries (`MultiPoint`, `MultiLineString`, `MultiPolygon`) map
    /// to the same value as their single-part counterpart — the multi-ness is
    /// expressed through the ring structure returned by [`feature_geometry`].
    #[must_use]
    pub fn feature_type(&self, layer_idx: usize, feature_idx: usize) -> u8 {
        match self.layers[layer_idx]
            .geometry
            .vector_types
            .get(feature_idx)
        {
            Some(GeometryType::Point | GeometryType::MultiPoint) => 1,
            Some(GeometryType::LineString | GeometryType::MultiLineString) => 2,
            Some(GeometryType::Polygon | GeometryType::MultiPolygon) => 3,
            _ => 0,
        }
    }

    /// Feature ID as `f64`, or `NaN` when the feature has no ID.
    ///
    /// The TS wrapper converts `NaN` → `undefined` to match the
    /// `VectorTileFeatureLike.id: number | undefined` contract.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn feature_id(&self, layer_idx: usize, feature_idx: usize) -> f64 {
        self.layers[layer_idx]
            .ids
            .as_deref()
            .and_then(|ids| ids.get(feature_idx).copied().flatten())
            .map_or(f64::NAN, |id| id as f64)
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
        let rings = self.layers[layer_idx]
            .geometry
            .to_mvt_rings(feature_idx)
            .map_err(|e| to_js_err(&e))?;

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
    /// Property columns are decoded on the **first call** for each layer and
    /// cached; subsequent calls for any feature in the same layer are cheap
    /// index operations with no further parsing.
    ///
    /// Only present (`Some`) values are included.  Null/absent optional property
    /// values are omitted from the object entirely, matching the behaviour of
    /// `@mapbox/vector-tile` which also skips null properties.
    ///
    /// `SharedDict` (struct-typed) columns are not yet supported and are
    /// silently skipped.
    #[must_use]
    pub fn feature_properties(&self, layer_idx: usize, feature_idx: usize) -> Object {
        let layer = &self.layers[layer_idx];

        // Lazy decode: on first access drain `raw_props` and decode all columns
        // for this layer in one pass.  Every subsequent call just reads the cache.
        if layer.props.borrow().is_none() {
            let raw: Vec<OwnedProperty> = layer.raw_props.borrow_mut().drain(..).collect();
            let decoded: Vec<DecodedProperty> = raw
                .into_iter()
                .flat_map(|p| {
                    p.borrow()
                        .decode_expand()
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|prop| match prop {
                            Property::Decoded(d) => Some(d),
                            Property::Encoded(_) => None,
                        })
                        .collect::<Vec<_>>()
                })
                .collect();
            *layer.props.borrow_mut() = Some(decoded);
        }

        let obj = Object::new();
        let guard = layer.props.borrow();
        let props = guard.as_ref().expect("props must be initialised above");
        for prop in props {
            if let Some(val) = prop_to_js(&prop.values, feature_idx) {
                let _ = Reflect::set(&obj, &JsValue::from_str(&prop.name), &val);
            }
        }
        obj
    }
}

/// Decode a raw MLT tile blob and return an [`MltTile`].
///
/// Geometry and ID columns are decoded eagerly so that `feature_type`,
/// `feature_id`, and `feature_geometry` are always cheap index operations.
/// Property columns are stored in their encoded form and decoded lazily on
/// the first `feature_properties` call for each layer, avoiding work for
/// features that are never rendered or inspected.
///
/// Decoded geometry and IDs are **moved** out of the parsed layer rather than
/// cloned, keeping peak memory and allocation cost as low as possible.
#[wasm_bindgen]
pub fn decode_tile(data: &[u8]) -> Result<MltTile, JsError> {
    let raw_layers = parse_layers(data).map_err(|e| to_js_err(&e))?;
    let mut layers = Vec::with_capacity(raw_layers.len());

    for mut raw_layer in raw_layers {
        // Skip non-Tag01 layers.
        if !matches!(raw_layer, mlt_core::Layer::Tag01(_)) {
            continue;
        }

        // Phase 1 — mutably borrow to decode geometry + IDs and snapshot the
        // encoded property columns as owned values.  The borrow is released
        // at the end of this block so we can consume `raw_layer` below.
        let raw_props: Vec<OwnedProperty> = {
            let layer01 = match &mut raw_layer {
                mlt_core::Layer::Tag01(l) => l,
                mlt_core::Layer::Unknown(_) => unreachable!(),
            };

            layer01
                .decode_geometry_and_id()
                .map_err(|e| to_js_err(&e))?;

            // Snapshot encoded property columns as owned data so they can
            // outlive `raw_layer` and be decoded lazily later.
            layer01.properties.iter().map(|p| p.to_owned()).collect()
        }; // mutable borrow of raw_layer ends here

        // Phase 2 — consume `raw_layer` to MOVE the decoded geometry and IDs
        // directly into `DecodedLayer`, with no intermediate clone.
        let layer01 = match raw_layer {
            mlt_core::Layer::Tag01(l) => l,
            mlt_core::Layer::Unknown(_) => unreachable!(),
        };

        let name = layer01.name.to_string();
        let extent = layer01.extent;

        let geometry = match layer01.geometry {
            mlt_core::v01::Geometry::Decoded(g) => g,
            mlt_core::v01::Geometry::Encoded(_) => {
                return Err(JsError::new("geometry was not decoded"));
            }
        };

        let ids = match layer01.id {
            Id::Decoded(DecodedId(v)) => v,
            Id::None => None,
            Id::Encoded(_) => return Err(JsError::new("id was not decoded")),
        };

        layers.push(DecodedLayer {
            name,
            extent,
            geometry,
            ids,
            raw_props: RefCell::new(raw_props),
            props: RefCell::new(None),
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
        PropValue::SharedDict => None,
    }
}
