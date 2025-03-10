use crate::decoding::fastpfor_decoder::FastPFOR;

pub struct DecodingUtils {}

impl DecodingUtils {
    // todo: refine functions to be closer in api (no return)
    pub fn decode_componentwise_delta_vec2(data: &mut [i32]) {
        data[0] = (data[0] >> 1) ^ ((data[0] << 31) >> 31);
        data[1] = (data[1] >> 1) ^ ((data[1] << 31) >> 31);
        let sz0 = (data.len() / 4) * 4;
        let mut i = 2;
        if sz0 >= 4 {
            while i < sz0 - 4 {
                let x1 = data[i];
                let y1 = data[i + 1];
                let x2 = data[i + 2];
                let y2 = data[i + 3];
                
                data[i + 0] = (x1.wrapping_shr(1) ^ (x1.wrapping_shl(31).wrapping_shr(31))).wrapping_add(data[i - 2]);
                data[i + 1] = (y1.wrapping_shr(1) ^ (y1.wrapping_shl(31).wrapping_shr(31))).wrapping_add(data[i - 1]);
                data[i + 2] = (x2.wrapping_shr(1) ^ (x2.wrapping_shl(31).wrapping_shr(31))).wrapping_add(data[i + 0]);
                data[i + 3] = (y2.wrapping_shr(1) ^ (y2.wrapping_shl(31).wrapping_shr(31))).wrapping_add(data[i + 1]);
                
                i += 4;
            }
        }

        while i != data.len() {
            data[i] = ((data[i] >> 1) ^ ((data[i] << 31) >> 31)) + data[i - 2];
            data[i + 1] = ((data[i + 1] >> 1) ^ ((data[i + 1] << 31) >> 31)) + data[i - 1];
            i += 2;
        }
    }
    
    /// Decodes multiple variable-length integers (varints) from a byte slice
    /// 
    /// # Arguments
    /// * `src` - Source byte slice containing encoded varints
    /// * `pos` - Current position in the byte slice, updated as bytes are read
    /// * `num_values` - Number of varint values to decode
    /// 
    /// # Returns
    /// Vector of decoded u32 values
    pub fn decode_varint(src: &[u8], pos: &mut usize, num_values: usize) -> Vec<u32> {
        let mut values = vec![0; num_values];
        for i in 0..num_values {
            Self::decode_varint_internal(src, pos, &mut values, i);
        }
        values
    }

    /// Decodes a variable-length integer (varint) from the source buffer into the destination array
    ///
    /// # Arguments
    /// * `src` - Source byte slice containing the encoded varint
    /// * `offset` - Current position in the source buffer, updated as bytes are read
    /// * `dst` - Destination array to store the decoded u32 value
    /// * `dst_offset` - Position in the destination array to store the result
    ///
    /// This implementation handles varints up to 4 bytes long, decoding them into u32 values.
    /// The encoding uses 7 bits per byte with the high bit (0x80) indicating if more bytes follow.
    pub fn decode_varint_internal(src: &[u8], offset: &mut usize, dst: &mut [u32], dst_offset: usize) {
        let mut b = src[*offset] as u32;
        *offset += 1;
        let mut value = b & 0x7f;
        if (b & 0x80) == 0 {
            dst[dst_offset] = value;
            return;
        }

        b = src[*offset] as u32;
        *offset += 1;
        value |= (b & 0x7f).wrapping_shl(7);
        if b & 0x80 == 0 {
            dst[dst_offset] = value;
            return;
        }

        b = src[*offset] as u32;
        *offset += 1;
        value |= (b & 0x7f).wrapping_shl(14);
        if (b & 0x80) == 0 {
            dst[dst_offset] = value;
            return;
        }

        b = src[*offset] as u32;
        *offset += 1;
        value |= (b & 0x7f).wrapping_shl(21);
        dst[dst_offset] = value;
    }
    
    pub fn decode_long_varint(src: &[u8], offset: &mut usize, num_values: usize) -> Vec<u64> {
        let mut values = vec![0; num_values];
        for i in 0..num_values {
            let value = Self::decode_long_varint_internal(src, offset);
            values[i] = value;
        }
        values
    }

