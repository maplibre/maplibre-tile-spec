use geozero::ToGeo;
use geozero::mvt::{Message, Tile};

use crate::data::{Feature, Layer, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnMapping {
    pub mvt_property_prefix: String,
    pub mvt_delimiter_sign: String,
    pub use_shared_dictionary_encoding: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapVectorTile {
    pub layers: Vec<Layer>,
}

#[must_use]
pub fn decode_mvt(mvt_tile: &[u8]) -> MapVectorTile {
    let mut tile = Tile::decode(mvt_tile).expect("Failed to decode MVT");
    let mut layers: Vec<Layer> = Vec::new();
    for layer in &mut tile.layers {
        let mut tile_extent = Some(0);
        let mut features: Vec<Feature> = Vec::new();
        for feature in &mut layer.features {
            let feature = Feature {
                id: feature.id.unwrap_or(0) as i64,
                geometry: feature.to_geo().expect("Failed to convert feature to geo"),
                properties: layer
                    .keys
                    .iter()
                    .zip(layer.values.iter())
                    .map(|(k, v)| (k.replace('_', ":"), Value::from(v.clone())))
                    .collect(),
            };
            features.push(feature);
            tile_extent = tile_extent.max(layer.extent);
        }
        layers.push(Layer {
            name: layer.name.clone(),
            features,
            tile_extent: tile_extent.unwrap_or(4096) as i32, // default to 4096?
        });
    }
    MapVectorTile { layers }
}
