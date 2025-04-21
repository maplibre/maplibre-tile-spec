use std::io::Cursor;

use bytes::{Buf, Bytes};
use fastpfor::rust::IncrementCursor;
use geo_types::Geometry;

use crate::data::MapLibreTile;
use crate::decoder::helpers::decode_boolean_rle;
use crate::decoder::varint;
use crate::metadata::proto_tileset::TileSetMetadata;
use crate::metadata::stream::StreamMetadata;
use crate::{MltError, MltResult};

const ID_COLUMN_NAME: &str = "id";
const GEOMETRY_COLUMN_NAME: &str = "geometry";

#[expect(unused_variables)]
pub fn decode(tile: &mut Bytes, tile_metadata: &TileSetMetadata) -> MltResult<MapLibreTile> {
    // let mut offset = Cursor::new(0);

    while tile.has_remaining() {
        let ids: Vec<i64> = vec![];
        let geometries: Vec<Geometry> = vec![];
        // Not sure the best way to cover this right now
        // var properties = new HashMap<String, List<Object>>();

        let version = tile.get_u8();

        let infos = varint::decode(tile, 5);
        let feature_table_id = infos
            .first()
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
            let num_streams_vec = varint::decode(tile, 1);
            let num_streams = num_streams_vec.get(0).ok_or_else(|| {
                MltError::DecodeError("Failed to retrieve num_streams".to_string())
            })?;
            if col_metadata.name == ID_COLUMN_NAME {
                if *num_streams == 2 {
                    let present_stream_metadata = StreamMetadata::decode(tile)?;
                    let present_stream = decode_boolean_rle(
                        tile,
                        *num_features,
                        present_stream_metadata.byte_length,
                    )?;
                }
                // Below is where the error occurs
                let id_data_stream_metadata = StreamMetadata::decode(tile)?;
                panic!("id_data_stream_metadata: {:?}", id_data_stream_metadata);
            }
        }
    }

    todo!("Implement decode");
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::*;
    use crate::metadata::tileset::read_metadata;

    #[test]
    #[expect(unused_variables)]
    fn test_decode() {
        let raw = fs::read(
            "/Users/employeeadmin/repositories/maplibre-tile-spec/test/expected/omt/2_2_2.mlt",
        )
        .unwrap();
        // Print out the first 10 Bytes
        let mut data = Bytes::from(raw);
        println!("data: {:?}", &data[0..10]);
        // Get metadata as a Path
        let metadata = read_metadata(Path::new("../../test/expected/omt/2_2_2.mlt.meta.pbf"));
        let tile = decode(&mut data, &metadata.expect("Failed to read metadata")).unwrap();
        // let mlt = MapLibreTile::decode(&data, &metadata).unwrap();
        //
        // assert_eq!(mlt.layers.len(), 1);
        // assert_eq!(mlt.layers[0].name, "layer_name");

        todo!("Implement test_decode");
    }
}
