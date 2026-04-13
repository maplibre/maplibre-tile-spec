use js_sys::{Array, Float64Array, Object, Reflect};
use wasm_bindgen::prelude::*;

use crate::geometry::LayerGeometry;
use crate::layer::DecodedLayer;
use crate::properties::{build_prop_cache, prop_value_to_js};

/// A fully decoded MLT tile.
///
/// Construct one via [`crate::decode_tile`], then use the index-based accessors
/// to read layer metadata and per-feature data.
///
/// All decoding is done eagerly at construction time via [`TileLayer01`].
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
        self.layers[layer_idx].tile.name.clone()
    }

    /// Extent of layer `layer_idx` in tile coordinates (typically 4096).
    #[must_use]
    pub fn layer_extent(&self, layer_idx: usize) -> u32 {
        self.layers[layer_idx].tile.extent
    }

    /// Number of features in layer `layer_idx`.
    #[must_use]
    pub fn feature_count(&self, layer_idx: usize) -> usize {
        self.layers[layer_idx].tile.features.len()
    }

    // -----------------------------------------------------------------------
    // Bulk typed-array accessors
    // -----------------------------------------------------------------------

    /// MVT geometry types — collapses single and multi into `1`/`2`/`3`.
    #[must_use]
    pub fn layer_types(&self, layer_idx: usize) -> js_sys::Uint8Array {
        self.layers[layer_idx].types_array.clone()
    }

    /// Original MLT geometry types — preserves the single vs multi distinction.
    #[must_use]
    pub fn layer_mlt_types(&self, layer_idx: usize) -> js_sys::Uint8Array {
        self.layers[layer_idx].mlt_types_array.clone()
    }

    /// All feature IDs for layer `layer_idx` as a `Float64Array`.
    ///
    /// One `f64` per feature.  Absent IDs are `NaN`.
    #[must_use]
    pub fn layer_ids(&self, layer_idx: usize) -> Float64Array {
        let features = &self.layers[layer_idx].tile.features;
        let floats: Vec<f64> = features
            .iter()
            .map(|f| {
                #[expect(clippy::cast_precision_loss)]
                f.id.map_or(f64::NAN, |v| v as f64)
            })
            .collect();
        Float64Array::from(floats.as_slice())
    }

    /// All decoded geometry arrays for layer `layer_idx`, in one call.
    #[must_use]
    pub fn layer_geometry(&self, layer_idx: usize) -> LayerGeometry {
        LayerGeometry::from_values(&self.layers[layer_idx].geometry)
    }

    // -----------------------------------------------------------------------
    // Bulk property API
    // -----------------------------------------------------------------------

    /// Column names for layer `layer_idx` as a JS `Array` of strings.
    pub fn layer_property_keys(&self, layer_idx: usize) -> Array {
        let tile = &self.layers[layer_idx].tile;
        build_prop_cache(tile).keys
    }

    /// All property values for layer `layer_idx` as a JS `Array` of columns.
    pub fn layer_properties(&self, layer_idx: usize) -> Array {
        let tile = &self.layers[layer_idx].tile;
        build_prop_cache(tile).columns
    }

    // -----------------------------------------------------------------------
    // Compatibility: per-feature API
    // -----------------------------------------------------------------------

    /// Properties for a single feature as a plain JS object.
    #[must_use]
    pub fn feature_properties(&self, layer_idx: usize, feature_idx: usize) -> Object {
        let tile = &self.layers[layer_idx].tile;
        let obj = Object::new();
        if let Some(feature) = tile.features.get(feature_idx) {
            for (name, val) in tile.property_names.iter().zip(feature.properties.iter()) {
                if let Some(js_val) = prop_value_to_js(val) {
                    let _ = Reflect::set(&obj, &JsValue::from_str(name), &js_val);
                }
            }
        }
        obj
    }
}
