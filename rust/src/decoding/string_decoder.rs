use crate::decoding::decoding_utils::DecodingUtils;
use crate::decoding::fsst_decoding::Fsst;
use crate::decoding::integer_decoder::IntegerDecoder;
use crate::decoding::stream_metadata_decoder::StreamMetadataDecoder;
use crate::maplibre::properties::Property;
use crate::proto::mod_Column::OneOftype_pb;
use crate::proto::Column;
use crate::types::stream_types::StreamType_Dictionary::{SHARED, SINGLE};
use crate::types::stream_types::{PhysicalStreamType, StreamType_VariableSizedItems};
use std::collections::HashMap;

pub struct StringDecoder {}
impl StringDecoder {
    pub fn decode_shared_dictionary(data: &[u8], offset: &mut usize, column: &Column) -> HashMap<String, Vec<Option<Property>>> {
        let mut dictionary_length_stream = None;
        let mut dictionary_stream = None;
        let mut symbol_length_stream = None;
        let mut symbol_table_stream = None;

        // TODO: refactor to be spec compliant
        let mut dictionary_stream_decoded = false;
        while !dictionary_stream_decoded {
            let stream_metadata = StreamMetadataDecoder::decode(data, offset);
            match stream_metadata.stream_metadata.physical_stream_type {
                PhysicalStreamType::LENGTH => {
                    if stream_metadata.stream_metadata.logical_stream_type.DictionaryType.is_some() {
                        dictionary_length_stream = Some(IntegerDecoder::decode_int_stream(data, offset, &stream_metadata, false));
                    } else {
                        symbol_length_stream = Some(IntegerDecoder::decode_int_stream(data, offset, &stream_metadata, false));
                    }
                },
                PhysicalStreamType::DATA => {
                    // TODO: fix only shared should be allowed
                    if stream_metadata.stream_metadata.logical_stream_type.DictionaryType == Some(SINGLE)
                        || stream_metadata.stream_metadata.logical_stream_type.DictionaryType == Some(SHARED) {
                        dictionary_stream = Some(data[*offset..*offset + stream_metadata.stream_metadata.byte_length as usize].to_vec());
                        *offset += stream_metadata.stream_metadata.byte_length as usize;
                        dictionary_stream_decoded = true;
                    } else {
                        symbol_table_stream = Some(data[*offset..*offset + stream_metadata.stream_metadata.byte_length as usize].to_vec());
                        *offset += stream_metadata.stream_metadata.byte_length as usize;
                    }
                },
                _ => { panic!("Exception in decode_shared_dictionary"); }
            }
        }

        let dictionary = if symbol_length_stream.is_some() {
            let utf8_values = Fsst::decode_unknown_length(
                symbol_table_stream.unwrap().as_slice(),
                &*symbol_length_stream.unwrap().iter().map(|i| *i as u32).collect::<Vec<u32>>(),
                dictionary_stream.unwrap().as_slice(),
            );
            Self::decode_dictionary_utf8_values(dictionary_length_stream.unwrap(), utf8_values)
        } else {
            Self::decode_dictionary_utf8_values(dictionary_length_stream.unwrap(), dictionary_stream.unwrap())
        };

        let mut values = HashMap::new();

        match &column.type_pb {
            OneOftype_pb::scalarType(_) => { panic!("Protobuf error: Scalar") }
            OneOftype_pb::complexType(ct) => {
                for child_field in ct.children.clone() {
                    let num_streams = DecodingUtils::decode_varint(data, offset, 1)[0];
                    if num_streams != 2 { panic!("Currently only optional string fields are implemented for a struct") }

                    let present_stream_metadata = StreamMetadataDecoder::decode(data, offset);
                    let present_stream = DecodingUtils::decode_boolean_rle(
                        data,
                        present_stream_metadata.stream_metadata.num_values as usize,
                        offset
                    );
                    let data_stream_metadata = StreamMetadataDecoder::decode(data, offset);
                    let data_reference_stream = IntegerDecoder::decode_int_stream(
                        data,
                        offset,
                        &data_stream_metadata,
                        false
                    );

                    let mut property_values = Vec::with_capacity(present_stream_metadata.stream_metadata.num_values as usize);
                    let mut counter = 0;
                    for i in 0..present_stream_metadata.stream_metadata.num_values {
                        let present = present_stream[i as usize];
                        if present {
                            let data_reference = data_reference_stream[counter];
                            counter += 1;
                            let value = dictionary[data_reference as usize].to_string();
                            property_values.push(Some(Property::String(value)));
                        } else {
                            property_values.push(None);
                        }
                    }

                    // TODO: get delimiter sign from column mappings
                    let mut column_name = column.name.to_string();
                    if child_field.name == "default" { } else {
                        column_name.push_str(":");
                        column_name.push_str(child_field.name.as_ref());
                    }
                    // TODO: refactor to work also when present stream is null
                    values.insert(column_name.to_string(), property_values);
                }
            }
            OneOftype_pb::None => { panic!("Protobuf error: None") }
        }

        values
    }

    pub fn decode_dictionary_utf8_values(length_stream: Vec<i32>, utf8_values: Vec<u8>) -> Vec<String> {
        let mut dictionary = Vec::new();
        let mut dictionary_offset = 0;
        for length in length_stream {
            let value = utf8_values[dictionary_offset..(dictionary_offset + length as usize)].to_vec();
            dictionary.push(String::from_utf8(value).unwrap());
            dictionary_offset += length as usize;
        }
        dictionary
    }

