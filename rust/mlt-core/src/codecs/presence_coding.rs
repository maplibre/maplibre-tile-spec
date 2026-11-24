//! The two ways a v2 layer says which features have a value: a bitmap, or that bitmap through ORC byte RLE.

use std::borrow::Cow;

use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;
use bitvec::vec::BitVec;
use bitvec::view::BitView as _;
use usize_cast::IntoUsize as _;

use crate::MltError::{self, PresenceRunOverflow};
use crate::codecs::rle::encode_byte_rle;
use crate::decoder::BoolLogical;
use crate::utils::take;
use crate::{MltRefResult, MltResult};

/// Charges the bits a coding builds, so a tile cannot ask for an allocation far larger
/// than the bytes it spent asking.
pub(crate) trait PresenceBudget {
    /// Reserve room for `count` bits, or fail rather than allocate it.
    fn reserve_bits(&mut self, count: u32) -> MltResult<()>;
}

/// For the readers that hold no budget of their own - the dump walker and the renderer,
/// which are handed a payload whose length already bounds what it can name.
pub(crate) struct Unmetered;

impl PresenceBudget for Unmetered {
    fn reserve_bits(&mut self, _count: u32) -> MltResult<()> {
        Ok(())
    }
}

/// How a bitfield is stored, numbered as the `Bool` stream family numbers its logical field.
/// A presence nibble names one as `code + 1`, and a shared bitfield's coding byte holds the code in the logical field of an encoding byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
#[repr(u8)]
pub enum PresenceCoding {
    /// `ceil(N/8)` bytes, LSB-first. Bits past `N` are padding and are ignored.
    Bitmap = 0,
    /// The bitmap's `ceil(N/8)` bytes through ORC byte RLE, ending as soon as they are all produced.
    ByteRle = 1,
}

impl PresenceCoding {
    /// Every coding, in the order a tie between them is broken.
    pub(crate) const ALL: [Self; 2] = [Self::Bitmap, Self::ByteRle];

    /// The coding a `Bool` stream's logical field numbers `code`, or [`None`] for one this version has no meaning for.
    #[must_use]
    pub(crate) fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Bitmap),
            1 => Some(Self::ByteRle),
            _ => None,
        }
    }

    /// The coding a bool stream's logical encoding names, or [`None`] for v1's header-counted byte-RLE.
    #[must_use]
    pub(crate) fn of_logical(logical: BoolLogical) -> Option<Self> {
        match logical {
            BoolLogical::None => Some(Self::Bitmap),
            BoolLogical::SelfDelimitedByteRle => Some(Self::ByteRle),
            BoolLogical::ByteRle(_) => None,
        }
    }
}

impl From<PresenceCoding> for BoolLogical {
    fn from(coding: PresenceCoding) -> Self {
        match coding {
            PresenceCoding::Bitmap => Self::None,
            PresenceCoding::ByteRle => Self::SelfDelimitedByteRle,
        }
    }
}

/// `bits` as the `ceil(N/8)` bytes of a bitmap.
fn packed(bits: &[bool]) -> Vec<u8> {
    let mut bytes = vec![0u8; bits.len().div_ceil(8)];
    for (i, &bit) in bits.iter().enumerate() {
        if bit {
            bytes[i / 8] |= 1 << (i % 8);
        }
    }
    bytes
}

/// Write `bits` in `coding`, appending to `out`.
pub(crate) fn write(out: &mut Vec<u8>, bits: &[bool], coding: PresenceCoding) {
    let bytes = packed(bits);
    match coding {
        PresenceCoding::Bitmap => out.extend_from_slice(&bytes),
        PresenceCoding::ByteRle => {
            let mut encoded = Vec::new();
            out.extend_from_slice(encode_byte_rle(&bytes, &mut encoded));
        }
    }
}

/// What `bits` would take in `coding`, without writing it.
#[must_use]
pub(crate) fn size(bits: &[bool], coding: PresenceCoding) -> usize {
    match coding {
        PresenceCoding::Bitmap => bits.len().div_ceil(8),
        PresenceCoding::ByteRle => {
            let mut encoded = Vec::new();
            encode_byte_rle(&packed(bits), &mut encoded).len()
        }
    }
}

/// The smallest coding for `bits`, ties broken toward the lowest code so that the same
/// mask always leaves the same bytes.
/// Without `allow_rle` that is always [`PresenceCoding::Bitmap`].
#[must_use]
pub(crate) fn smallest(bits: &[bool], allow_rle: bool) -> PresenceCoding {
    if !allow_rle {
        return PresenceCoding::Bitmap;
    }
    PresenceCoding::ALL
        .into_iter()
        .min_by_key(|&c| (size(bits, c), c as u8))
        .expect("ALL is not empty")
}

