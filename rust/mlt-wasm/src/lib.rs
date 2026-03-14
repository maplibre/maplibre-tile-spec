//! WebAssembly bindings for the `MapLibre` Tile (MLT) format.
//!
//! # Design
//!
//! A single [`tile::MltTile`] struct owns all decoded [`TileLayer01`] data for
//! every layer in the tile.  No per-layer or per-feature WASM objects are
//! created; every accessor takes explicit `(layer_idx, feature_idx)` arguments
//! so the JavaScript side can keep plain numeric indices rather than
//! heap-allocated wrapper objects.
//!
//! ## Geometry
//!
//! [`tile::MltTile::layer_geometry`] returns a [`geometry::LayerGeometry`]
//! whose typed-array getters expose the raw offset and vertex buffers.
//! JS walks these directly — zero WASM boundary crossings per feature.
//!
//! ## IDs
//!
//! [`tile::MltTile::layer_ids`] returns a `Float64Array` — one `f64` per
//! feature.  Absent IDs are `NaN` (≡ `undefined` after the JS wrapper checks
//! `isNaN`).  IDs above `Number.MAX_SAFE_INTEGER` lose precision.
//!
//! ## Properties
//!
//! [`tile::MltTile::layer_property_keys`] and [`tile::MltTile::layer_properties`]
//! expose all property columns as typed arrays built once per layer.  JS reads
//! any feature's property with a single array index — zero WASM calls during
//! traversal.

mod geometry;
mod layer;
mod properties;
mod tile;

use std::cell::RefCell;
use std::f64;

use ids::IdState;
use js_sys::{Float64Array, Uint8Array};
use layer::DecodedLayer;
use mlt_core::v01::{Geometry, GeometryType, Id, ParsedProperty};
use mlt_core::{EncDec, Layer, MltError, parse_layers};
use tile::MltTile;
use wasm_bindgen::prelude::*;

/// Decode a raw MLT tile blob and return an [`MltTile`].
///
/// All geometry, IDs and properties are decoded eagerly into row-oriented
/// [`TileLayer01`] values.
#[wasm_bindgen]
pub fn decode_tile(data: &[u8]) -> Result<MltTile, JsError> {
    let raw_layers = parse_layers(data).map_err(|e| to_js_err(&e))?;
    let mut layers = Vec::with_capacity(raw_layers.len());

    for raw_layer in raw_layers {
        // Skip non-Tag01 layers.
        let Layer::Tag01(layer01) = raw_layer else {
            continue;
        };

        let tile = TileLayer01::try_from(layer01).map_err(|e| to_js_err(&e))?;

        // Build types_array from the decoded geo_types::Geometry variants.
        let types_bytes: Vec<u8> = tile
            .features
            .iter()
            .map(|f| match &f.geometry {
                geo_types::Geometry::Point(_) | geo_types::Geometry::MultiPoint(_) => 1,
                geo_types::Geometry::Line(_)
                | geo_types::Geometry::LineString(_)
                | geo_types::Geometry::MultiLineString(_) => 2,
                geo_types::Geometry::Polygon(_) | geo_types::Geometry::MultiPolygon(_) => 3,
                _ => 0,
            })
            .collect();
        let types_array = Uint8Array::from(types_bytes.as_slice());

        layers.push(DecodedLayer { tile, types_array });
    }

    Ok(MltTile { layers })
}

pub(crate) fn to_js_err(e: &MltError) -> JsError {
    JsError::new(&e.to_string())
}
