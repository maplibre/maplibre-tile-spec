//! WebAssembly bindings for the `MapLibre` Tile (MLT) format.
//!
//! # Design
//!
//! A single [`tile::MltTile`] struct owns all decoded columnar data for every
//! layer in the tile.  No per-layer or per-feature WASM objects are created;
//! every accessor takes explicit `(layer_idx, feature_idx)` arguments so the
//! JavaScript side can keep plain numeric indices rather than heap-allocated
//! wrapper objects.
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
mod ids;
mod layer;
mod properties;
mod tile;

use std::cell::RefCell;
use std::f64;

use ids::IdState;
use js_sys::Uint8Array;
use layer::DecodedLayer;
use mlt_core::v01::{Geometry, GeometryType, Id};
use mlt_core::{MltError, parse_layers};
use tile::MltTile;
use wasm_bindgen::prelude::*;

/// Decode a raw MLT tile blob and return an [`MltTile`].
///
/// Geometry types and the per-layer `types_array` are decoded eagerly.
/// Full geometry, IDs, and property columns are decoded lazily on first access
/// and then cached inside the returned [`MltTile`].
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

        // Decode geometry types eagerly — cheap, needed for feature_count and
        // layer_types on every tile traversal.
        let vector_types: Vec<GeometryType> = match &layer01.geometry {
            Geometry::Encoded(encoded) => encoded
                .meta
                .clone()
                .decode_bits_u32()
                .map_err(|e| to_js_err(&e))?
                .decode_u32()
                .map_err(|e| to_js_err(&e))?
                .into_iter()
                .map::<Result<GeometryType, JsError>, _>(|v| {
                    u8::try_from(v)
                        .map_err(|_| JsError::new("invalid geometry type"))?
                        .try_into()
                        .map_err(|_| JsError::new("invalid geometry type"))
                })
                .collect::<Result<Vec<_>, _>>()?,
            Geometry::Decoded(decoded) => decoded.vector_types.clone(),
        };

        // Build types_array once up front; layer_types() returns a handle clone.
        let types_bytes: Vec<u8> = vector_types
            .iter()
            .map(|t| match t {
                GeometryType::Point | GeometryType::MultiPoint => 1,
                GeometryType::LineString | GeometryType::MultiLineString => 2,
                GeometryType::Polygon | GeometryType::MultiPolygon => 3,
                #[allow(unreachable_patterns)]
                _ => 0,
            })
            .collect();
        let types_array = Uint8Array::from(types_bytes.as_slice());

        let geometry = RefCell::new(layer01.geometry.to_owned());

        let ids = RefCell::new(match &layer01.id {
            None => IdState::Absent,
            Some(Id::Encoded(encoded)) => IdState::Encoded(encoded.to_owned()),
            Some(Id::Decoded(decoded)) => {
                use js_sys::Float64Array;
                let floats: Vec<f64> = decoded
                    .values()
                    .iter()
                    .copied()
                    .map(|f: Option<u64>| {
                        #[expect(clippy::cast_precision_loss)]
                        f.map_or(f64::NAN, |f| f as f64)
                    })
                    .collect();
                IdState::Ready(Float64Array::from(floats.as_slice()))
            }
        });

        let props = RefCell::new(
            layer01
                .properties
                .into_iter()
                .map(|p| p.decode().map(|d| d.to_owned()))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| to_js_err(&e))?,
        );

        layers.push(DecodedLayer {
            name,
            extent,
            types_array,
            geometry,
            geometry_cache: RefCell::new(None),
            ids,
            props,
            prop_cache: RefCell::new(None),
        });
    }

    Ok(MltTile { layers })
}

pub(crate) fn to_js_err(e: &MltError) -> JsError {
    JsError::new(&e.to_string())
}
