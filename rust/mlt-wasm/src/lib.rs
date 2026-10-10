//! WebAssembly bindings for the `MapLibre` Tile (MLT) format.
//!
//! # Design
//!
//! `decodeTileColumns` hands every layer to JS as typed arrays in one call, read
//! straight off `mlt-core`'s decoded columns. No per-layer or per-feature WASM objects are
//! kept alive, so nothing needs freeing, and JS walks the arrays with zero boundary crossings.
//!
//! ## Geometry
//!
//! The vertex buffer is passed on as decoded: `(x, y)` pairs, or `(x, y, z)` triples when the
//! layer has z coordinates, as the v2 wire format interleaves them.
//!
//! ## Ids and properties
//!
//! Each column is one value per feature with a presence bitmap beside it, rather than a
//! sentinel. 64-bit integers come out as `f64`, so values above `Number.MAX_SAFE_INTEGER`
//! lose precision.
//!
//! ## Annotate
//!
//! `annotateTile` is the other entry point: it walks a tile into an `AnnotatedTile`
//! handle over its regions, for tooling that shows the wire format rather than the
//! decoded tile.

mod annotate;
mod columns;
#[cfg(feature = "coverage")]
mod coverage;

use mlt_core::geojson::FeatureCollection;
use mlt_core::{Decoder, MltError, Parser};
use wasm_bindgen::prelude::*;

/// Decode a raw MLT tile blob into `GeoJSON`, as the serialized text.
///
/// Every feature carries `_layer` and `_extent`, and a v2 tile's vertex-scoped columns ride
/// along as `m:`-prefixed arrays, one value per vertex.
///
/// Text rather than a `JsValue`: building the object graph across the boundary costs a
/// crossing per value, where one string plus `JSON.parse` is a single copy.
#[wasm_bindgen(js_name = "tileGeoJson")]
pub fn tile_geojson(data: &[u8]) -> Result<String, JsError> {
    let mut parser = Parser::default();
    let raw_layers = parser.parse_layers(data).map_err(|e| to_js_err(&e))?;
    let mut dec = Decoder::default();
    let mut layers = Vec::with_capacity(raw_layers.len());
    for raw_layer in raw_layers {
        layers.push(raw_layer.decode_all(&mut dec).map_err(|e| to_js_err(&e))?);
    }
    let collection = FeatureCollection::from_layers(layers).map_err(|e| to_js_err(&e))?;
    Ok(serde_json::to_string(&collection)?)
}

pub(crate) fn to_js_err(e: &MltError) -> JsError {
    JsError::new(&e.to_string())
}
