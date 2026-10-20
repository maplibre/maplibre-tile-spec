//! Convert MVT data to/from [`FeatureCollection`](crate::geojson::FeatureCollection)
//! / [`TileLayer`](crate::TileLayer).

mod decode;
mod encode;

pub(crate) use encode::write_mvt_layer;

pub use crate::convert::mvt::decode::{mvt_to_feature_collection, mvt_to_tile_layers};
pub use crate::convert::mvt::encode::tile_layers_to_mvt;
