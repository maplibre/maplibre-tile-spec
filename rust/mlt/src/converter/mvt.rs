use geozero::mvt::{tile::Layer, Message, Tile};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnMapping {
    pub mvt_property_prefix: String,
    pub mvt_delimiter_sign: String,
    pub use_shared_dictionary_encoding: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapboxVectorTile {
    pub layers: Vec<Layer>,
}

pub fn decode_mvt(mvt_tile: &[u8]) -> MapboxVectorTile {
    let mut tile = Tile::decode(mvt_tile).expect("Failed to decode MVT");
    for layer in &mut tile.layers {
        if layer.extent() <= 0 {
            layer.extent.get_or_insert(0);
        }
    }
    MapboxVectorTile {
        layers: tile.layers,
    }
}
