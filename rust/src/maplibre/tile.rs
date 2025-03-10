use crate::decoding::decoding_utils::DecodingUtils;
use crate::decoding::geometry_decoder::GeometryDecoder;
use crate::decoding::integer_decoder::IntegerDecoder;
use crate::decoding::property_decoder::{PropertyDecoder, PropertyDecoderResult};
use crate::decoding::stream_metadata_decoder::StreamMetadataDecoder;
use crate::maplibre::feature::MapLibreFeature;
use crate::maplibre::layer::MapLibreLayer;
use crate::maplibre::properties::Property;
use crate::proto::{mod_Column, mod_ScalarColumn};
use crate::proto::{FeatureTableSchema, ScalarType, TileSetMetadata};
use crate::types::geometries::Geometry;
use core::fmt::Debug;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone, PartialEq)]
pub struct MapLibreTile {
    pub layers: Vec<MapLibreLayer>,
}

impl Serialize for MapLibreTile {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("MapLibreTile", 1)?;

        state.serialize_field("layers", &self.layers)?;

        state.end()
    }
}

impl MapLibreTile {
    pub fn decode(data: &Box<Vec<u8>>, tile_metadata: &TileSetMetadata) -> Self {
        let mut offset = 0;
        let mut mltile = MapLibreTile { layers: Vec::with_capacity(tile_metadata.featureTables.len()) };

        while offset < data.len() {
            let mut ids = Vec::new();
            let mut geometries = Vec::new();
            let mut properties = HashMap::new();


            let version = data[offset];
            offset += 1;
            let infos = DecodingUtils::decode_varint(data, &mut offset, 4);
            let feature_table_id = infos[0];
            let extent = infos[1];
            let num_features = infos[3];
            let metadata = tile_metadata.featureTables.get(feature_table_id as usize);
            if metadata.is_none() {
                todo!("Warn: could not find metadata for feature table id: {}", feature_table_id);
            }
            for column_metadata in &metadata.unwrap().columns {
                let column_name = column_metadata.name.clone();
                let num_streams = DecodingUtils::decode_varint(data, &mut offset, 1)[0];

                if column_name == "id" {
                    if num_streams == 2 {
                        let present_stream_metadata = StreamMetadataDecoder::decode(data, &mut offset);

                        // todo: return value of this function if not used, as advance offset without decoding?
                        DecodingUtils::decode_boolean_rle(
                            data,
                            present_stream_metadata.stream_metadata.num_values as usize,
                            &mut offset);
                    } else {
                        panic!("Unsupported number of streams for ID column: {}", num_streams);
                    }

                    let id_data_stream_metadata = StreamMetadataDecoder::decode(data, &mut offset);
                    let physical_type = {
                        match column_metadata.type_pb.clone() {
                            mod_Column::OneOftype_pb::scalarType(st) => {
                                match st.type_pb {
                                    mod_ScalarColumn::OneOftype_pb::physicalType(pt) => Some(pt),
                                    _ => None
                                }
                            }
                            _ => None
                        }
                    };
                    if physical_type == Some(ScalarType::UINT_32) {
                        let tmp = IntegerDecoder::decode_int_stream(data, &mut offset, &id_data_stream_metadata, false);
                        ids = tmp.iter().map(|i| *i as i64).collect();
                    } else if physical_type == Some(ScalarType::UINT_64) {
                        ids = IntegerDecoder::decode_long_stream(data, &mut offset, &id_data_stream_metadata, false);
                    } else {
                        panic!("Unsupported column type: {:?}", column_metadata.type_pb);
                    }
                } else if column_name == "geometry" {
                    let geometry_column = GeometryDecoder::decode_geometry_column(data, num_streams as usize, &mut offset);
                    geometries = GeometryDecoder::decode_geometry(geometry_column);
                } else {
                    let property_column = PropertyDecoder::decode_property_column(data, &mut offset, column_metadata, num_streams);
                    match property_column {
                        PropertyDecoderResult::Map(map) => {
                            for entry in map.iter() {
                                properties.insert(
                                    entry.0.to_string(),
                                    entry.1.clone()
                                );
                            }
                        }
                        PropertyDecoderResult::List(list) => {
                            properties.insert(column_name.to_string(), list);
                        }
                    }
                }
            }
            mltile.layers.push(Self::convert_to_layer(ids, extent, version, geometries, properties, metadata.unwrap(), num_features));
        }



        mltile
    }

