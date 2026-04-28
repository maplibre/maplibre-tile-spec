use std::fmt::Debug;

use num_traits::PrimInt;

use crate::MltResult;
use crate::codecs::rle::encode_rle;
use crate::decoder::RleMeta;

/// RLE-encode `data` into `target` and return the matching `RleMeta`.
///
/// `target` is treated as a scratch buffer: cleared before writing.
/// `num_logical` is the expanded output length (stored in `RleMeta::num_rle_values`).
pub(crate) fn apply_rle<T: PrimInt + Debug>(
    data: &[T],
    num_logical: usize,
    target: &mut Vec<T>,
) -> MltResult<RleMeta> {
    let (runs_vec, vals_vec) = encode_rle(data);
    let meta = RleMeta {
        runs: u32::try_from(runs_vec.len())?,
        num_rle_values: u32::try_from(num_logical)?,
    };
    target.clear();
    target.extend_from_slice(&runs_vec);
    target.extend_from_slice(&vals_vec);
    Ok(meta)
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, Default, strum::EnumIter)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum LogicalEncoder {
    #[default]
    None,
    Delta,
    DeltaRle,
    Rle,
    // FIXME: add more of the LogicalEncoding strategies
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::decoder::{
        DictionaryType, LogicalEncoding, LogicalValue, PhysicalEncoding, StreamMeta, StreamType,
    };
    use crate::encoder::Codecs;
    use crate::test_helpers::dec;

    pub fn make_meta(logical_encoding: LogicalEncoding, num_values: usize) -> StreamMeta {
        StreamMeta::new2(
            StreamType::Data(DictionaryType::None),
            logical_encoding,
            PhysicalEncoding::None,
            num_values,
        )
        .expect("proptest to not generate that large of a vec")
    }

    proptest! {
        #[test]
        fn test_u32_logical_roundtrip(
            values in prop::collection::vec(any::<u32>(), 0..100),
            logical in any::<LogicalEncoder>(),
        ) {
            let mut codecs = Codecs::default();
            let (computed, encoded) = codecs.logical.encode_u32(&values, logical).unwrap();
            let meta = make_meta(computed, values.len());
            let decoded = LogicalValue::new(meta).decode_u32(encoded, &mut dec()).unwrap();
            prop_assert_eq!(decoded, values);
        }

        #[test]
        fn test_i32_logical_roundtrip(
            values in prop::collection::vec(any::<i32>(), 0..100),
            logical in any::<LogicalEncoder>(),
        ) {
            let mut codecs = Codecs::default();
            let (computed, encoded) = codecs.logical.encode_i32(&values, logical).unwrap();
            let meta = make_meta(computed, values.len());
            let decoded = LogicalValue::new(meta).decode_i32(encoded, &mut dec()).unwrap();
            prop_assert_eq!(decoded, values);
        }

        #[test]
        fn test_u64_logical_roundtrip(
            values in prop::collection::vec(any::<u64>(), 0..100),
            logical in any::<LogicalEncoder>(),
        ) {
            let mut codecs = Codecs::default();
            let (computed, encoded) = codecs.logical.encode_u64(&values, logical).unwrap();
            let meta = make_meta(computed, values.len());
            let decoded = LogicalValue::new(meta).decode_u64(encoded, &mut dec()).unwrap();
            prop_assert_eq!(decoded, values);
        }

        #[test]
        fn test_i64_logical_roundtrip(
            values in prop::collection::vec(any::<i64>(), 0..100),
            logical in any::<LogicalEncoder>(),
        ) {
            let mut codecs = Codecs::default();
            let (computed, encoded) = codecs.logical.encode_i64(&values, logical).unwrap();
            let meta = make_meta(computed, values.len());
            let decoded = LogicalValue::new(meta).decode_i64(encoded, &mut dec()).unwrap();
            prop_assert_eq!(decoded, values);
        }
    }
}
