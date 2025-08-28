use crate::MltError;
use crate::metadata::stream::StreamMetadata;
use crate::metadata::stream_encoding::LogicalLevelTechnique;
use crate::vector::types::VectorType;

use zigzag::ZigZag;

/// Decode ([`ZigZag`] + delta) for Vec2s
// TODO: The encoded process is (delta + ZigZag) for each component
pub fn decode_componentwise_delta_vec2s<T: ZigZag>(data: &[T::UInt]) -> Result<Vec<T>, MltError> {
    let len = data.len();
    if len < 2 {
        return Err(MltError::MinLength {
            ctx: "vec2 delta stream",
            min: 2,
            got: len,
        });
    }
    if len % 2 != 0 {
        return Err(MltError::InvalidValueMultiple {
            ctx: "vec2 delta stream length",
            multiple_of: 2,
            got: len,
        });
    }

    let mut result = Vec::with_capacity(len);
    result.push(T::decode(data[0]));
    result.push(T::decode(data[1]));

    for i in (2..len).step_by(2) {
        result.push(T::decode(data[i]) + result[i - 2]);
        result.push(T::decode(data[i + 1]) + result[i - 1]);
    }

    Ok(result)
}

pub fn get_vector_type_int_stream(metadata: &StreamMetadata) -> VectorType {
    let tech1 = metadata.logical.technique1;
    let tech2 = metadata.logical.technique2;
    let runs = metadata.rle.as_ref().map(|r| r.runs);
    let n = metadata.num_values as usize;

    match (tech1, tech2, runs, n) {
        // L1 == RLE → runs == 1 → CONST; else FLAT
        (Some(LogicalLevelTechnique::Rle), _, Some(1), _) => VectorType::Const,
        (Some(LogicalLevelTechnique::Rle), _, Some(_), _) => VectorType::Flat,
        // L1 == DELTA && L2 == RLE && runs in {1,2} → SEQUENCE
        (Some(LogicalLevelTechnique::Delta), Some(LogicalLevelTechnique::Rle), Some(r), _)
            if r == 1 || r == 2 =>
        {
            VectorType::Sequence
        }
        // num_values == 1 → CONST; else FLAT
        (_, _, _, 1) => VectorType::Const,
        _ => VectorType::Flat,
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
    fn test_decode_componentwise_delta_vec2s() {
        // original Vec2s: [(3, 5), (7, 6), (12, 4)]
        // delta:          [3, 5, 4, 1, 5, -2]
        // ZigZag:         [6, 10, 8, 2, 10, 3]
        let encoded_from_positives: Vec<u32> = vec![6, 10, 8, 2, 10, 3];
        let decoded = decode_componentwise_delta_vec2s::<i32>(&encoded_from_positives).unwrap();
        assert_eq!(decoded, vec![3, 5, 7, 6, 12, 4]);

        // original Vec2s: [(3, 5), (-1, 6), (4, -4)]
        // delta:          [3, 5, -4, 1, 5, -10]
        // ZigZag:         [6, 10, 7, 2, 10, 19]
        let encoded_from_negatives: Vec<u32> = vec![6, 10, 7, 2, 10, 19];
        let decoded = decode_componentwise_delta_vec2s::<i32>(&encoded_from_negatives).unwrap();
        assert_eq!(decoded, vec![3, 5, -1, 6, 4, -4]);
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
}
