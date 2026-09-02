//! Choosing ALP parameters for a float column.

use crate::MltResult;
use crate::codecs::alp::{candidates, carries, encode_exact};
use crate::codecs::float::FloatValue;
use crate::decoder::{Alp, AlpScale, FastPForKind, PhysicalEncoding};
use crate::encoder::model::EncoderConfig;
use crate::encoder::stream::codecs::PhysicalCodecs;
use crate::encoder::stream::float_cost::{ENCODING_BYTE, data_bytes, raw_stored_bytes, varint_len};

/// A float column encoded as ALP integers, with the parameters that produced it.
pub(crate) struct AlpStream {
    pub(crate) params: Alp,
    /// Offsets from the frame of reference, varint-coded unless [`Self::packed`] holds them.
    pub(crate) offsets: Vec<u64>,
    /// The bitpacked payload and the kind that produced it, once `FastPFOR` has won the race.
    /// Holding the bytes rather than a flag keeps the header's physical field and the payload
    /// from ever disagreeing, and saves encoding the column a second time in order to write it.
    packed: Option<(FastPForKind, Vec<u8>)>,
    /// Bytes the chosen physical encoding takes for the offsets, its length varint included.
    payload_bytes: usize,
}

impl AlpStream {
    /// The smallest exception-free ALP encoding of `values`, or [`None`] when no parameters fit.
    /// Whether it beats storing the floats raw is the caller's question.
    ///
    /// The first scale that encodes every value is the smallest one, so the search takes it and
    /// stops: [`candidates`] walks net exponents in ascending order, and codes grow tenfold with
    /// each, so nothing later can be narrower. `early_exit_finds_the_same_size_as_the_exhaustive_search`
    /// holds this against a full walk of the grid.
    pub(crate) fn smallest<T: FloatValue>(values: &[T]) -> Option<Self> {
        if values.is_empty() {
            return None;
        }
        let mut codes = Vec::new();
        // A value one scale cannot carry usually defeats the next one too, so the last failure
        // is retried first: that rejects a scale in one test rather than a whole column pass.
        let mut suspect = 0;
        let scale = candidates().find(|&scale| {
            carries(values[suspect], scale)
                && match encode_exact(values, scale, &mut codes) {
                    Ok(()) => true,
                    Err(rejected) => {
                        suspect = rejected;
                        false
                    }
                }
        })?;
        let params = frame(&codes, scale);
        let offsets: Vec<u64> = codes.iter().map(|&code| params.offset_of(code)).collect();
        Some(Self {
            payload_bytes: data_bytes(offsets.iter().copied()),
            packed: None,
            params,
            offsets,
        })
    }

    /// The smallest exception-free ALP encoding, or [`None`] when it would not beat the raw column.
    /// The physical encodings race first, so the size this is judged on is the size it will store.
    pub(crate) fn worth_building<T: FloatValue>(
        values: &[T],
        codecs: &mut PhysicalCodecs,
        cfg: EncoderConfig,
    ) -> MltResult<Option<Self>> {
        let Some(mut alp) = Self::smallest(values) else {
            return Ok(None);
        };
        alp.race_fastpfor(codecs, cfg)?;
        Ok(Some(alp).filter(|alp| alp.stored_bytes() < raw_stored_bytes::<T>(values.len())))
    }

    /// Take `FastPFOR` for the offsets if it stores fewer bytes than varint does.
    ///
    /// Raced rather than preferred: its 128-value block framing does not amortise over a short
    /// column, where it comes out larger. It also codes `u32` words, so a column whose offsets
    /// do not all fit one is no candidate at all.
    pub(crate) fn race_fastpfor(
        &mut self,
        codecs: &mut PhysicalCodecs,
        cfg: EncoderConfig,
    ) -> MltResult<()> {
        if cfg.fastpfor().is_none() {
            return Ok(());
        }
        let Some(words) = self.narrow_offsets() else {
            return Ok(());
        };
        let kind = cfg.wire_version().fastpfor_kind();
        let packed = codecs.fastpfor(kind, &words)?;
        let packed_bytes = varint_len(packed.len()) + packed.len();
        if packed_bytes < self.payload_bytes {
            self.payload_bytes = packed_bytes;
            self.packed = Some((kind, packed.to_vec()));
        }
        Ok(())
    }

