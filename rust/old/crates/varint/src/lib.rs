#![no_std]

const N1: i64 = 2_i64.pow( 7);
const N2: i64 = 2_i64.pow(14);
const N3: i64 = 2_i64.pow(21);
const N4: i64 = 2_i64.pow(28);
const N5: i64 = 2_i64.pow(35);
const N6: i64 = 2_i64.pow(42);
const N7: i64 = 2_i64.pow(49);
const N8: i64 = 2_i64.pow(56);
const N9: i64 = 2_i64.wrapping_pow(63);


const MSB: i64 = 0x80;
const REST: i64 = 0x7F;
const MSBALL: i64 = !REST;
const INT: i64 = 2_i64.pow(31);




pub struct VarInt {}
impl VarInt {
    pub fn decode_varint(src: &[u8], num_values: usize, dst: &mut [u32]) -> usize {
        if dst.len() < num_values {
            panic!("Destination buffer is too small");
        }

        let mut offset = 0;
        let mut dst_offset = 0;
        for _ in 0..num_values {
            offset = Self::_decode_varint(src, offset, dst, dst_offset);
            dst_offset += 1;
        }
        offset
    }

    fn _decode_varint(src: &[u8], offset: usize, dst: &mut [u32], dst_offset: usize) -> usize {
        let mut b = src[offset] as u32;
        let mut value = b & 0x7f;
        let mut new_offset = offset + 1;
        if (b & 0x80) == 0 {
            dst[dst_offset] = value;
            return new_offset;
        }

        b = src[new_offset] as u32;
        value |= (b & 0x7f) << 7;
        new_offset += 1;
        if (b & 0x80) == 0 {
            dst[dst_offset] = value;
            return new_offset;
        }

        b = src[new_offset] as u32;
        value |= (b & 0x7f) << 14;
        new_offset += 1;
        if (b & 0x80) == 0 {
            dst[dst_offset] = value;
            return new_offset;
        }

        b = src[new_offset] as u32;
        value |= (b & 0x7f) << 21;
        new_offset += 1;
        dst[dst_offset] = value;
        new_offset
    }
}
//     pub fn length(number: i64) -> u8 {
//         match number {
//             n if n < N1 => 1,
//             n if n < N2 => 2,
//             n if n < N3 => 3,
//             n if n < N4 => 4,
//             n if n < N5 => 5,
//             n if n < N6 => 6,
//             n if n < N7 => 7,
//             n if n < N8 => 8,
//             n if n < N9 => 9,
//             _ => 10,
//         }
//     }
// 
//     pub fn decode_single(buffer: &[u8]) -> u32 {
//         let mut shift_amount: u32 = 0;
//         let mut decoded_value: u32 = 0;
// 
//         for &byte in buffer {
//             decoded_value |= ((byte & 0b01111111) as u32) << shift_amount;
//             if byte & 0b10000000 != 0 {
//                 shift_amount += 7;
//             } else {
//                 break;
//             }
//         }
// 
//         decoded_value
//     }
// 
//     pub fn decode(buffer: &[u8], output: &mut [u32]) -> usize {
//         let mut shift_amount: u32 = 0;
//         let mut decoded_value: u32 = 0;
//         let mut output_offset = 0;
// 
//         for &byte in buffer {
//             decoded_value |= ((byte & 0b01111111) as u32) << shift_amount;
//             if byte & 0b10000000 != 0 {
//                 shift_amount += 7;
//             } else {
//                 if output_offset < output.len() {
//                     output[output_offset] = decoded_value;
//                     output_offset += 1;
//                 } else {
//                     panic!("[Varint] output buffer too small");
//                 }
//                 decoded_value = 0;
//                 shift_amount = 0;
//             }
//         }
// 
//         if shift_amount != 0 {
//             panic!("[Varint] decoding failed");
//         }
// 
//         output_offset
//     }
// 
//     pub fn decode_n(buffer: &[u8], output: &mut [u32], count: usize) -> usize {
//         let mut shift_amount: u32 = 0;
//         let mut decoded_value: u32 = 0;
//         let mut output_offset = 0;
// 
//         for &byte in buffer {
//             decoded_value |= ((byte & 0b01111111) as u32) << shift_amount;
//             if byte & 0b10000000 != 0 {
//                 shift_amount += 7;
//             } else {
//                 if output_offset < output.len() && output_offset < count {
//                     output[output_offset] = decoded_value;
//                     output_offset += 1;
//                 } else {
//                     break;
//                 }
//                 decoded_value = 0;
//                 shift_amount = 0;
//             }
//         }
// 
//         if shift_amount != 0 {
//             panic!("[Varint] decoding failed");
//         }
// 
//         if output_offset < count {
//             panic!("[Varint] not enough varints in buffer");
//         }
// 
//         output_offset
//     }
// }
