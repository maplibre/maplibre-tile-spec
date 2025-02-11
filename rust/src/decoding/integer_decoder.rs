use crate::decoding::decoding_utils::DecodingUtils;
use crate::headers::stream::{HasStreamMetadata, MortonEncodedStreamMetadata};
use crate::types::compression_types::{LogicalLevelCompressionTechnique, PhysicalLevelCompressionTechnique};

pub struct IntegerDecoder {}
impl IntegerDecoder {

    /// Decodes a Morton-encoded stream of integers from a byte array.
    ///
    /// # Arguments
    /// * `data` - The byte array containing the encoded data
    /// * `offset` - Mutable reference to the current offset in the byte array
    /// * `stream_metadata` - Metadata about the encoded stream including compression technique and number of values
    ///
    /// # Returns
    /// A vector of decoded i32 integers
    ///
    /// # Panics
    /// Panics if an unsupported physical level compression technique is specified (currently only VARINT is supported)
    pub fn decode_morton_stream(data: &[u8], offset: &mut usize, stream_metadata: &MortonEncodedStreamMetadata) -> Vec<i32> {
        let values: Vec<i32>;
        match stream_metadata.base.physical_level_technique {
            PhysicalLevelCompressionTechnique::VARINT => {
                let tmp = DecodingUtils::decode_varint(data, offset, stream_metadata.base.num_values as usize);
                // fast typecasting, using mem::transmute
                values = unsafe { core::mem::transmute(tmp) };
            },
            PhysicalLevelCompressionTechnique::FAST_PFOR => {
                let tmp = DecodingUtils::decode_fastpfor(data, offset, stream_metadata.base.num_values, stream_metadata.base.byte_length);
                // fast typecasting, using mem::transmute
                values = unsafe { core::mem::transmute(tmp) };
            },
            _ => {
                panic!("Specified physical level technique not yet supported: ALP");
            }
        }

        Self::decode_morton_delta(&values, stream_metadata.base.num_values as usize, 0)
    }

    pub fn decode_morton_delta(data: &[i32], num_bits: usize, coordinate_shift: usize) -> Vec<i32> {
        let mut vertices = Vec::new();
        let mut previous_morton_code = 0;
        for delta_code in data {
            let morton_code = previous_morton_code + delta_code;
            let vertex = Self::decode_morton_code(morton_code, num_bits, coordinate_shift);
            vertices.push(vertex[0]);
            vertices.push(vertex[1]);
            previous_morton_code = morton_code;
        }
        vertices
    }

    pub fn decode_morton_codes(data: Vec<i32>, num_bits: usize, coordinate_shift: usize) -> Vec<i32> {
        let mut vertices = Vec::new();
        for morton_code in data {
            let vertex = Self::decode_morton_code(morton_code, num_bits, coordinate_shift);
            vertices.push(vertex[0]);
            vertices.push(vertex[1]);
        }
        vertices
    }

    pub fn decode_morton_code(morton_code: i32, num_bits: usize, coordinate_shift: usize) -> [i32; 2] {
        let x = Self::decode_morton(morton_code as u32, num_bits) as i32 - coordinate_shift as i32;
        let y = Self::decode_morton(morton_code as u32 >> 1, num_bits) as i32 - coordinate_shift as i32;
        [x, y]
    }

    pub fn decode_morton(code: u32, num_bits: usize) -> u32 {
        let mut coordinate = 0;
        for i in 0..num_bits {
            coordinate |= (code & (1u32.wrapping_shl((2 * i) as u32))).wrapping_shr(i as u32);
        }
        coordinate
    }