/// One ORC byte-RLE run, as it sits in the input.
enum Run<'a> {
    Literal(&'a [u8]),
    Repeat { value: u8, copies: usize },
}

impl Run<'_> {
    fn len(&self) -> usize {
        match self {
            Run::Literal(bytes) => bytes.len(),
            Run::Repeat { copies, .. } => *copies,
        }
    }
}

/// Take the next run of a Boolean RLE payload that still has `remaining` bytes to produce.
/// A run that would produce more than that overflows the `count` features the payload covers.
fn next_run(input: &[u8], remaining: usize, count: u32) -> MltRefResult<'_, Run<'_>> {
    let (&control, rest) = input.split_first().ok_or(MltError::BufferUnderflow(1, 0))?;
    let (rest, run) = if control >= 128 {
        let len = 256 - usize::from(control);
        let (literal, rest) = rest
            .split_at_checked(len)
            .ok_or_else(|| MltError::BufferUnderflow(u32::from(control ^ 0xFF) + 1, rest.len()))?;
        (rest, Run::Literal(literal))
    } else {
        let (&value, rest) = rest.split_first().ok_or(MltError::BufferUnderflow(1, 0))?;
        let copies = usize::from(control) + 3;
        (rest, Run::Repeat { value, copies })
    };
    if run.len() > remaining {
        return Err(PresenceRunOverflow(count));
    }
    Ok((rest, run))
}

/// Read `count` presence bits stored in `coding`, charging what it builds to `budget`.
///
/// A bitmap is borrowed from the tile bytes, and needs `ceil(count/8)` of them to be there
/// at all, so the input bounds it. Boolean RLE is built, and two bytes can expand to 130,
/// so the allocation is reserved before it is made rather than trusted.
pub(crate) fn read<'a>(
    input: &'a [u8],
    count: u32,
    coding: PresenceCoding,
    budget: &mut dyn PresenceBudget,
) -> MltRefResult<'a, Cow<'a, BitSlice<u8, Lsb0>>> {
    let n = count.into_usize();
    match coding {
        PresenceCoding::Bitmap => {
            let (input, bytes) = take(input, count.div_ceil(8))?;
            Ok((input, Cow::Borrowed(&bytes.view_bits::<Lsb0>()[..n])))
        }
        PresenceCoding::ByteRle => {
            let total = n.div_ceil(8);
            budget.reserve_bits(count)?;
            let mut plain = Vec::with_capacity(total.min(input.len().saturating_mul(130)));
            let mut input = input;
            while plain.len() < total {
                let run: Run;
                (input, run) = next_run(input, total - plain.len(), count)?;
                match run {
                    Run::Literal(bytes) => plain.extend_from_slice(bytes),
                    Run::Repeat { value, copies } => {
                        plain.extend(std::iter::repeat_n(value, copies));
                    }
                }
            }
            let mut bits = BitVec::<u8, Lsb0>::from_vec(plain);
            bits.truncate(n);
            Ok((input, Cow::Owned(bits)))
        }
    }
}

/// Split off the bytes `count` bits take in `coding`, without building them.
///
/// Every coding delimits itself, so this is how a reader finds where a stream's payload ends.
pub(crate) fn split(input: &[u8], count: u32, coding: PresenceCoding) -> MltRefResult<'_, &[u8]> {
    let total = count.into_usize().div_ceil(8);
    let len = match coding {
        PresenceCoding::Bitmap => total,
        PresenceCoding::ByteRle => {
            let mut rest = input;
            let mut produced = 0usize;
            while produced < total {
                let run: Run;
                (rest, run) = next_run(rest, total - produced, count)?;
                produced += run.len();
            }
            input.len() - rest.len()
        }
    };
    let (rest, payload) = take(input, u32::try_from(len)?)?;
    Ok((rest, payload))
}

