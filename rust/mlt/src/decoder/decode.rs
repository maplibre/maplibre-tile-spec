use bytes::{Buf, Bytes};
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

    while tile.has_remaining() {
        let ids: Vec<i64> = vec![];
        let geometries: Vec<Geometry> = vec![];

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
                let id_data_stream_metadata = StreamMetadata::decode(tile)?;
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
            "../../test/expected/omt/2_2_2.mlt",
        )
        .unwrap();
        let mut data = Bytes::from(raw);
        let metadata = read_metadata(Path::new("../../test/expected/omt/2_2_2.mlt.meta.pbf"));
        let tile = decode(&mut data, &metadata.expect("Failed to read metadata")).unwrap();

        todo!("Implement test_decode");
    }
}
