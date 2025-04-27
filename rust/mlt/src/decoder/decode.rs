use std::collections::HashMap;

use bitvec::prelude::*;
use bytes::{Buf, Bytes};
use geo_types::Geometry;
use zigzag::ZigZag;

use crate::data::MapLibreTile;
use crate::decoder::helpers::decode_boolean_rle;
use crate::decoder::varint;
use crate::encoder::geometry::GeometryScaling;
use crate::metadata::proto_tileset::TileSetMetadata;
use crate::metadata::stream::StreamMetadata;
use crate::{MltError, MltResult};

const ID_COLUMN_NAME: &str = "id";
const GEOMETRY_COLUMN_NAME: &str = "geometry";

pub struct Config {
    pub feature_table_decoding: Option<HashMap<String, String>>,
    pub geometry_scaling: Option<GeometryScaling>,
    pub id_within_max_safe_integer: bool,
}

#[expect(unused_variables)]
pub fn decode(
    tile: &mut Bytes,
    tile_metadata: &TileSetMetadata,
    config: Option<Config>,
) -> MltResult<MapLibreTile> {
    while tile.has_remaining() {
        let ids: Vec<i64> = vec![];
        let geometries: Vec<Geometry> = vec![];

        let infos = varint::decode(tile, 5);
        let version = tile.get_u8();

        let feature_table_id = infos
            .first()
            .ok_or_else(|| MltError::DecodeError("Failed to read feature table id".to_string()))?;
        let feature_table_body_size = infos.get(1).ok_or_else(|| {
            MltError::DecodeError("Failed to read feature table body size".to_string())
        })?;
        let feature_table_metadata = tile_metadata
            .feature_tables
            .get(*feature_table_id as usize)
            .ok_or_else(|| {
                MltError::DecodeError(format!(
                    "Failed to read feature table metadata for id {}",
                    feature_table_id
                ))
            })?;

        let property_column_names: Option<&String> = config.as_ref().and_then(|cfg| {
            cfg.feature_table_decoding
                .as_ref()
                .and_then(|ftd| ftd.get(&feature_table_metadata.name))
        });

        if property_column_names.is_none() {
            tile.advance(*feature_table_body_size as usize);
            continue;
        }

        let extent = infos
            .get(2)
            .ok_or_else(|| MltError::DecodeError("Failed to read tile extent".to_string()))?;

        let max_tile_extent: i32 =
            ZigZag::decode(*infos.get(3).ok_or_else(|| {
                MltError::DecodeError("Failed to read max tile extent".to_string())
            })?);

        let num_features = infos.get(4).ok_or_else(|| {
            MltError::DecodeError("Failed to read number of features".to_string())
        })?;

        for col_metadata in feature_table_metadata.columns.iter() {
            let num_streams_vec = varint::decode(tile, 1);
            let num_streams = num_streams_vec.first().ok_or_else(|| {
                MltError::DecodeError("Failed to retrieve num_streams".to_string())
            })?;
            if col_metadata.name == ID_COLUMN_NAME {
                let mut nullability_buffer = BitVec::<u8, Lsb0>::EMPTY;
                if *num_streams == 2 {
                    let present_stream_metadata = StreamMetadata::decode(tile)?;
                    let values =
                        decode_boolean_rle(tile, present_stream_metadata.num_values as usize)?;
                    nullability_buffer = BitVec::<u8, Lsb0>::with_capacity(*num_features as usize);
                    nullability_buffer.extend(values.iter().copied());
                }

                // Dummy
                nullability_buffer.resize(1, false);
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
        let raw = fs::read("../../ts/test/data/omt/unoptimized/mlt/plain/0_0_0.mlt")
            .expect("Failed to read file");
        let mut data = Bytes::from(raw);
        let metadata = read_metadata(Path::new(
            "../../ts/test/data/omt/unoptimized/mlt/plain/tileset.pbf",
        ))
        .expect("Failed to read metadata");
        let tile = decode(&mut data, &metadata, None).expect("Failed to read metadata");
    }
}