    /// The payload to write and the physical encoding that names it, which are chosen together.
    pub(crate) fn payload<'a>(
        &'a self,
        codecs: &'a mut PhysicalCodecs,
    ) -> (PhysicalEncoding, &'a [u8]) {
        match &self.packed {
            Some((kind, packed)) => (PhysicalEncoding::FastPFor(*kind), packed),
            None => (PhysicalEncoding::VarInt, codecs.varint(&self.offsets)),
        }
    }

    /// The offsets as the `u32` words `FastPFOR` codes, or [`None`] when one does not fit.
    fn narrow_offsets(&self) -> Option<Vec<u32>> {
        self.offsets
            .iter()
            .map(|&o| u32::try_from(o).ok())
            .collect()
    }

    /// Bytes this stream occupies, for the competition against a dictionary.
    pub(crate) fn stored_bytes(&self) -> usize {
        header_bytes(self.params) + self.payload_bytes
    }
}

/// The frame of reference for these scaled integers: the smallest, so every offset is non-negative.
fn frame(codes: &[i64], scale: AlpScale) -> Alp {
    Alp {
        scale,
        base: codes.iter().copied().min().unwrap_or(0),
    }
}

/// Bytes an ALP stream's header occupies: the encoding byte and the three parameter varints.
fn header_bytes(params: Alp) -> usize {
    ENCODING_BYTE
        + varint_len(params.scale.e)
        + varint_len(params.scale.f)
        + varint_len(params.base)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// Bytes these scaled integers would occupy at `scale`, once framed and varint-coded.
    /// Only the oracle needs this: the search itself never costs a scale it did not take.
    fn cost(codes: &[i64], scale: AlpScale) -> usize {
        let params = frame(codes, scale);
        header_bytes(params) + data_bytes(codes.iter().map(|&code| params.offset_of(code)))
    }

    /// The exhaustive search the first-fit one must agree with.
    fn exhaustive<T: FloatValue>(values: &[T]) -> Option<usize> {
        let mut scratch = Vec::new();
        let mut best: Option<usize> = None;
        for scale in candidates() {
            if encode_exact(values, scale, &mut scratch).is_err() {
                continue;
            }
            let bytes = cost(&scratch, scale);
            if best.is_none_or(|best_bytes| bytes < best_bytes) {
                best = Some(bytes);
            }
        }
        best
    }

    proptest! {
        #[test]
        fn early_exit_finds_the_same_size_as_the_exhaustive_search(
            values in prop::collection::vec(-1e6f64..1e6, 1..40),
        ) {
            let found = AlpStream::smallest(&values).map(|alp| alp.stored_bytes());
            prop_assert_eq!(found, exhaustive(&values));
        }

        #[test]
        fn every_column_that_encodes_at_all_returns_bit_for_bit(
            values in prop::collection::vec(-1e6f64..1e6, 1..40),
        ) {
            let Some(alp) = AlpStream::smallest(&values) else { return Ok(()) };
            let codes: Vec<i64> = alp.offsets.iter().map(|&o| alp.params.code_at(o)).collect();
            prop_assert_eq!(crate::codecs::alp::decode::<f64>(&codes, alp.params.scale), values);
        }

        #[test]
        fn offsets_are_true_distances_from_the_smallest_scaled_integer(
            values in prop::collection::vec(-1e6f64..1e6, 1..40),
        ) {
            let Some(alp) = AlpStream::smallest(&values) else { return Ok(()) };
            let codes: Vec<i64> = alp.offsets.iter().map(|&o| alp.params.code_at(o)).collect();
            let smallest = codes.iter().copied().min().expect("non-empty");
            prop_assert_eq!(smallest, alp.params.base);
            prop_assert_eq!(alp.offsets.iter().copied().min(), Some(0));
            for (&offset, &code) in alp.offsets.iter().zip(&codes) {
                prop_assert_eq!(offset, code.wrapping_sub(smallest).cast_unsigned());
            }
        }
    }

    /// The shape the retried-failure heuristic exists for: every scale is ruled out, but only
    /// by the very last value, so a naive search would walk the whole column once per candidate.
    #[test]
    fn a_column_rejected_only_by_its_last_value_takes_no_parameters() {
        let mut values: Vec<f64> = (0..500).map(|i| f64::from(i) * 0.5).collect();
        values.push(f64::NAN);
        assert!(AlpStream::smallest(&values).is_none());
    }

    #[test]
    fn two_decimal_values_pick_the_smallest_net_exponent() {
        let alp = AlpStream::smallest(&[1.25f64, 100.75, -0.25]).expect("some parameters fit");
        assert_eq!(alp.params.scale.net(), 2);
    }

    #[test]
    fn shifting_a_column_away_from_zero_costs_only_a_wider_base() {
        let here: Vec<f64> = (0..256).map(|i| f64::from(i) * 0.25).collect();
        let far: Vec<f64> = here.iter().map(|v| v + 52_500_000.0).collect();
        let a = AlpStream::smallest(&here).expect("fits").stored_bytes();
        let b = AlpStream::smallest(&far).expect("fits").stored_bytes();
        assert!(b <= a + 5, "{b} vs {a}");
    }
}
