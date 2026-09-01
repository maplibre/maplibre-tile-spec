//! Choosing ALP parameters for a float column.

use zigzag::ZigZag as _;

use crate::codecs::alp::{candidates, encode_exact};
use crate::codecs::float::FloatValue;
use crate::decoder::Alp;
use crate::encoder::stream::float_cost::{ENCODING_BYTE, data_bytes, raw_stored_bytes, varint_len};

/// A float column encoded as ALP integers, with the parameters that produced it.
pub(crate) struct AlpStream {
    pub(crate) params: Alp,
    /// Zigzagged, ready for the varint physical encoding.
    pub(crate) codes: Vec<u64>,
}

impl AlpStream {
    /// The smallest exception-free ALP encoding of `values`, or [`None`] when no parameters fit.
    /// Whether it beats storing the floats raw is the caller's question.
    pub(crate) fn smallest<T: FloatValue>(values: &[T]) -> Option<Self> {
        if values.is_empty() {
            return None;
        }
        let mut scratch = Vec::new();
        let mut best: Option<(Alp, usize)> = None;

        for params in candidates::<T>() {
            if encode_exact(values, params, &mut scratch).is_none() {
                continue;
            }
            let bytes = stored_bytes(scratch.iter().map(|&c| i64::encode(c)), params);
            if best.is_none_or(|(_, best_bytes)| bytes < best_bytes) {
                best = Some((params, bytes));
            }
        }

        let (params, _) = best?;
        encode_exact(values, params, &mut scratch)
            .expect("the winning parameters encoded every value a moment ago");
        Some(Self {
            params,
            codes: scratch.into_iter().map(i64::encode).collect(),
        })
    }

    /// The smallest exception-free ALP encoding, or [`None`] when it would not beat the raw column.
    pub(crate) fn worth_building<T: FloatValue>(values: &[T]) -> Option<Self> {
        Self::smallest(values)
            .filter(|alp| alp.stored_bytes() < raw_stored_bytes::<T>(values.len()))
    }

    /// Bytes this stream occupies, for the competition against a dictionary.
    pub(crate) fn stored_bytes(&self) -> usize {
        stored_bytes(self.codes.iter().copied(), self.params)
    }
}

/// Bytes an ALP stream of these zigzagged integers occupies, header included.
fn stored_bytes(codes: impl Iterator<Item = u64>, params: Alp) -> usize {
    ENCODING_BYTE + varint_len(params.e) + varint_len(params.f) + data_bytes(codes)
}
