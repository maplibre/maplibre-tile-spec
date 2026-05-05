//! Convert MVT data to/from [`FeatureCollection`](crate::geojson::FeatureCollection)
//! / [`TileLayer`](crate::TileLayer).

mod decode;
mod encode;
mod vector_tile;

pub use decode::{mvt_to_feature_collection, mvt_to_tile_layers};
pub use encode::tile_layers_to_mvt;
