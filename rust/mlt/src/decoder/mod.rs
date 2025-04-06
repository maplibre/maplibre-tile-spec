mod utils;

use crate::data::MapLibreTile;
use crate::metadata::{proto_tileset::TileSetMetadata, tileset::read_metadata};
use crate::{MltError, MltResult};
use fastpfor::rust::IncrementCursor;
use geo_types::Geometry;
use std::io::Cursor;
use std::path::Path;

// Impl in the future
pub fn decode(data: &[u8], tile_metadata: &TileSetMetadata) -> MltResult<MapLibreTile> {
    let mut offset = Cursor::new(0);

    while offset.position() < data.len() as u64 {
        let ids: Vec<i64> = vec![];
        let geometries: Vec<Geometry> = vec![];
        // Not sure the best way to cover this right now
        // var properties = new HashMap<String, List<Object>>();

        let version = data
            .get(offset.position() as usize)
            .ok_or_else(|| MltError::DecodeError("Failed to read version".to_string()))?;
        offset.increment();
        let infos = utils::decode_varint(data, &mut offset, 5);
        let feature_table_id = infos.get(0).ok_or_else(|| {
            MltError::DecodeError("Failed to read feature table id".to_string())
        })?;
        let feature_table_body_size = infos.get(1).ok_or_else(|| {
            MltError::DecodeError("Failed to read feature table body size".to_string())
        })?;
        let tile_extent = infos.get(2).ok_or_else(|| {
            MltError::DecodeError("Failed to read tile extent".to_string())
        })?;
        let max_tile_extent = infos.get(3).ok_or_else(|| {
            MltError::DecodeError("Failed to read max tile extent".to_string())
        })?;
        let num_features = infos.get(4).ok_or_else(|| {
            MltError::DecodeError("Failed to read number of features".to_string())
        })?;

    }

    Ok(MapLibreTile { layers: vec![] })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_decode() {
        let data = fs::read("../../test/expected/omt/2_2_2.mlt").unwrap();
        // Get metadata as a Path
        let metadata = read_metadata(Path::new("../../test/expected/omt/2_2_2.mlt.meta.pbf"));
        let tile = decode(&data, &metadata.unwrap()).unwrap();
        // let mlt = MapLibreTile::decode(&data, &metadata).unwrap();
        //
        // assert_eq!(mlt.layers.len(), 1);
        // assert_eq!(mlt.layers[0].name, "layer_name");
    }
}