    pub fn decode_int_stream(data: &[u8], offset: &mut usize, metadata: &HasStreamMetadata, is_signed: bool) -> Vec<i32> {
        let mut values= match metadata.stream_metadata.physical_level_technique {
            PhysicalLevelCompressionTechnique::VARINT => {
                DecodingUtils::decode_varint(data, offset, metadata.stream_metadata.num_values as usize)
                    .iter()
                    .map(|i| *i as i32)
                    .collect::<Vec<i32>>()
            },
            PhysicalLevelCompressionTechnique::FAST_PFOR => {
                DecodingUtils::decode_fastpfor(data, offset, metadata.stream_metadata.num_values, metadata.stream_metadata.byte_length)
                    .iter()
                    .map(|i| *i as i32)
                    .collect::<Vec<i32>>()
            }
            _ => {
                panic!("[IntegerDecoder::decode_int_stream] PhysicalLevelCompressionTechnique not implemented yet! ({:?})", metadata.stream_metadata.physical_level_technique);
            }
        };
        Self::decode_int_array(&mut values, metadata, is_signed)
    }

    pub fn decode_int_array(values: &mut Vec<i32>, metadata: &HasStreamMetadata, is_signed: bool) -> Vec<i32> {
        match metadata.stream_metadata.logical_level_technique1 {
            LogicalLevelCompressionTechnique::DELTA => {
                if metadata.stream_metadata.logical_level_technique2 == LogicalLevelCompressionTechnique::RLE {
                    let rle_metadata = metadata.rle_stream_metadata.expect("RLE Stream metadata not provided while decoding");
                    // todo: make type cast faster with mem::transmute
                    let tmp = values.iter().map(|i| *i as u32).collect::<Vec<u32>>();
                    let tmp2 = DecodingUtils::decode_unsigned_rle(&*tmp, rle_metadata.run_count as usize, rle_metadata.num_rle_values as usize);
                    // todo: make type cast faster with mem::transmute
                    let tmp3 = tmp2.iter().map(|i| *i as i32).collect::<Vec<i32>>();

                    return Self::decode_zig_zag_delta(&tmp3);
                }
                Self::decode_zig_zag_delta(values)
            },
            LogicalLevelCompressionTechnique::RLE => {
                let rle_metadata = metadata.rle_stream_metadata.expect("RLE Stream metadata not provided while decoding");
                let decoded_values = Self::decode_rle(values, rle_metadata.run_count as usize);

                if is_signed { Self::decode_zig_zag(&decoded_values) } else { decoded_values }
            },
            LogicalLevelCompressionTechnique::NONE => {
                if is_signed { Self::decode_zig_zag(values) } else { values.to_vec() }
            },
            LogicalLevelCompressionTechnique::MORTON => {
                let morton_metadata = metadata.morton_stream_metadata.expect("Morton Stream metadata not provided while decoding");

                // todo: clone bad
                Self::decode_morton_codes(values.clone(), morton_metadata.num_bits as usize, morton_metadata.coordinate_shift as usize)
            },
            LogicalLevelCompressionTechnique::COMPONENTWISE_DELTA => {
                // todo: make type cast faster with mem::transmute
                DecodingUtils::decode_componentwise_delta_vec2(values);
                values.to_vec()
            }
            _ => panic!("Unsupported logical level technique for integers: {:#?}", metadata.stream_metadata.logical_level_technique1),
        }
    }

    pub fn decode_long_stream(data: &[u8], offset: &mut usize, metadata: &HasStreamMetadata, is_signed: bool) -> Vec<i64> {
        let values = DecodingUtils::decode_long_varint(data, offset, metadata.stream_metadata.num_values as usize);
        // todo: make type cast faster with mem::transmute
        let values = values.iter().map(|i| *i as i64).collect::<Vec<i64>>();
        Self::decode_long_array(values, metadata, is_signed)
    }

