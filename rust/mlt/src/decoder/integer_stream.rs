use num_traits::{PrimInt, Unsigned};
use zigzag::ZigZag;

use crate::decoder::integer::decode_physical;
use crate::decoder::tracked_bytes::TrackedBytes;
use crate::metadata::stream::StreamMetadata;
use crate::metadata::stream_encoding::LogicalLevelTechnique;
use crate::vector::types::VectorType;
use crate::{MltError, MltResult};

/// Decode ([`ZigZag`] + delta) for Vec2s
// TODO: The encoded process is (delta + ZigZag) for each component
pub fn decode_componentwise_delta_vec2s<T: ZigZag>(data: &[T::UInt]) -> MltResult<Vec<T>> {
    if data.is_empty() || !data.len().is_multiple_of(2) {
        return Err(MltError::InvalidPairStreamSize(data.len()));
    }

    let mut result = Vec::with_capacity(data.len());
    let mut last1 = T::zero();
    let mut last2 = T::zero();

    for i in (0..data.len()).step_by(2) {
        last1 = T::decode(data[i]) + last1;
        last2 = T::decode(data[i + 1]) + last2;
        result.push(last1);
        result.push(last2);
    }

    Ok(result)
}

pub fn get_vector_type_int_stream(metadata: &StreamMetadata) -> VectorType {
    match (
        metadata.logical.technique1,
        metadata.rle.as_ref().map(|r| r.runs),
        metadata.num_values,
    ) {
        // L1 == RLE → runs == 1 → CONST; else FLAT
        (Some(LogicalLevelTechnique::Rle), Some(1), _) => VectorType::Const,
        (Some(LogicalLevelTechnique::Rle), Some(_), _) => VectorType::Flat,
        // L1 == DELTA && L2 == RLE && runs in {1,2} → SEQUENCE
        (Some(LogicalLevelTechnique::Delta), Some(1 | 2), _)
            if metadata.logical.technique2 == Some(LogicalLevelTechnique::Rle) =>
        {
            VectorType::Sequence
        }
        // num_values == 1 → CONST; else FLAT
        (_, _, 1) => VectorType::Const,
        _ => VectorType::Flat,
    }
}

pub fn decode_zigzag_const_rle<T: ZigZag>(data: &[T::UInt]) -> MltResult<T> {
    Ok(T::decode(*data.get(1).ok_or(MltError::MinLength {
        ctx: "zigzag const RLE stream",
        min: 2,
        got: data.len(),
    })?))
}

pub fn decode_unsigned_const_rle<T: PrimInt + Unsigned>(data: &[T]) -> MltResult<T> {
    Ok(*data.get(1).ok_or(MltError::MinLength {
        ctx: "unsigned const RLE stream",
        min: 2,
        got: data.len(),
    })?)
}

pub fn decode_const_int_stream_signed(
    tile: &mut TrackedBytes,
    metadata: &StreamMetadata,
) -> MltResult<i32> {
    match decode_physical(tile, metadata)?.as_slice() {
        [v] => Ok(i32::decode(*v)),
        values => decode_zigzag_const_rle::<i32>(values),
    }
}

pub fn decode_const_int_stream_unsigned(
    tile: &mut TrackedBytes,
    metadata: &StreamMetadata,
) -> MltResult<u32> {
    match decode_physical(tile, metadata)?.as_slice() {
        [v] => Ok(*v),
        values => decode_unsigned_const_rle::<u32>(values),
    }
}

/// Extract sequence parameters from ZigZag-encoded RLE data
/// Returns (base, delta) for generating arithmetic sequences
pub fn decode_sequence_int_stream(
    tile: &mut TrackedBytes,
    metadata: &StreamMetadata,
) -> MltResult<(i32, i32)> {
    let values = decode_physical(tile, metadata)?;
    decode_zigzag_sequence_rle::<i32>(&values)
}

