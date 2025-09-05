use crate::metadata::stream::StreamMetadata;
use crate::metadata::stream_encoding::LogicalLevelTechnique;
use crate::vector::types::VectorType;

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
