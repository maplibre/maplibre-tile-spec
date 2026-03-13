use js_sys::{Array, Float64Array, Int32Array, Object, Reflect, Uint8Array, Uint32Array};
use mlt_core::v01::{ParsedProperty, StagedGeometry, StagedProperty};
use wasm_bindgen::prelude::*;

use crate::geometry::LayerGeometry;
use crate::ids::IdState;
use crate::layer::DecodedLayer;
use crate::properties::{build_prop_cache, prop_to_js};

/// A fully decoded MLT tile.
///
/// Construct one via [`crate::decode_tile`], then use the index-based accessors
/// to read layer metadata and per-feature data.
///
/// Geometry types and the `types_array` are decoded eagerly in `decode_tile`.
/// Full geometry, IDs, and property columns are decoded lazily on first access
/// and then cached; repeated calls for the same layer are cheap handle clones.
#[wasm_bindgen]
pub struct MltTile {
    pub(crate) layers: Vec<DecodedLayer>,
}

#[wasm_bindgen]
impl MltTile {
    // -----------------------------------------------------------------------
    // Layer metadata
    // -----------------------------------------------------------------------

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
        self.layers[layer_idx].types_array.length() as usize
    }

    // -----------------------------------------------------------------------
    // Bulk typed-array accessors
    // -----------------------------------------------------------------------

    /// All geometry types for every feature in `layer_idx` as a `Uint8Array`.
    ///
    /// One byte per feature: `1` = Point, `2` = `LineString`, `3` = Polygon,
    /// `0` = Unknown.  Pre-built during `decode_tile` — this call is a cheap
    /// handle clone with no allocation.
    #[must_use]
    pub fn layer_types(&self, layer_idx: usize) -> Uint8Array {
        self.layers[layer_idx].types_array.clone()
    }

    /// All feature IDs for layer `layer_idx` as a `Float64Array`.
    ///
    /// One `f64` per feature.  Absent IDs are `NaN`.  Decoded and cached on
    /// first call; subsequent calls are handle clones.
    pub fn layer_ids(&self, layer_idx: usize) -> Result<Float64Array, JsError> {
        self.ensure_ids_decoded(layer_idx)?;
        let guard = self.layers[layer_idx].ids.borrow();
        Ok(match &*guard {
            IdState::Ready(arr) => arr.clone(),
            _ => Float64Array::new_with_length(0),
        })
    }

    /// All decoded geometry arrays for layer `layer_idx`, in one call.
    ///
    /// Returns a [`LayerGeometry`] whose typed-array getters let JS implement
    /// `loadGeometry(i)` with zero WASM boundary crossings per feature.
    /// Built and cached on first call; subsequent calls are handle clones.
    pub fn layer_geometry(&self, layer_idx: usize) -> Result<LayerGeometry, JsError> {
        self.ensure_geometry_decoded(layer_idx)?;

        let layer = &self.layers[layer_idx];

        // Fast path: typed arrays already built — return handle clones.
        {
            let cache = layer.geometry_cache.borrow();
            if let Some(c) = &*cache {
                return Ok(LayerGeometry {
                    geometry_offsets: c.geometry_offsets.clone(),
                    part_offsets: c.part_offsets.clone(),
                    ring_offsets: c.ring_offsets.clone(),
                    vertices: c.vertices.clone(),
                });
            }
        }

        // Slow path: build the typed arrays once from the decoded geometry.
        let guard = layer.geometry.borrow();
        let StagedGeometry::Decoded(d) = &*guard else {
            unreachable!("geometry should be decoded here");
        };

        let geometry_offsets = d
            .geometry_offsets
            .as_deref()
            .map_or_else(|| Uint32Array::new_with_length(0), Uint32Array::from);

        let part_offsets = d
            .part_offsets
            .as_deref()
            .map_or_else(|| Uint32Array::new_with_length(0), Uint32Array::from);

        let ring_offsets = d
            .ring_offsets
            .as_deref()
            .map_or_else(|| Uint32Array::new_with_length(0), Uint32Array::from);

        let vertices = d
            .vertices
            .as_deref()
            .map_or_else(|| Int32Array::new_with_length(0), Int32Array::from);

        drop(guard);

        let cached = LayerGeometry {
            geometry_offsets,
            part_offsets,
            ring_offsets,
            vertices,
        };
        let ret = LayerGeometry {
            geometry_offsets: cached.geometry_offsets.clone(),
            part_offsets: cached.part_offsets.clone(),
            ring_offsets: cached.ring_offsets.clone(),
            vertices: cached.vertices.clone(),
        };
        *layer.geometry_cache.borrow_mut() = Some(cached);

        Ok(ret)
    }

    // -----------------------------------------------------------------------
    // Bulk property API
    // -----------------------------------------------------------------------

    /// Column names for layer `layer_idx` as a JS `Array` of strings.
    ///
    /// One entry per logical column; `SharedDict` columns expand to one entry
    /// per sub-item (sub-item suffix appended to the parent column name).
    ///
    /// Parallel to [`layer_properties`]: `keys[k]` is the name for `columns[k]`.
    /// Built once and cached — subsequent calls are handle clones.
    pub fn layer_property_keys(&self, layer_idx: usize) -> Array {
        self.ensure_props_cached(layer_idx);
        self.layers[layer_idx]
            .prop_cache
            .borrow()
            .as_ref()
            .unwrap()
            .keys
            .clone()
    }

    /// All property values for layer `layer_idx` as a JS `Array` of columns.
    ///
    /// Parallel to [`layer_property_keys`]: each element is a typed array or
    /// plain `Array` of length `feature_count`.  Index `i` gives the value for
    /// feature `i`; absent values are `NaN` (numeric columns) or `undefined`
    /// (bool / string columns).
    ///
    /// JS can read any feature's property with a single array index
    /// (`columns[col][featureIdx]`) — zero WASM calls during traversal.
    ///
    /// Built once and cached — subsequent calls are handle clones.
    pub fn layer_properties(&self, layer_idx: usize) -> Array {
        self.ensure_props_cached(layer_idx);
        self.layers[layer_idx]
            .prop_cache
            .borrow()
            .as_ref()
            .unwrap()
            .columns
            .clone()
    }

    // -----------------------------------------------------------------------
    // Compatibility: per-feature API
    // -----------------------------------------------------------------------

    /// Properties for a single feature as a plain JS object.
    ///
    /// Kept for compatibility with the `VectorTileFeatureLike` interface.
    /// For high-throughput traversal prefer [`layer_properties`] +
    /// [`layer_property_keys`], which eliminate all per-feature WASM calls.
    #[must_use]
    pub fn feature_properties(&self, layer_idx: usize, feature_idx: usize) -> Object {
        // Ensure columns are decoded and cached (ignoring error — returns empty
        // object on failure rather than panicking across the WASM boundary).
        self.ensure_props_cached(layer_idx);

        let layer = &self.layers[layer_idx];
        let obj = Object::new();
        let cache = layer.prop_cache.borrow();
        let pc = cache.as_ref().unwrap();
        let guard = layer.props.borrow();
        let mut key_idx = 0usize;

        for p in &*guard {
            if let StagedProperty::Decoded(prop) = p {
                if let ParsedProperty::SharedDict(shared_dict) = prop {
                    for item in &shared_dict.items {
                        if let Some(val) = item.get(shared_dict, feature_idx) {
                            let _ = Reflect::set(
                                &obj,
                                &pc.keys
                                    .get(u32::try_from(key_idx).expect("key index fits in u32")),
                                &JsValue::from_str(val),
                            );
                        }
                        key_idx += 1;
                    }
                } else {
                    if let Some(val) = prop_to_js(prop, feature_idx) {
                        let _ = Reflect::set(
                            &obj,
                            &pc.keys
                                .get(u32::try_from(key_idx).expect("key index fits in u32")),
                            &val,
                        );
                    }
                    key_idx += 1;
                }
            }
        }

        obj
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    fn ensure_geometry_decoded(&self, layer_idx: usize) -> Result<(), JsError> {
        let layer = &self.layers[layer_idx];
        let mut geom = layer.geometry.borrow_mut();
        if matches!(&*geom, StagedGeometry::Encoded(_)) {
            let owned = std::mem::replace(
                &mut *geom,
                StagedGeometry::Decoded(mlt_core::v01::ParsedGeometry::default()),
            );
            let StagedGeometry::Encoded(encoded) = owned else {
                unreachable!()
            };
            let decoded = mlt_core::v01::ParsedGeometry::try_from(encoded)
                .map_err(|e| crate::to_js_err(&e))?;
            *geom = StagedGeometry::Decoded(decoded);
        }
        Ok(())
    }

    fn ensure_ids_decoded(&self, layer_idx: usize) -> Result<(), JsError> {
        let layer = &self.layers[layer_idx];
        let mut ids = layer.ids.borrow_mut();
        if matches!(&*ids, IdState::Encoded(_)) {
            let prev = std::mem::replace(&mut *ids, IdState::Absent);
            let IdState::Encoded(encoded) = prev else {
                unreachable!()
            };
            let decoded =
                mlt_core::v01::ParsedId::try_from(encoded).map_err(|e| crate::to_js_err(&e))?;

            let floats: Vec<f64> = decoded
                .values()
                .iter()
                .copied()
                .map(|f: Option<u64>| {
                    #[expect(clippy::cast_precision_loss)]
                    f.map_or(f64::NAN, |v| v as f64)
                })
                .collect();
            let arr = Float64Array::from(floats.as_slice());
            *ids = IdState::Ready(arr);
        }
        Ok(())
    }

    fn ensure_props_cached(&self, layer_idx: usize) {
        let layer = &self.layers[layer_idx];

        // Build the bulk cache if not already present.
        if layer.prop_cache.borrow().is_none() {
            let n = layer.types_array.length();
            let guard = layer.props.borrow();
            let cache = build_prop_cache(&guard, n);
            drop(guard);
            *layer.prop_cache.borrow_mut() = Some(cache);
        }
    }
}