    pub fn decode_long_varint_internal(bytes: &[u8], offset: &mut usize) -> u64 {
        let mut value = 0;
        let mut shift = 0;
        let mut index = *offset;
        while index < bytes.len() {
            let b = bytes[index];
            value |= (b as u64 & 0x7F) << shift;
            index += 1;
            if (b & 0x80) == 0 {
                break;
            }
            shift += 7;
            if shift >= 64 {
                panic!("Varint too long");
            }
        }
        *offset = index;
        value
    }

    pub fn decode_zig_zag(encoded: i32) -> i32 {
        (encoded >> 1) ^ ((encoded & 1) * -1)
    }

    pub fn decode_zig_zag_array(encoded: &mut [i32]) {
        for i in 0..encoded.len() {
            encoded[i] = Self::decode_zig_zag(encoded[i]);
        }
    }

    pub fn decode_zig_zag_long(encoded: i64) -> i64 {
        (encoded >> 1) ^ ((encoded & 1) * -1)
    }

    pub fn decode_zig_zag_long_array(encoded: &mut [i64]) {
        for i in 0..encoded.len() {
            encoded[i] = Self::decode_zig_zag_long(encoded[i]);
        }
    }

    /// Decodes a byte sequence encoded with run-length encoding (RLE).
    ///
    /// # Arguments
    /// * `buffer` - The encoded byte slice
    /// * `num_bytes` - Expected number of bytes after decoding
    /// * `pos` - Current position in the buffer, updated as bytes are read
    ///
    /// # Example
    /// ```
    /// use maplibre_tile_spec::decoding::decoding_utils::DecodingUtils;
    /// 
    /// let encoded = vec![0x01, 0x41, 0xff, 0x20, 0x00, 0x42];
    /// let decoded = DecodingUtils::decode_byte_rle(&encoded, 8, &mut 0);
    /// assert_eq!(decoded, b"AAAA BBB");
    /// ```
    pub fn decode_byte_rle(buffer: &[u8], num_bytes: usize, pos: &mut usize) -> Vec<u8> {
        let mut values = Vec::with_capacity(num_bytes);

        while values.len() < num_bytes {
            let header = buffer[*pos];
            *pos += 1;

            /* Runs */
            if header <= 0x7f {
                let num_runs = header as usize + 3;
                let value = buffer[*pos];
                *pos += 1;
                for _ in 0..num_runs { values.push(value); }
            } else {
                /* Literals */
                let num_literals = 256 - header as usize;
                for _ in 0..num_literals {
                    values.push(buffer[*pos]);
                    *pos += 1;
                }
            }
        }

        values
    }

    // let mut values = vec![0u8; num_bytes];
    // let mut value_offset = 0;
    //
    // while value_offset < num_bytes {
    //     let header = buffer[*pos];
    //     *pos += 1;
    //
    //     /* Runs */
    //     if header <= 0x7f {
    //         let num_runs = header as usize + 3;
    //         let value = buffer[*pos];
    //         *pos += 1;
    //         let end_value_offset = value_offset + num_runs;
    //         values[value_offset..end_value_offset].fill(value);
    //         value_offset = end_value_offset;
    //     } else {
    //         /* Literals */
    //         let num_literals = 256 - header as usize;
    //         for _ in 0..num_literals {
    //             values[value_offset] = buffer[*pos];
    //             *pos += 1;
    //             value_offset += 1;
    //         }
    //     }
    // }
    // values
    // }

    pub fn decode_unsigned_rle(data: &[u32], num_runs: usize, num_total_values: usize) -> Vec<u32> {
        let mut values = vec![0; num_total_values];
        let mut offset = 0;
        for i in 0..num_runs {
            let run_length = data[i] as usize;
            let value = data[i + num_runs];
            values[offset..offset + run_length].fill(value);
            offset += run_length;
        }
        values
    }

    pub fn decode_unsigned_rle_long(data: &[u64], num_runs: usize, num_total_values: usize) -> Vec<u64> {
        let mut values = vec![0; num_total_values];
        let mut offset = 0;
        for i in 0..num_runs {
            let run_length = data[i] as usize;
            let value = data[i + num_runs];
            values[offset..offset + run_length].fill(value);
            offset += run_length;
        }
        values
    }

