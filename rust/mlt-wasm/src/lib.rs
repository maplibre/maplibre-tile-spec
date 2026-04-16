//! WebAssembly bindings for the `MapLibre` Tile (MLT) format.
//!
//! # Design
//!
//! A single `MltTile` struct owns all decoded [`mlt_core::TileLayer01`] data for
//! every layer in the tile.  No per-layer or per-feature WASM objects are
//! created; every accessor takes explicit `(layer_idx, feature_idx)` arguments
//! so the JavaScript side can keep plain numeric indices rather than
//! heap-allocated wrapper objects.
//!
//! ## Geometry
//!
//! `MltTile::layer_geometry` returns a `LayerGeometry`
//! whose typed-array getters expose the raw offset and vertex buffers.
//! JS walks these directly — zero WASM boundary crossings per feature.
//!
//! ## IDs
//!
//! `MltTile::layer_ids` returns a `Float64Array` — one `f64` per
//! feature.  Absent IDs are `NaN` (≡ `undefined` after the JS wrapper checks
//! `isNaN`).  IDs above `Number.MAX_SAFE_INTEGER` lose precision.
//!
//! ## Properties
//!
//! `MltTile::layer_property_keys` and `MltTile::layer_properties`
//! expose all property columns as typed arrays built once per layer.  JS reads
//! any feature's property with a single array index — zero WASM calls during
//! traversal.

mod geometry;
mod layer;
mod properties;
mod tile;

use js_sys::Uint8Array;
use layer::DecodedLayer;
use mlt_core::{Decoder, GeometryType, MltError, Parser};
use tile::MltTile;
use wasm_bindgen::prelude::*;

/// Decode a raw MLT tile blob and return an `MltTile`.
///
/// All geometry, IDs and properties are decoded eagerly into row-oriented
/// [`mlt_core::TileLayer01`] values.
#[wasm_bindgen]
pub fn decode_tile(data: &[u8]) -> Result<MltTile, JsError> {
    let mut parser = Parser::default();
    let raw_layers = parser.parse_layers(data).map_err(|e| to_js_err(&e))?;
    let mut dec = Decoder::default();
    let mut layers = Vec::with_capacity(raw_layers.len());

    for raw_layer in raw_layers {
        // Skip non-Tag01 layers.
        let mlt_core::Layer::Tag01(layer01) = raw_layer else {
            continue;
        };

        // Decode geometry to columnar GeometryValues first (for WASM typed-array output),
        // then decode the whole layer to TileLayer01 for properties/IDs.
        // We need to re-parse the same layer, so clone the raw geometry before consuming.
        let parsed_geometry = layer01
            .geometry
            .clone()
            .into_parsed(&mut dec)
            .map_err(|e| to_js_err(&e))?;

        let (types_bytes, mlt_types_bytes): (Vec<u8>, Vec<u8>) = parsed_geometry
            .vector_types()
            .iter()
            .map(|t| {
                let mvt = match t {
                    GeometryType::Point | GeometryType::MultiPoint => 1,
                    GeometryType::LineString | GeometryType::MultiLineString => 2,
                    GeometryType::Polygon | GeometryType::MultiPolygon => 3,
                    #[allow(unreachable_patterns)]
                    _ => 0,
                };
                (mvt, *t as u8)
            })
            .unzip();
        let types_array = Uint8Array::from(types_bytes.as_slice());
        let mlt_types_array = Uint8Array::from(mlt_types_bytes.as_slice());

        let tile = layer01.into_tile(&mut dec).map_err(|e| to_js_err(&e))?;

        layers.push(DecodedLayer {
            tile,
            types_array,
            mlt_types_array,
            geometry: parsed_geometry,
        });
    }

    Ok(MltTile { layers })
}

pub(crate) fn to_js_err(e: &MltError) -> JsError {
    JsError::new(&e.to_string())
}
