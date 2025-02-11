use std::collections::HashMap;
use crate::decoding::decoding_utils::DecodingUtils;
use crate::decoding::double_decoder::DoubleDecoder;
use crate::decoding::float_decoder::FloatDecoder;
use crate::decoding::integer_decoder::IntegerDecoder;
use crate::decoding::stream_metadata_decoder::StreamMetadataDecoder;
use crate::decoding::string_decoder::StringDecoder;
use crate::maplibre::properties::Property;
use crate::proto::{Column, mod_Column, mod_ScalarColumn, ScalarType, mod_ComplexColumn, ComplexType, LogicalScalarType, LogicalComplexType};

pub enum PropertyDecoderResult {
    Map(HashMap<String, Vec<Option<Property>>>),
    List(Vec<Option<Property>>),
}

pub struct PropertyDecoder {}
impl PropertyDecoder {
    pub fn decode_property_column(data: &[u8], offset: &mut usize, column: &Column, num_streams: u32) -> PropertyDecoderResult {
        let mut present_stream_metadata = None;


        let scalar_column = column.type_pb.clone();
        if scalar_column != mod_Column::OneOftype_pb::None {
            let mut present_stream = Vec::new();
            let mut num_values = 0;
            if num_streams > 1 {
                present_stream_metadata = Some(StreamMetadataDecoder::decode(data, offset));
                num_values = present_stream_metadata.unwrap().stream_metadata.num_values;
                present_stream = DecodingUtils::decode_boolean_rle(
                    data,
                    present_stream_metadata.clone().unwrap().stream_metadata.num_values as usize,
                    offset);
            }
            
            let physical_type: mod_Column::OneOftype_pb = column.type_pb.clone();
            match physical_type {
                mod_Column::OneOftype_pb::scalarType(st) => {
                    match st.type_pb {
                        mod_ScalarColumn::OneOftype_pb::physicalType(pt) => {
                            match pt {
                                ScalarType::BOOLEAN => {
                                    let data_stream_metadata = StreamMetadataDecoder::decode(data, offset);
                                    // todo: ?
                                    let data_stream = DecodingUtils::decode_boolean_rle(
                                        data,
                                        data_stream_metadata.rle_stream_metadata.unwrap().run_count as usize,
                                        offset);
                                    let mut boolean_values: Vec<Option<bool>> = vec![None; present_stream_metadata.unwrap().stream_metadata.num_values as usize];

                                    let mut counter = 0;
                                    for i in 0..present_stream_metadata.unwrap().stream_metadata.num_values as usize {
                                        let value = if present_stream[i] {
                                            let tmp = data_stream[counter];
                                            counter += 1;
                                            Some(tmp)
                                        } else { None };
                                        boolean_values.push(value);
                                    }

                                    let mut result = Vec::new();
                                    for value in boolean_values {
                                        if let Some(value) = value {
                                            result.push(Some(Property::bool(value)))
                                        }
                                    }

                                    return PropertyDecoderResult::List(result);
                                }
                                ScalarType::INT_8 => {
                                    todo!("Currently not supported")
                                }
                                ScalarType::UINT_8 => {
                                    todo!("Currently not supported")
                                }
                                ScalarType::INT_32 => {
                                    let data_stream_metadata = StreamMetadataDecoder::decode(data, offset);
                                    let data_stream = IntegerDecoder::decode_int_stream(data, offset, &data_stream_metadata, true);
                                    let mut int32_values = vec![None; present_stream_metadata.unwrap().stream_metadata.num_values as usize];

                                    let mut counter = 0;
                                    for i in 0..present_stream_metadata.unwrap().stream_metadata.num_values as usize {
                                        let value = if present_stream[i] {
                                            let tmp = data_stream[counter];
                                            counter += 1;
                                            Some(tmp)
                                        } else { None };
                                        int32_values.push(value);
                                    }

                                    let mut result = Vec::new();
                                    for value in int32_values {
                                        if let Some(value) = value {
                                            result.push(Some(Property::i32(value)))
                                        } else {
                                            result.push(None);
                                        }
                                    }

                                    return PropertyDecoderResult::List(result);
                                }
                                ScalarType::UINT_32 => {
                                    let data_stream_metadata = StreamMetadataDecoder::decode(data, offset);
                                    let data_stream = IntegerDecoder::decode_int_stream(data, offset, &data_stream_metadata, false);
                                    let data_stream = data_stream.iter().map(|i| *i as u32).collect::<Vec<u32>>();
                                    let mut uint32_values = vec![None; present_stream_metadata.unwrap().stream_metadata.num_values as usize];

                                    let mut counter = 0;
                                    for i in 0..present_stream_metadata.unwrap().stream_metadata.num_values as usize {
                                        let value = if present_stream[i] {
                                            let tmp = data_stream[counter];
                                            counter += 1;
                                            Some(tmp)
                                        } else { None };
                                        uint32_values.push(value);
                                    }

                                    let mut result = Vec::new();
                                    for value in uint32_values {
                                        if let Some(value) = value {
                                            result.push(Some(Property::u32(value)))
                                        } else {
                                            result.push(None);
                                        }
                                    }

                                    return PropertyDecoderResult::List(result);
                                }
                                ScalarType::INT_64 => {
                                    let data_stream_metadata = StreamMetadataDecoder::decode(data, offset);
                                    let data_stream = IntegerDecoder::decode_long_stream(data, offset, &data_stream_metadata, true);
                                    let mut int64_values = Vec::with_capacity(present_stream_metadata.unwrap().stream_metadata.num_values as usize);

                                    let mut counter = 0;
                                    for i in 0..present_stream_metadata.unwrap().stream_metadata.num_values as usize {
                                        let value = if present_stream[i] {
                                            let tmp = data_stream[counter];
                                            counter += 1;
                                            Some(tmp)
                                        } else { None };
                                        int64_values.push(value);
                                    }

                                    let mut result = Vec::new();
                                    for value in int64_values {
                                        if let Some(value) = value {
                                            result.push(Some(Property::i64(value)))
                                        } else {
                                            result.push(None);
                                        }
                                    }

                                    return PropertyDecoderResult::List(result);
                                }
                                ScalarType::UINT_64 => {
                                    let data_stream_metadata = StreamMetadataDecoder::decode(data, offset);
                                    let data_stream = IntegerDecoder::decode_long_stream(data, offset, &data_stream_metadata, false);
                                    let mut uint64_values = vec![None; present_stream_metadata.unwrap().stream_metadata.num_values as usize];
                                    
                                    let mut counter = 0;
                                    for i in 0..present_stream_metadata.unwrap().stream_metadata.num_values as usize {
                                        let value = if present_stream[i] {
                                            let tmp = data_stream[counter];
                                            counter += 1;
                                            Some(tmp as u64)
                                        } else { None };
                                        uint64_values.push(value);
                                    }

                                    let mut result = Vec::new();
                                    for value in uint64_values {
                                        if let Some(value) = value {
                                            result.push(Some(Property::u64(value)))
                                        } else {
                                            result.push(None);
                                        }
                                    }

                                    return PropertyDecoderResult::List(result);
                                }
                                ScalarType::FLOAT => {
                                    let data_stream_metadata = StreamMetadataDecoder::decode(data, offset);
                                    let data_stream = FloatDecoder::decode_float_stream(data, offset, &data_stream_metadata.stream_metadata);
                                    let mut float_values = vec![None; present_stream_metadata.unwrap().stream_metadata.num_values as usize];

                                    let mut counter = 0;
                                    for i in 0..present_stream_metadata.unwrap().stream_metadata.num_values as usize {
                                        let value = if present_stream[i] {
                                            let tmp = data_stream[counter];
                                            counter += 1;
                                            Some(tmp)
                                        } else { None };
                                        float_values.push(value);
                                    }

                                    let mut result = Vec::new();
                                    for value in float_values {
                                        if let Some(value) = value {
                                            result.push(Some(Property::f32(value)))
                                        }
                                    }

                                    return PropertyDecoderResult::List(result);
                                }
                                ScalarType::DOUBLE => {
                                    let data_stream_metadata = StreamMetadataDecoder::decode(data, offset);
                                    let data_stream = DoubleDecoder::decode_double_stream(data, offset, &data_stream_metadata.stream_metadata);
                                    let mut double_values = vec![None; present_stream_metadata.unwrap().stream_metadata.num_values as usize];

                                    let mut counter = 0;
                                    for i in 0..present_stream_metadata.unwrap().stream_metadata.num_values as usize {
                                        let value = if present_stream[i] {
                                            let tmp = data_stream[counter];
                                            counter += 1;
                                            Some(tmp)
                                        } else { None };
                                        double_values.push(value);
                                    }

                                    let mut result = Vec::new();
                                    for value in double_values {
                                        if let Some(value) = value {
                                            result.push(Some(Property::f64(value)))
                                        }
                                    }

                                    return PropertyDecoderResult::List(result);
                                }
                                ScalarType::STRING => {
                                    let string_values = StringDecoder::decode(data, offset, (num_streams - 1) as usize, present_stream, num_values as usize);

                                    let mut result = Vec::new();
                                    for value in string_values {
                                        if let Some(value) = value {
                                            result.push(Some(Property::String(value)));
                                        } else {
                                            result.push(None);
                                        }
                                    }

                                    return PropertyDecoderResult::List(result);
                                }
                            }
                        },
                        mod_ScalarColumn::OneOftype_pb::logicalType(lt) => {
                            match lt {
                                LogicalScalarType::TIMESTAMP => { todo!("Unsupported scalar property format yet") }
                                LogicalScalarType::DATE => { todo!("Unsupported scalar property format yet") }
                                LogicalScalarType::JSON => { todo!("Unsupported scalar property format yet") }
                            }
                        }
                        mod_ScalarColumn::OneOftype_pb::None => { todo!("Unsupported scalar property format yet") }
                    }
                }
                mod_Column::OneOftype_pb::complexType(st) => {
                    match st.type_pb {
                        mod_ComplexColumn::OneOftype_pb::physicalType(pt) => {
                            match pt {
                                ComplexType::VEC_2 => { todo!("Unsupported complex property format yet") }
                                ComplexType::VEC_3 => { todo!("Unsupported complex property format yet") }
                                ComplexType::GEOMETRY => { todo!("Unsupported complex property format yet") }
                                ComplexType::GEOMETRY_Z => { todo!("Unsupported complex property format yet") }
                                ComplexType::LIST => { todo!("Unsupported complex property format yet") }
                                ComplexType::MAP => { todo!("Unsupported complex property format yet") }
                                ComplexType::STRUCT => { todo!("Unsupported complex property format yet") }
                            }
                        },
                        mod_ComplexColumn::OneOftype_pb::logicalType(lt) => {
                            match lt {
                                LogicalComplexType::BINARY => { todo!("Unsupported complex property format yet") }
                                LogicalComplexType::RANGE_MAP => { todo!("Unsupported complex property format yet") }
                            }
                        }
                        mod_ComplexColumn::OneOftype_pb::None => { todo!("Unsupported complex property format yet") }
                    }
                }
                mod_Column::OneOftype_pb::None => { todo!("Unsupported property format yet") }
            }
        }

        if num_streams == 1 {
            panic!("Present stream currently not supported for structs")
        } else {
            PropertyDecoderResult::Map(StringDecoder::decode_shared_dictionary(data, offset, column))
        }
    }
}
