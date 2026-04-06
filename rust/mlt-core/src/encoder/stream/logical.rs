use std::fmt::Debug;

use num_traits::PrimInt;

use crate::MltResult;
use crate::codecs::rle::encode_rle;
use crate::codecs::zigzag::{encode_zigzag, encode_zigzag_delta};
use crate::decoder::{LogicalEncoding, RleMeta};

/// RLE-encode a sequence into `[run-lengths | unique-values]` and return the matching `RleMeta`.
/// `num_logical` is the expanded output length (stored in `RleMeta::num_rle_values`).
fn apply_rle<T: PrimInt + Debug>(data: &[T], num_logical: usize) -> MltResult<(Vec<T>, RleMeta)> {
    let (runs_vec, vals_vec) = encode_rle(data);
    let meta = RleMeta {
        runs: u32::try_from(runs_vec.len())?,
        num_rle_values: u32::try_from(num_logical)?,
    };
    let mut combined = runs_vec;
    combined.extend(vals_vec);
    Ok((combined, meta))
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
    /// Logically encode `u32` values, returning the physically-stored sequence and the concrete decoder.
    ///
    /// [`LogicalEncoding`] is derived from the actual data.
    /// See [`LogicalValue::decode_u32`] for the reverse operation.
    #[cfg_attr(feature = "__hotpath", hotpath::measure)]
    pub fn encode_u32s(self, values: &[u32]) -> MltResult<(Vec<u32>, LogicalEncoding)> {
        match self {
            Self::None => Ok((values.to_vec(), LogicalEncoding::None)),
            Self::Delta => {
                let values = values.iter().map(|&v| v.cast_signed()).collect::<Vec<_>>();
                let u32s = encode_zigzag_delta(&values);
                Ok((u32s, LogicalEncoding::Delta))
            }
            Self::Rle => {
                let (u32s, meta) = apply_rle(values, values.len())?;
                Ok((u32s, LogicalEncoding::Rle(meta)))
            }
            Self::DeltaRle => {
                let values = values.iter().map(|&v| v.cast_signed()).collect::<Vec<_>>();
                let delta = encode_zigzag_delta(&values);
                let (u32s, meta) = apply_rle(&delta, values.len())?;
                Ok((u32s, LogicalEncoding::DeltaRle(meta)))
            }
        }
    }

    /// Logically encode `i32` values into the `u32` physical representation.
    ///
    /// [`LogicalEncoding`] is derived from the actual data.
    /// See [`LogicalValue::decode_i32`] for the reverse operation.
    #[cfg_attr(feature = "__hotpath", hotpath::measure)]
    pub fn encode_i32s(self, values: &[i32]) -> MltResult<(Vec<u32>, LogicalEncoding)> {
        match self {
            Self::None => Ok((encode_zigzag(values), LogicalEncoding::None)),
            Self::Delta => Ok((encode_zigzag_delta(values), LogicalEncoding::Delta)),
            Self::Rle => {
                let (u32s, meta) = apply_rle(&encode_zigzag(values), values.len())?;
                Ok((u32s, LogicalEncoding::Rle(meta)))
            }
            Self::DeltaRle => {
                let (u32s, meta) = apply_rle(&encode_zigzag_delta(values), values.len())?;
                Ok((u32s, LogicalEncoding::DeltaRle(meta)))
            }
        }
    }

    /// Logically encode `u64` values into the `u64` physical representation.
    ///
    /// [`LogicalEncoding`] is derived from the actual data.
    /// See [`LogicalValue::decode_u64`] for the reverse operation.
    #[cfg_attr(feature = "__hotpath", hotpath::measure)]
    pub fn encode_u64s(self, values: &[u64]) -> MltResult<(Vec<u64>, LogicalEncoding)> {
        match self {
            Self::None => Ok((values.to_vec(), LogicalEncoding::None)),
            Self::Delta => Ok((
                encode_zigzag_delta(&values.iter().map(|&v| v.cast_signed()).collect::<Vec<_>>()),
                LogicalEncoding::Delta,
            )),
            Self::Rle => {
                let (u64s, meta) = apply_rle(values, values.len())?;
                Ok((u64s, LogicalEncoding::Rle(meta)))
            }
            Self::DeltaRle => {
                let delta = encode_zigzag_delta(
                    &values.iter().map(|&v| v.cast_signed()).collect::<Vec<_>>(),
                );
                let (u64s, meta) = apply_rle(&delta, values.len())?;
                Ok((u64s, LogicalEncoding::DeltaRle(meta)))
            }
        }
    }

    /// Logically encode `i64` values into the `u64` physical representation.
    ///
    /// [`LogicalEncoding`] is derived from the actual data.
    /// See [`LogicalValue::decode_i64`] for the reverse operation.
    pub fn encode_i64s(self, values: &[i64]) -> MltResult<(Vec<u64>, LogicalEncoding)> {
        match self {
            Self::None => Ok((encode_zigzag(values), LogicalEncoding::None)),
            Self::Delta => Ok((encode_zigzag_delta(values), LogicalEncoding::Delta)),
            Self::Rle => {
                let (u64s, meta) = apply_rle(&encode_zigzag(values), values.len())?;
                Ok((u64s, LogicalEncoding::Rle(meta)))
            }
            Self::DeltaRle => {
                let (u64s, meta) = apply_rle(&encode_zigzag_delta(values), values.len())?;
                Ok((u64s, LogicalEncoding::DeltaRle(meta)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::decoder::{
        DictionaryType, IntEncoding, LogicalEncoding, LogicalValue, PhysicalEncoding, StreamMeta,
        StreamType,
    };
    use crate::test_helpers::dec;

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
            let (encoded, computed) = logical.encode_u32s(&values).unwrap();
            let meta = make_meta(computed, values.len());
            let decoded = LogicalValue::new(meta).decode_u32(&encoded, &mut dec()).unwrap();
            prop_assert_eq!(decoded, values);
        }

        #[test]
        fn test_i32_logical_roundtrip(
            values in prop::collection::vec(any::<i32>(), 0..100),
            logical in any::<LogicalEncoder>(),
        ) {
            let (encoded, computed) = logical.encode_i32s(&values).unwrap();
            let meta = make_meta(computed, values.len());
            let decoded = LogicalValue::new(meta).decode_i32(&encoded, &mut dec()).unwrap();
            prop_assert_eq!(decoded, values);
        }

        #[test]
        fn test_u64_logical_roundtrip(
            values in prop::collection::vec(any::<u64>(), 0..100),
            logical in any::<LogicalEncoder>(),
        ) {
            let (encoded, computed) = logical.encode_u64s(&values).unwrap();
            let meta = make_meta(computed, values.len());
            let decoded = LogicalValue::new(meta).decode_u64(&encoded, &mut dec()).unwrap();
            prop_assert_eq!(decoded, values);
        }

        #[test]
        fn test_i64_logical_roundtrip(
            values in prop::collection::vec(any::<i64>(), 0..100),
            logical in any::<LogicalEncoder>(),
        ) {
            let (encoded, computed) = logical.encode_i64s(&values).unwrap();
            let meta = make_meta(computed, values.len());
            let decoded = LogicalValue::new(meta).decode_i64(&encoded, &mut dec()).unwrap();
            prop_assert_eq!(decoded, values);
        }
    }
}