    fn convert_to_layer(ids: Vec<i64>, extent: u32, version: u8, geometries: Vec<Geometry>, properties: HashMap<String, Vec<Option<Property>>>, metadata: &FeatureTableSchema, num_features: u32) -> MapLibreLayer {


        if num_features != geometries.len() as u32
            || num_features != ids.len() as u32 {
            println!("[Warn][MapLibreTile::convert_to_layer] ids({}), geometries({}) and features({}) for one layer are not equal", ids.len(), geometries.len(), num_features);
        }

        let mut features = Vec::with_capacity(num_features as usize);

        // fs::write("F:\\Maps\\debug.txt", format!("{:#?}", properties)).expect("TODO: panic message");
        
        for i in 0..num_features as usize {
            let mut props = HashMap::new();
            for (key, value) in &properties {
                if let Some(val) = value.clone().get(i) { props.insert(key.clone(), val.clone()); }
            }

            features.push(MapLibreFeature {
                id: ids[i],
                geometry: geometries[i].clone(),
                properties: props,
            });
        }

        MapLibreLayer {
            version,
            name: metadata.name.to_string(),
            features,
            tile_extent: 0,// extent,  // TODO: reenable
        }

        // let mut features = Vec::with_capacity(num_features as usize);
        // for j in 0..num_features as usize {
        //     let mut p: HashMap<String, String> = HashMap::new();
        //     for (key, value) in &properties {
        //         match value {
        //             Properties::boolean_property(value) => {
        //                 if let Some(val) = value.get(j) {
        //                     if !val.is_some() {
        //                         p.insert(key.clone(), val.unwrap().to_string());
        //                     }
        //                 }
        //             }
        //             Properties::int8_property(value) => {
        //                 if let Some(val) = value.get(j) {
        //                     if !val.is_some() {
        //                         p.insert(key.clone(), val.unwrap().to_string());
        //                     }
        //                 }
        //             }
        //             Properties::uint8_property(value) => {
        //                 if let Some(val) = value.get(j) {
        //                     if !val.is_some() {
        //                         p.insert(key.clone(), val.unwrap().to_string());
        //                     }
        //                 }
        //             }
        //             Properties::int32_property(value) => {
        //                 if let Some(val) = value.get(j) {
        //                     if !val.is_some() {
        //                         p.insert(key.clone(), val.unwrap().to_string());
        //                     }
        //                 }
        //             }
        //             Properties::uint32_property(value) => {
        //                 if let Some(val) = value.get(j) {
        //                     if !val.is_some() {
        //                         p.insert(key.clone(), val.unwrap().to_string());
        //                     }
        //                 }
        //             }
        //             Properties::int64_property(value) => {
        //                 if let Some(val) = value.get(j) {
        //                     if !val.is_some() {
        //                         p.insert(key.clone(), val.unwrap().to_string());
        //                     }
        //                 }
        //             }
        //             Properties::uint64_property(value) => {
        //                 if let Some(val) = value.get(j) {
        //                     if !val.is_some() {
        //                         p.insert(key.clone(), val.unwrap().to_string());
        //                     }
        //                 }
        //             }
        //             Properties::float_property(value) => {
        //                 if let Some(val) = value.get(j) {
        //                     if !val.is_some() {
        //                         p.insert(key.clone(), val.unwrap().to_string());
        //                     }
        //                 }
        //             }
        //             Properties::double_property(value) => {
        //                 if let Some(val) = value.get(j) {
        //                     if !val.is_some() {
        //                         p.insert(key.clone(), val.unwrap().to_string());
        //                     }
        //                 }
        //             }
        //             Properties::string_property(value) => {
        //                 if let Some(val) = value.get(j) {
        //                     if !val.is_some() {
        //                         p.insert(key.clone(), val.clone().unwrap());
        //                     }
        //                 }
        //             }
        //         }
        //     }
        //     features.push(MapLibreFeature {
        //         id: ids[j] as u32,
        //         extent,
        //         properties: p,
        //     });
        // }
        //
        // MapLibreLayer {
        //     name: metadata.name.to_string(),
        //     version,
        //     length: features.len() as u32,
        //     features
        // }
    }
}