    /// Decodes a run-length encoded sequence of booleans from a byte buffer.
    /// 
    /// Each boolean value is packed into bits (8 per byte) and then run-length encoded.
    /// This function first decodes the RLE bytes and then unpacks the bits into boolean values.
    ///
    /// # Arguments
    /// * `buffer` - The encoded byte buffer to read from
    /// * `num_booleans` - The number of boolean values to decode
    /// * `pos` - Current position in the buffer, updated as bytes are read
    ///
    /// # Returns
    /// A vector containing the decoded boolean values
    ///
    /// # Example
    /// ```
    /// use maplibre_tile_spec::decoding::decoding_utils::DecodingUtils
    /// 
    /// let raw = vec![0x01, b'A', 0xff, b' ', 0x00, b'B'];
    /// let decoded = DecodingUtils::decode_byte_rle(&raw, 8, &mut 0);
    /// assert_eq!(decoded, b"AAAA BBB");
    /// ```
    pub fn decode_boolean_rle(buffer: &[u8], num_booleans: usize, pos: &mut usize) -> Vec<bool> {
        let num_bytes = (num_booleans as f64 / 8.0).ceil() as usize;
        let decoded_bytes = Self::decode_byte_rle(buffer, num_bytes, pos);
        let mut result = Vec::with_capacity(num_booleans);

        for b in decoded_bytes.iter() {
            for j in 0..8 {
                let bit = (b & (1 << j)) != 0;
                result.push(bit);
            }
        }

        result.to_vec()
    }

    pub fn decode_floats_le(encoded_values: &[u8], pos: &mut usize, num_values: usize) -> Vec<f32> {
        let mut values = vec![0.0; num_values];
        for i in 0..num_values {
            let bytes = &encoded_values[*pos..*pos + 4];
            values[i] = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            *pos += 4;
        }
        values
    }

    pub fn decode_doubles_le(encoded_values: &[u8], pos: &mut usize, num_values: usize) -> Vec<f64> {
        let mut values = vec![0.0; num_values];
        for i in 0..num_values {
            let bytes = &encoded_values[*pos..*pos + 8];
            values[i] = f64::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]]);
            *pos += 8;
        }
        values
    }

    pub fn decode_fastpfor(data: &[u8], offset: &mut usize, num_values: u32, byte_length: u32) -> Vec<u32> {
        let encoded_slice = &data[*offset..*offset + byte_length as usize];

        let mut int_values = vec![0; byte_length.div_ceil(4) as usize];
        for i in (0..int_values.len()).step_by(4) {
            let value =
                (encoded_slice[i + 0] as u32)
                    | (encoded_slice[i + 1] as u32) << 8
                    | (encoded_slice[i + 2] as u32) << 16
                    | (encoded_slice[i + 3] as u32) << 24;
            int_values.push(value);
        }

        let mut decoded_values = vec![0; num_values as usize];
        let mut core = FastPFOR::default();
        core.decode(&int_values, &mut decoded_values);

        *offset += byte_length as usize;
        decoded_values
    }

    pub fn decode_fastpfor_delta_coordinates(data: &[u8], offset: &mut usize, num_values: u32, byte_length: u32) -> Vec<u32> {
        let encoded_slice = &data[*offset..*offset + byte_length as usize];

        let mut int_values = vec![0; byte_length.div_ceil(4) as usize];
        for i in (0..int_values.len()).step_by(4) {
            let value =
                (encoded_slice[i + 0] as u32)
                    | (encoded_slice[i + 1] as u32) << 8
                    | (encoded_slice[i + 2] as u32) << 16
                    | (encoded_slice[i + 3] as u32) << 24;
            int_values.push(value);
        }

        let mut decompressed_value = vec![0; num_values as usize];
        let mut core = FastPFOR::default();
        core.decode(&int_values, &mut decompressed_value);

        let mut decoded_value = Vec::with_capacity(num_values as usize);
        for i in 0..num_values {
            let zig_zag_value = decompressed_value[i as usize];
            decoded_value.push((zig_zag_value >> 1) ^ ((zig_zag_value & 1) as i32 * -1) as u32);
        }

        *offset += byte_length as usize;

        let mut values = Vec::with_capacity(num_values as usize);
        let mut previous_value_x = 0;
        let mut previous_value_y = 0;
        for i in (0..num_values).step_by(2) {
            let delta_x = decoded_value[i as usize];
            let delta_y = decoded_value[(i + 1) as usize];
            let x = previous_value_x + delta_x;
            let y = previous_value_y + delta_y;

            values.push(x);
            values.push(y);

            previous_value_x = x;
            previous_value_y = y;
        }

        values
    }
}