    pub fn decode_long_array(values: Vec<i64>, metadata: &HasStreamMetadata, is_signed: bool) -> Vec<i64> {
        match metadata.stream_metadata.logical_level_technique1 {
            LogicalLevelCompressionTechnique::DELTA => {
                if metadata.stream_metadata.logical_level_technique2 == LogicalLevelCompressionTechnique::RLE {
                    let rle_metadata = metadata.rle_stream_metadata.expect("RLE Stream metadata not provided while decoding");
                    // todo: make type cast faster with mem::transmute
                    let values = values.iter().map(|i| *i as u64).collect::<Vec<u64>>();
                    let values = DecodingUtils::decode_unsigned_rle_long(values.as_slice(), rle_metadata.run_count as usize, rle_metadata.num_rle_values as usize);
                    // todo: make type cast faster with mem::transmute
                    let values = values.iter().map(|i| *i as i64).collect::<Vec<i64>>();

                    Self::decode_long_zig_zag_delta(&values)
                } else {
                    Self::decode_long_zig_zag_delta(&values)
                }
            }
            LogicalLevelCompressionTechnique::RLE => {
                let rle_metadata = metadata.rle_stream_metadata.expect("RLE Stream metadata not provided while decoding");
                let decoded_values = Self::decode_long_rle(&*values, rle_metadata.run_count as usize);
                if is_signed {
                    Self::decode_zig_zag_long(&decoded_values)
                } else {
                    decoded_values
                }
            }
            LogicalLevelCompressionTechnique::NONE => {
                if is_signed {
                    // todo: make type cast faster with mem::transmute
                    let values = values.iter().map(|i| *i as i32).collect::<Vec<i32>>();
                    let values = Self::decode_zig_zag(&values);
                    // todo: make type cast faster with mem::transmute
                    values.iter().map(|i| *i as i64).collect::<Vec<i64>>()
                } else {
                    values
                }
            }
            _ => panic!("The specified logical level technique is not supported for integers: {:?}", metadata.stream_metadata.logical_level_technique1),
        }
    }

    pub fn decode_rle(data: &[i32], num_runs: usize) -> Vec<i32> {
        let mut values = Vec::new();
        for i in 0..num_runs {
            let run = data[i] as usize;
            let value = data[i + num_runs];
            for _ in 0..run {
                values.push(value);
            }
        }
        values
    }

    pub fn decode_long_rle(data: &[i64], num_runs: usize) -> Vec<i64> {
        let mut values = Vec::new();
        for i in 0..num_runs {
            let run = data[i] as usize;
            let value = data[i + num_runs];
            for _ in 0..run {
                values.push(value);
            }
        }
        values
    }

    pub fn decode_zig_zag_delta(data: &Vec<i32>) -> Vec<i32> {
        let mut values = Vec::new();
        let mut previous_value = 0;
        for zig_zag_delta in data {
            let delta = DecodingUtils::decode_zig_zag(*zig_zag_delta);
            let value = previous_value + delta;
            values.push(value);
            previous_value = value;
        }
        values
    }


    /// Decodes a delta-encoded sequence of integers back into the original values.
    /// 
    /// Delta encoding stores sequential values as differences from the previous value.
    /// This function reconstructs the original sequence by accumulating the deltas.
    ///
    /// # Examples
    ///
    /// ```
    /// use maplibre_tile_spec::decoding::integer_decoder::IntegerDecoder;
    /// 
    /// let deltas = vec![5, 2, 3, -1];
    /// let values = IntegerDecoder::decode_delta(&deltas);
    /// assert_eq!(values, vec![5, 7, 10, 9]);
    /// ```
    pub fn decode_delta(data: &Vec<i32>) -> Vec<i32> {
        let mut values = Vec::new();
        let mut previous_value = 0;
        for delta in data {
            let value = previous_value + delta;
            values.push(value);
            previous_value = value;
        }
        values
    }

    pub fn decode_long_zig_zag_delta(data: &Vec<i64>) -> Vec<i64> {
        let mut values = Vec::new();
        let mut previous_value = 0;
        for zig_zag_delta in data {
            let delta = DecodingUtils::decode_zig_zag_long(*zig_zag_delta);
            let value = previous_value + delta;
            values.push(value);
            previous_value = value;
        }
        values
    }

    pub fn decode_zig_zag(data: &Vec<i32>) -> Vec<i32> {
        data.into_iter().map(|zig_zag_delta| DecodingUtils::decode_zig_zag(*zig_zag_delta)).collect()
    }

    pub fn decode_zig_zag_long(data: &Vec<i64>) -> Vec<i64> {
        data.into_iter().map(|zig_zag_delta| DecodingUtils::decode_zig_zag_long(*zig_zag_delta)).collect()
    }
}