fn decode_zigzag_sequence_rle<T: ZigZag>(data: &[T::UInt]) -> MltResult<(T, T)> {
    if data.len() < 2 {
        return Err(MltError::MinLength {
            ctx: "zigzag sequence RLE stream",
            min: 2,
            got: data.len(),
        });
    }
    if data.len() == 2 {
        let value = T::decode(data[1]);
        Ok((value, value))
    } else {
        Ok((T::decode(data[1]), T::decode(data[3])))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::stream::{Rle, StreamMetadata};
    use crate::metadata::stream_encoding::{
        Logical, LogicalLevelTechnique, LogicalStreamType, Physical, PhysicalLevelTechnique,
        PhysicalStreamType,
    };

    #[test]
    fn test_decode_zigzag_sequence_rle() {
        let encoded: Vec<u32> = vec![1, 200, 4, 2];
        let decoded = decode_zigzag_sequence_rle::<i32>(&encoded).unwrap();
        assert_eq!(decoded, (100, 1));

        let encoded: Vec<u32> = vec![4, 200];
        let decoded = decode_zigzag_sequence_rle::<i32>(&encoded).unwrap();
        assert_eq!(decoded, (100, 100));

        let encoded: Vec<u32> = vec![4];
        let decoded = decode_zigzag_sequence_rle::<i32>(&encoded);
        assert!(decoded.is_err());

        let encoded: Vec<u32> = vec![];
        let decoded = decode_zigzag_sequence_rle::<i32>(&encoded);
        assert!(decoded.is_err());
    }

    fn generate_metadata(
        t1: LogicalLevelTechnique,
        t2: LogicalLevelTechnique,
        runs: Option<u32>,
        num_values: u32,
    ) -> StreamMetadata {
        StreamMetadata {
            logical: Logical::new(Some(LogicalStreamType::Dictionary(None)), t1, t2),
            physical: Physical::new(PhysicalStreamType::Present, PhysicalLevelTechnique::Varint),
            num_values,
            byte_length: 0,
            morton: None,
            rle: runs.map(|r| Rle {
                runs: r,
                num_rle_values: r * 2,
            }),
        }
    }

    #[test]
    fn table_driven_vector_type_int_stream() {
        let cases = vec![
            (
                "RLE runs = 1 → CONST",
                generate_metadata(
                    LogicalLevelTechnique::Rle,
                    LogicalLevelTechnique::Delta,
                    Some(1),
                    10,
                ),
                VectorType::Const,
            ),
            (
                "Delta + RLE runs = 2 → SEQUENCE",
                generate_metadata(
                    LogicalLevelTechnique::Delta,
                    LogicalLevelTechnique::Rle,
                    Some(2),
                    8,
                ),
                VectorType::Sequence,
            ),
            (
                "Fallback: num_values == 1 → CONST",
                generate_metadata(
                    LogicalLevelTechnique::Delta,
                    LogicalLevelTechnique::Delta,
                    None,
                    1,
                ),
                VectorType::Const,
            ),
            (
                "Default: no special case, num_values > 1 → FLAT",
                generate_metadata(
                    LogicalLevelTechnique::Delta,
                    LogicalLevelTechnique::Delta,
                    None,
                    5,
                ),
                VectorType::Flat,
            ),
        ];

        for (desc, meta, expected) in cases {
            let vt = get_vector_type_int_stream(&meta);
            assert_eq!(vt, expected, "case failed: {desc}");
        }
    }

    #[test]
    fn test_decode_componentwise_delta_vec2s() {
        let cases: &[(&[u32], &[i32])] = &[
            // original Vec2s: [(3, 5), (7, 6), (12, 4)]
            // delta:          [3, 5, 4, 1, 5, -2]
            // ZigZag:         [6, 10, 8, 2, 10, 3]
            (&[6, 10, 8, 2, 10, 3], &[3, 5, 7, 6, 12, 4]),
            // original Vec2s: [(3, 5), (-1, 6), (4, -4)]
            // delta:          [3, 5, -4, 1, 5, -10]
            // ZigZag:         [6, 10, 7, 2, 10, 19]
            (&[6, 10, 7, 2, 10, 19], &[3, 5, -1, 6, 4, -4]),
            (&[10, 14, 3, 9], &[5, 7, 3, 2]),
            (&[6, 12, 10, 12, 24, 44], &[3, 6, 8, 12, 20, 34]),
            (
                &[0, 0, 8192, 0, 0, 8192, 8191, 0],
                &[0, 0, 4096, 0, 4096, 4096, 0, 4096],
            ),
            (
                &[
                    1416, 520, 1888, 6448, 2927, 1136, 224, 47, 5920, 4671, 752, 351, 1999, 1423,
                    447, 671, 1184, 1792, 143, 351, 623, 320, 95, 1055, 976, 880, 1407, 1471, 3983,
                    336, 703, 80, 1680, 559, 15, 1120, 1279, 848, 1312, 1280, 1055, 528, 511, 976,
                    1072, 175, 1072, 1423, 976, 352, 463, 416, 2527, 2896, 2192, 1167,
                ],
                &[
                    708, 260, 1652, 3484, 188, 4052, 300, 4028, 3260, 1692, 3636, 1516, 2636, 804,
                    2412, 468, 3004, 1364, 2932, 1188, 2620, 1348, 2572, 820, 3060, 1260, 2356,
                    524, 364, 692, 12, 732, 852, 452, 844, 1012, 204, 1436, 860, 2076, 332, 2340,
                    76, 2828, 612, 2740, 1148, 2028, 1636, 2204, 1404, 2412, 140, 3860, 1236, 3276,
                ],
            ),
            (
                &[
                    558, 7970, 72, 13, 3766, 6579, 100, 34, 8, 90, 134, 78, 92, 33, 76, 0, 28, 25,
                    84, 22, 52, 13, 12, 41, 80, 9, 50, 34, 44, 1, 2, 79, 50, 21, 28, 47, 30, 17, 4,
                    25, 58, 53, 90, 7, 48, 14, 96, 19, 18, 20, 118, 11, 46, 49, 12, 71, 16, 11,
                    711, 1277, 15, 32, 15, 8, 13, 34, 13, 4, 25, 26, 17, 2, 1, 8, 8, 12, 5, 8, 5,
                    1, 0, 16, 9, 2, 0, 14, 7, 4, 0, 6, 5, 6, 0, 4, 9, 1, 9, 10, 14, 0, 0, 4, 7, 6,
                    9, 3, 3, 8, 17, 11, 1, 4, 11, 3, 1, 10, 19, 12, 3, 3, 2, 3, 15, 3, 0, 8, 19, 2,
                    25, 32, 18, 8, 0, 8, 37, 3, 9, 8, 5, 18, 19, 0, 5, 6, 3, 18, 13, 0, 0, 20, 7,
                    0, 7, 9, 13, 6, 11, 7, 9, 2, 0, 12, 7, 6, 17, 2, 45, 28, 19, 26, 10, 14, 7, 4,
                    11, 5, 19, 8, 13, 11, 11, 20, 23, 20, 13, 4, 13, 32, 37, 16, 17, 24, 23, 18, 5,
                    12, 33, 26, 13, 20, 4, 36, 49, 38, 17, 46, 1390, 1536, 15, 1679, 2447, 7360,
                ],
                &[
                    279, 3985, 315, 3978, 2198, 688, 2248, 705, 2252, 750, 2319, 789, 2365, 772,
                    2403, 772, 2417, 759, 2459, 770, 2485, 763, 2491, 742, 2531, 737, 2556, 754,
                    2578, 753, 2579, 713, 2604, 702, 2618, 678, 2633, 669, 2635, 656, 2664, 629,
                    2709, 625, 2733, 632, 2781, 622, 2790, 632, 2849, 626, 2872, 601, 2878, 565,
                    2886, 559, 2530, -80, 2522, -64, 2514, -60, 2507, -43, 2500, -41, 2487, -28,
                    2478, -27, 2477, -23, 2481, -17, 2478, -13, 2475, -14, 2475, -6, 2470, -5,
                    2470, 2, 2466, 4, 2466, 7, 2463, 10, 2463, 12, 2458, 11, 2453, 16, 2460, 16,
                    2460, 18, 2456, 21, 2451, 19, 2449, 23, 2440, 17, 2439, 19, 2433, 17, 2432, 22,
                    2422, 28, 2420, 26, 2421, 24, 2413, 22, 2413, 26, 2403, 27, 2390, 43, 2399, 47,
                    2399, 51, 2380, 49, 2375, 53, 2372, 62, 2362, 62, 2359, 65, 2357, 74, 2350, 74,
                    2350, 84, 2346, 84, 2342, 79, 2335, 82, 2329, 78, 2324, 79, 2324, 85, 2320, 88,
                    2311, 89, 2288, 103, 2278, 116, 2283, 123, 2279, 125, 2273, 122, 2263, 126,
                    2256, 120, 2250, 130, 2238, 140, 2231, 142, 2224, 158, 2205, 166, 2196, 178,
                    2184, 187, 2181, 193, 2164, 206, 2157, 216, 2159, 234, 2134, 253, 2125, 276,
                    2820, 1044, 2812, 204, 1588, 3884,
                ],
            ),
        ];

        for (input, expected) in cases {
            let output = decode_componentwise_delta_vec2s::<i32>(input).unwrap();
            assert_eq!(&output, expected);
        }
    }

    #[test]
    fn test_decode_zigzag_const_rle() {
        let encoded: Vec<u32> = vec![0, 10];
        let decoded = decode_zigzag_const_rle::<i32>(&encoded).unwrap();
        assert_eq!(decoded, 5);

        let encoded_neg: Vec<u32> = vec![0, 11];
        let decoded_neg = decode_zigzag_const_rle::<i32>(&encoded_neg).unwrap();
        assert_eq!(decoded_neg, -6);

        let encoded_extra: Vec<u32> = vec![0, 10, 20, 30];
        let decoded_extra = decode_zigzag_const_rle::<i32>(&encoded_extra).unwrap();
        assert_eq!(decoded_extra, 5);

        let encoded_single: Vec<u32> = vec![0];
        let decoded_single = decode_zigzag_const_rle::<i32>(&encoded_single);
        assert!(decoded_single.is_err());

        let encoded_empty: Vec<u32> = vec![];
        let decoded_empty = decode_zigzag_const_rle::<i32>(&encoded_empty);
        assert!(decoded_empty.is_err());
    }

    #[test]
    fn test_decode_unsigned_const_rle() {
        let encoded: Vec<u32> = vec![0, 10];
        let decoded = decode_unsigned_const_rle::<u32>(&encoded).unwrap();
        assert_eq!(decoded, 10);

        let encoded_extra: Vec<u32> = vec![0, 10, 20, 30];
        let decoded_extra = decode_unsigned_const_rle::<u32>(&encoded_extra).unwrap();
        assert_eq!(decoded_extra, 10);

        let encoded_single: Vec<u32> = vec![0];
        let decoded_single = decode_unsigned_const_rle::<u32>(&encoded_single);
        assert!(decoded_single.is_err());

        let encoded_empty: Vec<u32> = vec![];
        let decoded_empty = decode_unsigned_const_rle::<u32>(&encoded_empty);
        assert!(decoded_empty.is_err());
    }

    #[test]
    fn test_decode_const_int_stream_signed() {
        // Single value, Varint bytes: [0x01] → values = [1] → ZigZag(1) = -1
        let mut single_bytes: TrackedBytes = vec![0x01u8].into();
        let single_meta = generate_metadata(
            LogicalLevelTechnique::Delta,
            LogicalLevelTechnique::Delta,
            None,
            1,
        );
        let decoded_single =
            decode_const_int_stream_signed(&mut single_bytes, &single_meta).unwrap();
        assert_eq!(decoded_single, -1);

        // RLE-const, Varint bytes: [0x00, 0x0A] → values = [0, 10]
        // decode_zigzag_const_rle takes index 1 → 10 → ZigZag(10) = 5
        let mut rle_bytes: TrackedBytes = vec![0x00u8, 0x0A].into();
        let rle_meta = generate_metadata(
            LogicalLevelTechnique::Delta,
            LogicalLevelTechnique::Delta,
            None,
            2,
        );
        let decoded_rle = decode_const_int_stream_signed(&mut rle_bytes, &rle_meta).unwrap();
        assert_eq!(decoded_rle, 5);
    }

    #[test]
    fn test_decode_const_int_stream_unsigned() {
        // Single value, Varint bytes: [0x02] → values = [2]
        let mut single_bytes: TrackedBytes = vec![0x02u8].into();
        let single_meta = generate_metadata(
            LogicalLevelTechnique::None,
            LogicalLevelTechnique::None,
            None,
            1,
        );
        let decoded_single =
            decode_const_int_stream_unsigned(&mut single_bytes, &single_meta).unwrap();
        assert_eq!(decoded_single, 2);

        // RLE-const, Varint bytes: [0x00, 0x0A] → values = [0, 10]
        // decode_unsigned_const_rle takes index 1 → 10
        let mut rle_bytes: TrackedBytes = vec![0x00u8, 0x0A].into();
        let rle_meta = generate_metadata(
            LogicalLevelTechnique::None,
            LogicalLevelTechnique::None,
            None,
            2,
        );
        let decoded_rle = decode_const_int_stream_unsigned(&mut rle_bytes, &rle_meta).unwrap();
        assert_eq!(decoded_rle, 10);
    }
}
