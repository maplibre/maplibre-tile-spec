use std::fmt::Debug;
use std::mem;

use bytemuck;
use num_traits::PrimInt;

use crate::MltResult;
use crate::codecs::rle::encode_rle;
use crate::codecs::zigzag::{encode_zigzag, encode_zigzag_delta};
use crate::decoder::{LogicalEncoding, RleMeta};

/// RLE-encode `data` into `target` and return the matching `RleMeta`.
///
/// `target` is treated as a scratch buffer: cleared before writing.
/// `num_logical` is the expanded output length (stored in `RleMeta::num_rle_values`).
fn apply_rle<T: PrimInt + Debug>(
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
impl LogicalEncoder {
    /// Logically encode `u32` values into `target`.
    ///
    /// `target` is treated as a scratch buffer: it is cleared before writing.
    /// After the call, `target` holds the physically-stored sequence.
    ///
    /// `scratch` is a reusable intermediate buffer for two-pass encodings
    /// (Rle, `DeltaRle`). Passing a long-lived buffer avoids a fresh allocation
    /// per call.
    ///
    /// See [`crate::decoder::LogicalValue::decode_u32`] for the reverse operation.
    #[hotpath::measure]
    pub fn encode_u32s(
        self,
        values: &[u32],
        target: &mut Vec<u32>,
        scratch: &mut Vec<u32>,
    ) -> MltResult<LogicalEncoding> {
        match self {
            Self::None => {
                // FIXME: avoid this copying - just use source
                target.clear();
                target.extend_from_slice(values);
                Ok(LogicalEncoding::None)
            }
            Self::Delta => {
                encode_zigzag_delta(bytemuck::cast_slice::<u32, i32>(values), target);
                Ok(LogicalEncoding::Delta)
            }
            Self::Rle => {
                let meta = apply_rle(values, values.len(), target)?;
                Ok(LogicalEncoding::Rle(meta))
            }
            Self::DeltaRle => {
                encode_zigzag_delta(bytemuck::cast_slice::<u32, i32>(values), scratch);
                let meta = apply_rle(scratch, values.len(), target)?;
                Ok(LogicalEncoding::DeltaRle(meta))
            }
        }
    }

    /// Logically encode `i32` values into `target` (u32 physical representation).
    ///
    /// `target` is treated as a scratch buffer: it is cleared before writing.
    /// After the call, `target` holds the physically-stored sequence.
    ///
    /// `scratch` is a reusable intermediate buffer for two-pass encodings
    /// (Rle, `DeltaRle`). Passing a long-lived buffer avoids a fresh allocation
    /// per call.
    ///
    /// See [`crate::decoder::LogicalValue::decode_i32`] for the reverse operation.
    #[hotpath::measure]
    pub fn encode_i32s(
        self,
        values: &[i32],
        target: &mut Vec<u32>,
        scratch: &mut Vec<u32>,
    ) -> MltResult<LogicalEncoding> {
        match self {
            Self::None => {
                encode_zigzag(values, target);
                Ok(LogicalEncoding::None)
            }
            Self::Delta => {
                encode_zigzag_delta(values, target);
                Ok(LogicalEncoding::Delta)
            }
            Self::Rle => {
                encode_zigzag(values, scratch);
                let meta = apply_rle(scratch, values.len(), target)?;
                Ok(LogicalEncoding::Rle(meta))
            }
            Self::DeltaRle => {
                encode_zigzag_delta(values, scratch);
                let meta = apply_rle(scratch, values.len(), target)?;
                Ok(LogicalEncoding::DeltaRle(meta))
            }
        }
    }

    /// Logically encode `u64` values into `target`.
    ///
    /// `target` is treated as a scratch buffer: it is cleared before writing.
    /// After the call, `target` holds the physically-stored sequence.
    /// See [`crate::decoder::LogicalValue::decode_u64`] for the reverse operation.
    #[hotpath::measure]
    pub fn encode_u64s(self, values: &[u64], target: &mut Vec<u64>) -> MltResult<LogicalEncoding> {
        match self {
            Self::None => {
                target.clear();
                target.extend_from_slice(values);
                Ok(LogicalEncoding::None)
            }
            Self::Delta => {
                encode_zigzag_delta(bytemuck::cast_slice::<u64, i64>(values), target);
                Ok(LogicalEncoding::Delta)
            }
            Self::Rle => {
                let meta = apply_rle(values, values.len(), target)?;
                Ok(LogicalEncoding::Rle(meta))
            }
            Self::DeltaRle => {
                encode_zigzag_delta(bytemuck::cast_slice::<u64, i64>(values), target);
                let intermediate = mem::take(target);
                let meta = apply_rle(&intermediate, values.len(), target)?;
                Ok(LogicalEncoding::DeltaRle(meta))
            }
        }
    }

    /// Logically encode `i64` values into `target` (u64 physical representation).
    ///
    /// `target` is treated as a scratch buffer: it is cleared before writing.
    /// After the call, `target` holds the physically-stored sequence.
    /// See [`crate::decoder::LogicalValue::decode_i64`] for the reverse operation.
    pub fn encode_i64s(self, values: &[i64], target: &mut Vec<u64>) -> MltResult<LogicalEncoding> {
        match self {
            Self::None => {
                encode_zigzag(values, target);
                Ok(LogicalEncoding::None)
            }
            Self::Delta => {
                encode_zigzag_delta(values, target);
                Ok(LogicalEncoding::Delta)
            }
            Self::Rle => {
                encode_zigzag(values, target);
                let zz = mem::take(target);
                let meta = apply_rle(&zz, values.len(), target)?;
                Ok(LogicalEncoding::Rle(meta))
            }
            Self::DeltaRle => {
                encode_zigzag_delta(values, target);
                let intermediate = mem::take(target);
                let meta = apply_rle(&intermediate, values.len(), target)?;
                Ok(LogicalEncoding::DeltaRle(meta))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::decoder::{
        DictionaryType, IntEncoding, LogicalEncoding, PhysicalEncoding, StreamType,
    };
    use crate::test_helpers::dec;
    use crate::{LogicalValue, StreamMeta};

    fn make_meta(logical_encoding: LogicalEncoding, num_values: usize) -> StreamMeta {
        let num_values =
            u32::try_from(num_values).expect("proptest to not generate that large of a vec");
        StreamMeta::new(
            StreamType::Data(DictionaryType::None),
            IntEncoding::new(logical_encoding, PhysicalEncoding::None),
            num_values,
        )
    }

    proptest! {
        #[test]
        fn test_u32_logical_roundtrip(
            values in prop::collection::vec(any::<u32>(), 0..100),
            logical in any::<LogicalEncoder>(),
        ) {
            let mut encoded = Vec::new();
            let mut scratch = Vec::new();
            let computed = logical.encode_u32s(&values, &mut encoded, &mut scratch).unwrap();
            let meta = make_meta(computed, values.len());
            let decoded = LogicalValue::new(meta).decode_u32(&encoded, &mut dec()).unwrap();
            prop_assert_eq!(decoded, values);
        }

        #[test]
        fn test_i32_logical_roundtrip(
            values in prop::collection::vec(any::<i32>(), 0..100),
            logical in any::<LogicalEncoder>(),
        ) {
            let mut encoded = Vec::new();
            let mut scratch = Vec::new();
            let computed = logical.encode_i32s(&values, &mut encoded, &mut scratch).unwrap();
            let meta = make_meta(computed, values.len());
            let decoded = LogicalValue::new(meta).decode_i32(&encoded, &mut dec()).unwrap();
            prop_assert_eq!(decoded, values);
        }

        #[test]
        fn test_u64_logical_roundtrip(
            values in prop::collection::vec(any::<u64>(), 0..100),
            logical in any::<LogicalEncoder>(),
        ) {
            let mut encoded = Vec::new();
            let computed = logical.encode_u64s(&values, &mut encoded).unwrap();
            let meta = make_meta(computed, values.len());
            let decoded = LogicalValue::new(meta).decode_u64(&encoded, &mut dec()).unwrap();
            prop_assert_eq!(decoded, values);
        }

        #[test]
        fn test_i64_logical_roundtrip(
            values in prop::collection::vec(any::<i64>(), 0..100),
            logical in any::<LogicalEncoder>(),
        ) {
            let mut encoded = Vec::new();
            let computed = logical.encode_i64s(&values, &mut encoded).unwrap();
            let meta = make_meta(computed, values.len());
            let decoded = LogicalValue::new(meta).decode_i64(&encoded, &mut dec()).unwrap();
            prop_assert_eq!(decoded, values);
        }
    }
}