    pub fn decode(
        data: &[u8],
        offset: &mut usize,
        num_streams: usize,
        present_stream: Vec<bool>,
        num_values: usize,
    ) -> Vec<Option<String>> {
        let mut offset_stream = Vec::new();
        let data_stream = Vec::new();

        let mut dictionary_length_stream = Vec::new();
        let mut symbol_length_stream = Vec::new();

        let mut dictionary_stream = Vec::new();
        let mut symbol_table_stream = Vec::new();

        for _ in 0..num_streams {
            let stream_metadata = StreamMetadataDecoder::decode(data, offset);
            match stream_metadata.stream_metadata.physical_stream_type {
                PhysicalStreamType::OFFSET => {
                    offset_stream = IntegerDecoder::decode_int_stream(data, offset, &stream_metadata, false);
                },
                PhysicalStreamType::LENGTH => {
                    let ls = IntegerDecoder::decode_int_stream(data, offset, &stream_metadata, false);
                    if stream_metadata.stream_metadata.logical_stream_type.LengthType == Some(StreamType_VariableSizedItems::DICTIONARY) {
                        dictionary_length_stream = ls;
                    } else {
                        symbol_length_stream = ls;
                    }
                },
                PhysicalStreamType::DATA => {
                    let ds = Vec::from(&data[*offset..*offset + stream_metadata.stream_metadata.byte_length as usize]);
                    *offset += stream_metadata.stream_metadata.byte_length as usize;

                    if stream_metadata.stream_metadata.logical_stream_type.DictionaryType == Some(SINGLE) {
                        dictionary_stream = ds;
                    } else {
                        symbol_table_stream = ds;
                    }
                },
                _ => panic!("Unexpected stream type"),
            }
        }

        if !symbol_table_stream.is_empty() {
            let symbol_length_stream = symbol_length_stream.iter().map(|i| *i as u32).collect::<Vec<u32>>();
            let utf8_values = Fsst::decode_unknown_length(
                &*symbol_table_stream,
                symbol_length_stream.as_slice(),
                &*dictionary_stream
            );
            Self::decode_dictionary(present_stream, dictionary_length_stream, utf8_values, offset_stream, num_values)
        } else if !dictionary_stream.is_empty() {
            Self::decode_dictionary(present_stream, dictionary_length_stream, dictionary_stream, offset_stream, num_values)
        } else {
            Self::decode_plain(present_stream, dictionary_length_stream, data_stream, num_values)
        }
    }

    /// Decodes a stream of strings with optional null values.
    ///
    /// Takes encoded components and reconstructs the original sequence of optional strings.
    /// Strings are encoded as UTF-8 bytes with their lengths stored separately.
    ///
    /// # Arguments
    /// * `present_stream` - Boolean vector indicating presence (true) or absence (null/false) of values
    /// * `length_stream` - Lengths of the non-null strings in sequence
    /// * `utf8_values` - Concatenated UTF-8 bytes of all non-null strings
    /// * `num_values` - Total number of values (including nulls) to decode
    ///
    /// # Example
    /// ```
    /// use maplibre_tile_spec::decoding::string_decoders::StringDecoder;
    /// 
    /// let present = vec![true, false, true];
    /// let lengths = vec![5, 5];
    /// let utf8 = "HelloWorld".as_bytes().to_vec();
    /// let result = StringDecoder::decode_plain(present, lengths, utf8, 3);
    /// assert_eq!(result, vec![Some("Hello".to_string()), None, Some("World".to_string())]);
    /// ```
    pub fn decode_plain(
        present_stream: Vec<bool>,
        length_stream: Vec<i32>,
        utf8_values: Vec<u8>,
        num_values: usize
    ) -> Vec<Option<String>> {
        let mut decoded_values = Vec::new();
        let mut length_offset = 0;
        let mut str_offset = 0;

        for i in 0..num_values {
            if present_stream[i] == true {
                let length = length_stream[length_offset];
                length_offset += 1;
                let value = String::from_utf8(
                    Vec::from(&utf8_values[str_offset..str_offset + length as usize]))
                    .unwrap();
                decoded_values.push(Some(value.to_string()));
                str_offset += length as usize;
            } else {
                decoded_values.push(None);
            }
        }

        decoded_values
    }

    pub fn decode_dictionary(
        present_stream: Vec<bool>,
        length_stream: Vec<i32>,
        utf8_values: Vec<u8>,
        dictionary_offsets: Vec<i32>,
        num_values: usize,
    ) -> Vec<Option<String>> {
        let mut dictionary = Vec::new();
        let mut dictionary_offset = 0;

        for length in length_stream {
            let value = String::from_utf8_lossy(&utf8_values[dictionary_offset..dictionary_offset + length as usize]);
            dictionary.push(value.to_string());
            dictionary_offset += length as usize;
        }

        let mut values: Vec<Option<String>> = Vec::new();
        let mut offset = 0;

        for i in 0..num_values {
            if present_stream[i] {
                let value: String = dictionary[dictionary_offsets[offset] as usize].clone();
                values.push(Some(value));
                offset += 1;
            } else {
                values.push(None);
            }
        }

        values
    }
}
