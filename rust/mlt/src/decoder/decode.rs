use std::collections::HashMap;

use bitvec::prelude::*;
use bytes::{Buf, Bytes};
use geo_types::Geometry;
use zigzag::ZigZag;

use crate::data::MapLibreTile;
use crate::decoder::helpers::{decode_boolean_rle, get_data_type_from_column};
use crate::decoder::varint;
use crate::encoder::geometry::GeometryScaling;
use crate::metadata::proto_tileset::{Column, TileSetMetadata};
use crate::metadata::stream::StreamMetadata;
use crate::{MltError, MltResult};

const ID_COLUMN_NAME: &str = "id";
const GEOMETRY_COLUMN_NAME: &str = "geometry";

pub struct Config {
    pub feature_table_decoding: Option<HashMap<String, String>>,
    pub geometry_scaling: Option<GeometryScaling>,
    pub id_within_max_safe_integer: bool,
}

pub struct Decoder {
    pub tile: Bytes,
    pub config: Option<Config>,
    pub original_tile_size: usize,
}

impl Decoder {
    pub fn new(tile: Vec<u8>, config: Option<Config>) -> Self {
        let tile = Bytes::from(tile);
        Self {
            original_tile_size: tile.len(),
            tile,
            config,
        }
    }

    // Returns the offset of the tile data for debugging purposes
    // NOTE: This is a temporary workaround to track the offset,
    // and should be removed once `Bytes` supports offset tracking internally.
    pub fn offset(&self) -> usize {
        self.original_tile_size - self.tile.len()
    }

    #[expect(unused_variables)]
    pub fn decode(&mut self, tile_metadata: &TileSetMetadata) -> MltResult<MapLibreTile> {
        while self.tile.has_remaining() {
            let ids: Vec<i64> = vec![];
            let geometries: Vec<Geometry> = vec![];

            let version = self.tile.get_u8();
            let infos = varint::decode(&mut self.tile, 5);

            println!("infos: {:?}", infos);

            let feature_table_id = infos.first().ok_or_else(|| {
                MltError::DecodeError("Failed to read feature table id".to_string())
            })?;
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

            let property_column_names: Option<&String> = self.config.as_ref().and_then(|cfg| {
                cfg.feature_table_decoding
                    .as_ref()
                    .and_then(|ftd| ftd.get(&feature_table_metadata.name))
            });

            if property_column_names.is_none() {
                self.tile.advance(*feature_table_body_size as usize);
                continue;
            }

            let extent = infos
                .get(2)
                .ok_or_else(|| MltError::DecodeError("Failed to read tile extent".to_string()))?;

            let max_tile_extent: i32 = ZigZag::decode(*infos.get(3).ok_or_else(|| {
                MltError::DecodeError("Failed to read max tile extent".to_string())
            })?);

            let num_features = infos.get(4).ok_or_else(|| {
                MltError::DecodeError("Failed to read number of features".to_string())
            })?;

            for col_metadata in feature_table_metadata.columns.iter() {
                let num_streams_vec = varint::decode(&mut self.tile, 1);
                let num_streams = num_streams_vec.first().ok_or_else(|| {
                    MltError::DecodeError("Failed to retrieve num_streams".to_string())
                })?;
                if col_metadata.name == ID_COLUMN_NAME {
                    let mut nullability_buffer = BitVec::<u8, Lsb0>::EMPTY;
                    if *num_streams == 2 {
                        let present_stream_metadata = StreamMetadata::decode(&mut self.tile)?;
                        let values = decode_boolean_rle(
                            &mut self.tile,
                            present_stream_metadata.num_values as usize,
                        )?;
                        nullability_buffer =
                            BitVec::<u8, Lsb0>::with_capacity(*num_features as usize);
                        nullability_buffer.extend(values.iter().copied());
                    }

                    // Dummy
                    nullability_buffer.resize(1, false);
                }
            }
        }

        // TODO: Implement the rest of the decoding logic
        // For now, just return an empty MapLibreTile
        // This is a placeholder to avoid compilation errors
        Ok(MapLibreTile { layers: vec![] })
    }

    #[expect(unused_variables)]
    fn decode_id_column(
        &mut self,
        column_metadata: &Column,
        column_name: &str,
        nullability_buffer: BitVec<u8>,
        id_within_max_safe_integer: bool,
    ) -> MltResult<()> {
        let id_data_stream_metadata = StreamMetadata::decode(&mut self.tile)?;
        let id_data_type = get_data_type_from_column(column_metadata)?;
        Ok(())
    }
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
        let mut mlt = Decoder::new(raw, None);
        let metadata = read_metadata(Path::new(
            "../../ts/test/data/omt/unoptimized/mlt/plain/tileset.pbf",
        ))
        .expect("Failed to read metadata");

        // Write metadata to a txt file
        let metadata_str = format!("{:#?}", metadata);
        fs::write("../target/metadata_output.txt", metadata_str)
            .expect("Failed to write metadata to file");

        let tile = mlt.decode(&metadata).expect("Failed to decode tile");
    }
}