/// How many of `count` bits `payload` marks present, without building them.
///
/// `payload` is exactly what [`split`] returned for `count` and `coding`.
pub(crate) fn popcount(payload: &[u8], count: u32, coding: PresenceCoding) -> MltResult<u32> {
    let n = count.into_usize();
    let total = n.div_ceil(8);
    let ones = |byte: u8, at: usize| {
        let live = (n - at * 8).min(8);
        (byte & (u8::MAX >> (8 - live))).count_ones() as usize
    };
    let bits = match coding {
        #[expect(
            clippy::unnecessary_lazy_evaluations,
            reason = "an eager error is dropped on every success"
        )]
        PresenceCoding::Bitmap => payload
            .get(..total)
            .ok_or_else(|| MltError::UnableToTake(count))?
            .view_bits::<Lsb0>()[..n]
            .count_ones(),
        PresenceCoding::ByteRle => {
            let mut rest = payload;
            let mut at = 0usize;
            let mut sum = 0usize;
            while at < total {
                let run: Run;
                (rest, run) = next_run(rest, total - at, count)?;
                sum += match &run {
                    Run::Literal(bytes) => bytes
                        .iter()
                        .enumerate()
                        .map(|(i, &byte)| ones(byte, at + i))
                        .sum::<usize>(),
                    Run::Repeat { value, copies } => {
                        (at..at + copies).map(|i| ones(*value, i)).sum::<usize>()
                    }
                };
                at += run.len();
            }
            sum
        }
    };
    Ok(u32::try_from(bits)?)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn mask(len: usize, present: impl Fn(usize) -> bool) -> Vec<bool> {
        (0..len).map(present).collect()
    }

    fn count_of(bits: &[bool]) -> u32 {
        u32::try_from(bits.len()).unwrap()
    }

    fn written(bits: &[bool], coding: PresenceCoding) -> Vec<u8> {
        let mut buf = Vec::new();
        write(&mut buf, bits, coding);
        buf
    }

    fn read_back(bits: &[bool], coding: PresenceCoding) -> Vec<bool> {
        let buf = written(bits, coding);
        assert_eq!(buf.len(), size(bits, coding), "size disagrees with write");
        let (rest, out) = read(&buf, count_of(bits), coding, &mut Unmetered).unwrap();
        assert_eq!(rest, [] as [u8; 0]);
        out.iter().by_vals().collect()
    }

    fn cases() -> Vec<Vec<bool>> {
        vec![
            vec![],
            vec![true],
            vec![false],
            vec![true; 9],
            vec![false; 9],
            mask(100, |i| (20..60).contains(&i)),
            mask(1000, |i| i == 777),
            mask(64, |i| i % 2 == 0),
            mask(17, |i| i > 12),
            mask(4096, |i| matches!(i, 5 | 6 | 1000 | 3000)),
            mask(70, |i| i % 13 == 0),
            mask(2003, |_| false),
            mask(2003, |_| true),
            mask(1603, |i| (i * 7 + i / 8) % 5 < 2),
        ]
    }

    #[rstest]
    #[case::empty(vec![])]
    #[case::one_present(vec![true])]
    #[case::nine_absent(vec![false; 9])]
    #[case::not_a_multiple_of_eight(mask(1003, |i| i % 13 == 0))]
    #[case::repeat_longer_than_one_run_holds(mask(2003, |_| false))]
    #[case::all_present_over_two_runs(mask(2003, |_| true))]
    #[case::literal_longer_than_one_run_holds(mask(1603, |i| (i * 7 + i / 8) % 5 < 2))]
    #[case::scattered(mask(4096, |i| matches!(i, 5 | 6 | 1000 | 3000)))]
    fn byte_rle_round_trips(#[case] bits: Vec<bool>) {
        assert_eq!(read_back(&bits, PresenceCoding::ByteRle), bits);
    }

    #[test]
    fn bitmap_round_trips_every_case() {
        for bits in cases() {
            assert_eq!(read_back(&bits, PresenceCoding::Bitmap), bits);
        }
    }

    #[rstest]
    #[case::repeat_past_the_bytes(&[0x00, 0xFF], 16)]
    #[case::literal_past_the_bytes(&[0xFD, 1, 2, 3], 16)]
    #[case::second_run_past_the_bytes(&[0xFF, 7, 0x00, 0xFF], 16)]
    fn a_run_past_the_bytes_the_mask_covers_is_rejected(#[case] input: &[u8], #[case] count: u32) {
        assert!(matches!(
            read(input, count, PresenceCoding::ByteRle, &mut Unmetered),
            Err(PresenceRunOverflow(c)) if c == count
        ));
        assert!(matches!(
            split(input, count, PresenceCoding::ByteRle),
            Err(PresenceRunOverflow(c)) if c == count
        ));
        assert!(matches!(
            popcount(input, count, PresenceCoding::ByteRle),
            Err(PresenceRunOverflow(c)) if c == count
        ));
    }

    #[rstest]
    #[case::no_input(&[], 8, MltError::BufferUnderflow(1, 0))]
    #[case::repeat_without_its_value(&[0x00], 24, MltError::BufferUnderflow(1, 0))]
    #[case::literal_short_of_its_bytes(&[0xFE, 1], 16, MltError::BufferUnderflow(2, 1))]
    #[case::ends_before_every_byte_is_made(&[0xFF, 7], 16, MltError::BufferUnderflow(1, 0))]
    fn a_truncated_payload_is_rejected(
        #[case] input: &[u8],
        #[case] count: u32,
        #[case] expected: MltError,
    ) {
        let err = read(input, count, PresenceCoding::ByteRle, &mut Unmetered)
            .err()
            .unwrap();
        assert_eq!(err.to_string(), expected.to_string());
        let err = split(input, count, PresenceCoding::ByteRle).err().unwrap();
        assert_eq!(err.to_string(), expected.to_string());
    }

    #[test]
    fn a_short_bitmap_is_rejected() {
        assert!(matches!(
            read(&[0], 9, PresenceCoding::Bitmap, &mut Unmetered),
            Err(MltError::UnableToTake(2))
        ));
    }

    #[test]
    fn byte_rle_hands_back_the_input_after_its_last_run() {
        let (rest, bits) = read(
            &[0x00, 0xFF, 0xAA, 0xBB],
            24,
            PresenceCoding::ByteRle,
            &mut Unmetered,
        )
        .unwrap();
        assert_eq!(
            (rest, bits.len(), bits.count_ones()),
            (&[0xAA, 0xBB][..], 24, 24)
        );
    }

    #[test]
    fn bits_past_the_count_in_the_last_byte_are_ignored() {
        let (_, bits) = read(&[0x00, 0xFF], 20, PresenceCoding::ByteRle, &mut Unmetered).unwrap();
        assert_eq!((bits.len(), bits.count_ones()), (20, 20));
    }

    #[test]
    fn smallest_is_the_smallest() {
        for bits in cases() {
            let picked = smallest(&bits, true);
            let best = PresenceCoding::ALL
                .into_iter()
                .map(|c| size(&bits, c))
                .min()
                .unwrap();
            assert_eq!(size(&bits, picked), best, "on {bits:?}");
        }
    }

    #[rstest]
    #[case::all_absent(mask(4096, |_| false))]
    #[case::all_present(mask(4096, |_| true))]
    #[case::one_block(mask(4096, |i| (100..2000).contains(&i)))]
    fn byte_rle_beats_a_bitmap_on_long_runs(#[case] bits: Vec<bool>) {
        assert_eq!(smallest(&bits, true), PresenceCoding::ByteRle);
        assert!(size(&bits, PresenceCoding::ByteRle) < size(&bits, PresenceCoding::Bitmap));
    }

    #[rstest]
    #[case::eight_distinct_bytes(mask(64, |i| i % 8 == i / 8))]
    #[case::scattered_bytes(mask(1603, |i| (i * 7 + i / 8) % 5 < 2))]
    #[case::one_byte(vec![true])]
    #[case::two_bytes(mask(16, |i| i < 3))]
    fn a_bitmap_beats_byte_rle_on_scattered_bits(#[case] bits: Vec<bool>) {
        assert_eq!(smallest(&bits, true), PresenceCoding::Bitmap);
    }

    #[test]
    fn an_empty_mask_ties_to_the_bitmap() {
        assert_eq!(size(&[], PresenceCoding::Bitmap), 0);
        assert_eq!(size(&[], PresenceCoding::ByteRle), 0);
        assert_eq!(smallest(&[], true), PresenceCoding::Bitmap);
    }

    #[test]
    fn a_bitmap_is_forced_when_byte_rle_is_not_allowed() {
        assert_eq!(
            smallest(&mask(4096, |_| false), false),
            PresenceCoding::Bitmap
        );
    }

    #[test]
    fn only_codes_zero_and_one_name_a_coding() {
        let named: Vec<u8> = (0..=u8::MAX)
            .filter(|&c| PresenceCoding::from_code(c).is_some())
            .collect();
        assert_eq!(named, [0, 1]);
    }

    #[test]
    fn split_takes_exactly_what_write_made() {
        for bits in cases() {
            for coding in PresenceCoding::ALL {
                let mut buf = written(&bits, coding);
                buf.extend([0xAA, 0xBB]);
                let (rest, payload) = split(&buf, count_of(&bits), coding).unwrap();
                assert_eq!(rest, [0xAA, 0xBB], "{coding:?} on {bits:?}");
                assert_eq!(payload.len(), size(&bits, coding), "{coding:?} on {bits:?}");
            }
        }
    }

    #[test]
    fn popcount_agrees_with_read() {
        for bits in cases() {
            for coding in PresenceCoding::ALL {
                let buf = written(&bits, coding);
                let expected = u32::try_from(bits.iter().filter(|&&b| b).count()).unwrap();
                assert_eq!(
                    popcount(&buf, count_of(&bits), coding).unwrap(),
                    expected,
                    "{coding:?} on {bits:?}"
                );
            }
        }
    }

    #[rstest]
    #[case::bitmap(PresenceCoding::Bitmap, &[0b1111_1111], 3)]
    #[case::byte_rle_repeat(PresenceCoding::ByteRle, &[0x01, 0xFF], 27)]
    #[case::byte_rle_literal(PresenceCoding::ByteRle, &[0xFE, 0xFF, 0xFF], 11)]
    fn popcount_ignores_padding_bits_past_the_count(
        #[case] coding: PresenceCoding,
        #[case] payload: &[u8],
        #[case] count: u32,
    ) {
        assert_eq!(popcount(payload, count, coding).unwrap(), count);
    }

    #[test]
    fn split_rejects_a_truncated_bitmap() {
        assert!(matches!(
            split(&[1], 16, PresenceCoding::Bitmap),
            Err(MltError::UnableToTake(2))
        ));
    }

    struct Counted(u32);
    impl PresenceBudget for Counted {
        fn reserve_bits(&mut self, count: u32) -> MltResult<()> {
            self.0 += count;
            Ok(())
        }
    }

    #[rstest]
    #[case::two_bytes_naming_four_billion_features(&[0x00], u32::MAX)]
    #[case::two_bytes_expanding_to_130(&[0x7F, 0x00], 1040)]
    fn byte_rle_is_charged_to_the_budget_before_anything_is_built(
        #[case] input: &[u8],
        #[case] count: u32,
    ) {
        let mut budget = Counted(0);
        let _ = read(input, count, PresenceCoding::ByteRle, &mut budget);
        assert_eq!(budget.0, count);
    }

    struct Refused;
    impl PresenceBudget for Refused {
        fn reserve_bits(&mut self, count: u32) -> MltResult<()> {
            Err(MltError::MemoryLimitExceeded {
                limit: 0,
                used: 0,
                requested: count,
            })
        }
    }

    #[test]
    fn a_refused_budget_is_an_error() {
        assert!(matches!(
            read(&[0xFF, 0], 8, PresenceCoding::ByteRle, &mut Refused),
            Err(MltError::MemoryLimitExceeded { requested: 8, .. })
        ));
    }

    mod properties {
        use hegel::generators as gs;

        use super::*;

        #[hegel::composite]
        fn presence_mask(tc: &hegel::TestCase) -> Vec<bool> {
            let len = tc.draw(gs::integers::<usize>().max_value(600));
            match tc.draw(gs::integers::<u8>().max_value(3)) {
                0 => {
                    let per_mille = tc.draw(gs::integers::<u32>().max_value(1000));
                    (0..len)
                        .map(|_| tc.draw(gs::integers::<u32>().max_value(999)) < per_mille)
                        .collect()
                }
                1 => {
                    let mut mask = Vec::with_capacity(len);
                    let mut present = tc.draw(gs::booleans());
                    while mask.len() < len {
                        let run = tc.draw(gs::integers::<usize>().min_value(1).max_value(200));
                        mask.extend(std::iter::repeat_n(present, run.min(len - mask.len())));
                        present = !present;
                    }
                    mask
                }
                2 => vec![tc.draw(gs::booleans()); len],
                _ => {
                    let hot = tc.draw(
                        gs::vecs(gs::integers::<usize>().max_value(len.max(1) - 1)).max_size(4),
                    );
                    (0..len).map(|at| hot.contains(&at)).collect()
                }
            }
        }

        #[hegel::test]
        fn smallest_presence_coding_is_no_larger_than_any_coding(tc: hegel::TestCase) {
            let bits = tc.draw(presence_mask());
            let chosen = size(&bits, smallest(&bits, true));
            for coding in PresenceCoding::ALL {
                assert_eq!(chosen.min(size(&bits, coding)), chosen, "vs {coding:?}");
            }
        }

        #[hegel::test]
        fn every_presence_coding_writes_then_reads_back_the_mask(tc: hegel::TestCase) {
            let bits = tc.draw(presence_mask());
            for coding in PresenceCoding::ALL {
                assert_eq!(read_back(&bits, coding), bits, "{coding:?}");
            }
        }
    }
}
