use std::collections::HashMap;

use bitvec::prelude::*;
use bytes::Buf;
use zigzag::ZigZag;

use crate::data::MapLibreTile;
use crate::decoder::boolean::decode_boolean_rle;
use crate::decoder::helpers::get_data_type_from_column;
use crate::decoder::tracked_bytes::TrackedBytes;
use crate::encoder::geometry::GeometryScaling;
use crate::metadata::proto_tileset::{Column, ScalarType, TileSetMetadata};
use crate::metadata::stream::StreamMetadata;
use crate::{MltError, MltResult, decoder};

const ID_COLUMN_NAME: &str = "id";
const GEOMETRY_COLUMN_NAME: &str = "geometry";

pub struct Config {
    pub feature_table_decoding: Option<HashMap<String, String>>,
    pub geometry_scaling: Option<GeometryScaling>,
    pub id_within_max_safe_integer: bool,
}

pub struct Decoder {
    pub tile: TrackedBytes,
    pub config: Option<Config>,
}

impl Decoder {
    pub fn new<T: Into<TrackedBytes>>(tile: T, config: Option<Config>) -> Self {
        Self {
            tile: tile.into(),
            config,
        }
    }

    pub fn decode(&mut self, tile_metadata: &TileSetMetadata) -> MltResult<MapLibreTile> {
        while self.tile.has_remaining() {
            // let ids: Vec<i64> = vec![];
            // let geometries: Vec<Geometry> = vec![];

            let _version = self.tile.get_u8();
            let infos = decoder::varint::decode(&mut self.tile, 5)?;

            println!("infos: {infos:?}");

            let feature_table_id: u32 = *infos.first().ok_or(MltError::MissingInfo(0))?;
            let feature_table_body_size: u32 = *infos.get(1).ok_or(MltError::MissingInfo(1))?;
            let feature_table_metadata = tile_metadata
                .feature_tables
                .get(feature_table_id as usize)
                .ok_or(MltError::FeatureTableNotFound(feature_table_id))?;

            let property_column_names: Option<&String> = self.config.as_ref().and_then(|cfg| {
                cfg.feature_table_decoding
                    .as_ref()
                    .and_then(|ftd| ftd.get(&feature_table_metadata.name))
            });

            if property_column_names.is_none() {
                self.tile.advance(feature_table_body_size as usize);
                continue;
            }

            let _extent = *infos.get(2).ok_or(MltError::MissingInfo(2))?;
            let _max_tile_extent: i32 =
                ZigZag::decode(*infos.get(3).ok_or(MltError::MissingInfo(3))?);
            let num_features = infos.get(4).ok_or(MltError::MissingInfo(4))?;

            for col_metadata in &feature_table_metadata.columns {
                let num_streams_vec = decoder::varint::decode::<u32>(&mut self.tile, 1)?;
                let num_streams = num_streams_vec
                    .first()
                    .ok_or(MltError::MissingField("num_streams"))?;
                if col_metadata.name == ID_COLUMN_NAME {
                    let mut nullability_buffer = BitVec::<u8, Lsb0>::EMPTY;
                    if *num_streams == 2 {
                        let present_stream_metadata = StreamMetadata::decode(&mut self.tile)?;
                        let values = decode_boolean_rle(
                            &mut self.tile,
                            present_stream_metadata.num_values as usize,
                        );

                        nullability_buffer =
                            BitVec::<u8, Lsb0>::with_capacity(*num_features as usize);
                        nullability_buffer.extend(values.iter().copied());
                    }

                    // Dummy
                    nullability_buffer.resize(1, false);

                    let id_data_stream_metadata = StreamMetadata::decode(&mut self.tile)?;
                    let id_data_type = get_data_type_from_column(col_metadata)?;

                    // Decode ID column based on its data type
                    if id_data_type == ScalarType::Uint32 {
                        println!("Decoding ID column with Uint32 type");
                        let ids = decoder::integer::decode_int_stream(
                            &mut self.tile,
                            &id_data_stream_metadata,
                            false,
                        );
                        println!("Decoded IDs: {ids:?}");
                    }
                    // TODO: Handle 64-bit integers and other types
                    else {
                        let ids = decoder::integer::decode_long_stream(
                            &mut self.tile,
                            &id_data_stream_metadata,
                            false,
                        );
                        println!("Decoded IDs: {ids:?}");
                    }
                }
            }
        }

        // TODO: Implement the rest of the decoding logic
        // For now, just return an empty MapLibreTile
        // This is a placeholder to avoid compilation errors
        Ok(MapLibreTile { layers: vec![] })
    }

    fn decode_id_column(
        &mut self,
        column_metadata: &Column,
        _column_name: &str,
        _nullability_buffer: BitVec<u8>,
        _id_within_max_safe_integer: bool,
    ) -> MltResult<()> {
        let _id_data_stream_metadata = StreamMetadata::decode(&mut self.tile)?;
        let _id_data_type = get_data_type_from_column(column_metadata)?;
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
    #[ignore = "tile format has changed, the decode function is no longer valid. See mlt crate for updated parsing."]
    fn test_decode() {
        let raw = fs::read("../../test/expected/omt/2_2_2.mlt").expect("Failed to read file");
        let mut mlt = Decoder::new(raw, None);
        let metadata = read_metadata(Path::new("../../test/expected/omt/2_2_2.mlt.meta.pbf"))
            .expect("Failed to read metadata");

        // Write metadata to a txt file
        let metadata_str = format!("{metadata:#?}");
        fs::write("../target/metadata_output.txt", metadata_str)
            .expect("Failed to write metadata to file");

        let _tile = mlt.decode(&metadata).expect("Failed to decode tile");
    }
}
