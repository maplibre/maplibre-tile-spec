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
        (Some(LogicalLevelTechnique::Delta), Some(r), _)
            if metadata.logical.technique2 == Some(LogicalLevelTechnique::Rle)
                && (r == 1 || r == 2) =>
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::stream::{Rle, StreamMetadata};
    use crate::metadata::stream_encoding::{
        Logical, LogicalLevelTechnique, LogicalStreamType, Physical, PhysicalLevelTechnique,
        PhysicalStreamType,
    };

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
