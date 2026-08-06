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
    use crate::decoder::RawStream;
    use crate::encoder::model::StreamCtx;
    use crate::encoder::{Codecs, Encoder, EncoderConfig, ExplicitEncoder, IntEncoder};
    use crate::test_helpers::{assert_empty, dec, parser};

    proptest! {
        #[test]
        fn test_u32_logical_roundtrip(
            values in prop::collection::vec(any::<u32>(), 0..100),
            logical in any::<LogicalEncoder>(),
        ) {
            let mut enc = Encoder::with_explicit(
                EncoderConfig::default(),
                ExplicitEncoder::all(IntEncoder::varint_with(logical)),
            );
            let mut codecs = Codecs::default();
            let ctx = StreamCtx::prop_data("test");
            codecs.write_int_stream(&values, &ctx, &mut enc).unwrap();
            let parsed = assert_empty(RawStream::from_bytes(enc.data(), &mut parser()));
            let decoded = parsed.decode_ints::<u32>(&mut dec()).unwrap();
            prop_assert_eq!(decoded, values);
        }

        #[test]
        fn test_i32_logical_roundtrip(
            values in prop::collection::vec(any::<i32>(), 0..100),
            logical in any::<LogicalEncoder>(),
        ) {
            let mut enc = Encoder::with_explicit(
                EncoderConfig::default(),
                ExplicitEncoder::all(IntEncoder::varint_with(logical)),
            );
            let mut codecs = Codecs::default();
            let ctx = StreamCtx::prop_data("test");
            codecs.write_int_stream(&values, &ctx, &mut enc).unwrap();
            let parsed = assert_empty(RawStream::from_bytes(enc.data(), &mut parser()));
            let decoded = parsed.decode_ints::<i32>(&mut dec()).unwrap();
            prop_assert_eq!(decoded, values);
        }

        #[test]
        fn test_u64_logical_roundtrip(
            values in prop::collection::vec(any::<u64>(), 0..100),
            logical in any::<LogicalEncoder>(),
        ) {
            let mut enc = Encoder::with_explicit(
                EncoderConfig::default(),
                ExplicitEncoder::all(IntEncoder::varint_with(logical)),
            );
            let mut codecs = Codecs::default();
            let ctx = StreamCtx::prop_data("test");
            codecs.write_int_stream(&values, &ctx, &mut enc).unwrap();
            let parsed = assert_empty(RawStream::from_bytes(enc.data(), &mut parser()));
            let decoded = parsed.decode_ints::<u64>(&mut dec()).unwrap();
            prop_assert_eq!(decoded, values);
        }

        #[test]
        fn test_i64_logical_roundtrip(
            values in prop::collection::vec(any::<i64>(), 0..100),
            logical in any::<LogicalEncoder>(),
        ) {
            let mut enc = Encoder::with_explicit(
                EncoderConfig::default(),
                ExplicitEncoder::all(IntEncoder::varint_with(logical)),
            );
            let mut codecs = Codecs::default();
            let ctx = StreamCtx::prop_data("test");
            codecs.write_int_stream(&values, &ctx, &mut enc).unwrap();
            let parsed = assert_empty(RawStream::from_bytes(enc.data(), &mut parser()));
            let decoded = parsed.decode_ints::<i64>(&mut dec()).unwrap();
            prop_assert_eq!(decoded, values);
        }
    }
}
