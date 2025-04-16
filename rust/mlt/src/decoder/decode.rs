use crate::decoder::varint;

use crate::data::MapLibreTile;
use crate::metadata::proto_tileset::TileSetMetadata;
use crate::{MltError, MltResult};
use bytes::Bytes;
use fastpfor::rust::IncrementCursor;
use geo_types::Geometry;
use std::io::Cursor;

const ID_COLUMN_NAME: &str = "id";
const GEOMETRY_COLUMN_NAME: &str = "geometry";

// Impl in the future
#[expect(unused_variables)]
pub fn decode(tile: &Bytes, tile_metadata: &TileSetMetadata) -> MltResult<MapLibreTile> {
    let mut offset = Cursor::new(0);

    while offset.position() < tile.len() as u64 {
        println!("Offset: {}", offset.position());
        let ids: Vec<i64> = vec![];
        let geometries: Vec<Geometry> = vec![];
        // Not sure the best way to cover this right now
        // var properties = new HashMap<String, List<Object>>();

        let version = tile
            .get(offset.position() as usize)
            .ok_or_else(|| MltError::DecodeError("Failed to read version".to_string()))?;
        offset.increment();
        let infos = varint::decode(tile, 5, &mut offset);
        let feature_table_id = infos
            .get(0)
            .ok_or_else(|| MltError::DecodeError("Failed to read feature table id".to_string()))?;

        let feature_table_body_size = infos.get(1).ok_or_else(|| {
            MltError::DecodeError("Failed to read feature table body size".to_string())
        })?;
        let tile_extent = infos
            .get(2)
            .ok_or_else(|| MltError::DecodeError("Failed to read tile extent".to_string()))?;
        let max_tile_extent = infos
            .get(3)
            .ok_or_else(|| MltError::DecodeError("Failed to read max tile extent".to_string()))?;
        let num_features = infos.get(4).ok_or_else(|| {
            MltError::DecodeError("Failed to read number of features".to_string())
        })?;

        let metadata = tile_metadata
            .feature_tables
            .get(*feature_table_id as usize)
            .ok_or_else(|| {
                MltError::DecodeError(format!(
                    "Failed to read feature table metadata for id {}",
                    feature_table_id
                ))
            })?;

        for col_metadata in metadata.columns.iter() {
            let num_streams_vec = varint::decode(tile, 1, &mut offset);
            let num_streams = num_streams_vec.get(0).ok_or_else(|| {
                MltError::DecodeError("Failed to retrieve num_streams".to_string())
            })?;
            // Column "id" ignore as currently no-op:
            // https://github.com/maplibre/maplibre-tile-spec/blob/0a67590c338d6e93fd62043293e425be0b8cf85e/java/src/main/java/com/mlt/decoder/MltDecoder.java#L65
        }
    }

    todo!("Implement decode");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::tileset::read_metadata;
    use std::fs;
    use std::path::Path;

    #[test]
    #[expect(unused_variables)]
    fn test_decode() {
        let raw = fs::read("../../test/expected/omt/2_2_2.mlt").unwrap();
        let data = Bytes::from(raw);
        // Get metadata as a Path
        let metadata = read_metadata(Path::new("../../test/expected/omt/2_2_2.mlt.meta.pbf"));
        let tile = decode(&data, &metadata.expect("Failed to read metadata")).unwrap();
        // let mlt = MapLibreTile::decode(&data, &metadata).unwrap();
        //
        // assert_eq!(mlt.layers.len(), 1);
        // assert_eq!(mlt.layers[0].name, "layer_name");

        todo!("Implement test_decode");
    }
}
