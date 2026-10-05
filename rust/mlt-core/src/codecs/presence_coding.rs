//! The three ways a v2 layer says which features have a value.
//!
//! A bitmap costs `ceil(N/8)` bytes whatever it holds, so it is the cheapest form only
//! while `N` is small or the values are scattered. A column that is present over one run
//! of features, or on a handful of them, carries far less information than that, and the
//! other two codings charge for the information rather than for the feature count.
//!
//! [`Sparse`](PresenceCoding::Sparse) is a bitmap of the bitmap's non-zero bytes followed by
//! those bytes, so a column pays for the bytes it uses and one bit per eight others.
//!
//! [`Runs`](PresenceCoding::Runs) and [`Sparse`](PresenceCoding::Sparse) are self-delimiting, so nothing in the wire format writes a length for them.

use std::borrow::Cow;

use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;
use bitvec::vec::BitVec;
use bitvec::view::BitView as _;
use integer_encoding::VarIntWriter as _;
use usize_cast::IntoUsize as _;

use crate::MltError::{self, PresenceRunOverflow, PresenceSparseByte, PresenceSparseSummary};
use crate::codecs::varint::parse_varint;
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
#[serde(rename_all = "lowercase")]
#[repr(u8)]
pub enum PresenceCoding {
    /// `ceil(N/8)` bytes, LSB-first. Bits past `N` are padding and are ignored.
    Bitmap = 0,
    /// Alternating varint run lengths. The first run counts absent features and may be
    /// `0`; the lengths sum to `N`.
    Runs = 1,
    /// A bitmap with one bit per byte of the plain bitmap, set where that byte is not
    /// zero, followed by the non-zero bytes in order.
    Sparse = 2,
}

impl PresenceCoding {
    /// Every coding, in the order a tie between them is broken.
    pub(crate) const ALL: [Self; 3] = [Self::Bitmap, Self::Runs, Self::Sparse];

    /// The coding a `Bool` stream's logical field numbers `code`, or [`None`] for one this version has no meaning for.
    #[must_use]
    pub(crate) fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Bitmap),
            1 => Some(Self::Runs),
            2 => Some(Self::Sparse),
            _ => None,
        }
    }
}

impl From<PresenceCoding> for BoolLogical {
    fn from(coding: PresenceCoding) -> Self {
        match coding {
            PresenceCoding::Bitmap => Self::None,
            PresenceCoding::Runs => Self::Runs,
            PresenceCoding::Sparse => Self::Sparse,
        }
    }
}

impl PresenceCoding {
    /// The coding a bool stream's logical encoding names, or [`None`] for v1's byte-RLE.
    #[must_use]
    pub(crate) fn of_logical(logical: BoolLogical) -> Option<Self> {
        match logical {
            BoolLogical::None => Some(Self::Bitmap),
            BoolLogical::Runs => Some(Self::Runs),
            BoolLogical::Sparse => Some(Self::Sparse),
            BoolLogical::ByteRle(_) => None,
        }
    }
}

/// Runs of equal bits, as `(is_present, length)` pairs starting from the absent run that
/// may be empty. This is the shape both [`PresenceCoding::Runs`] and the size estimate use.
fn runs(bits: &[bool]) -> impl Iterator<Item = (bool, u64)> + '_ {
    let mut at = 0usize;
    // The first run is the absent one, so a mask that starts present opens with a zero.
    let mut want = false;
    std::iter::from_fn(move || {
        if at >= bits.len() {
            return None;
        }
        let mut len = 0u64;
        while at < bits.len() && bits[at] == want {
            len += 1;
            at += 1;
        }
        want = !want;
        Some((!want, len))
    })
}

/// Write `bits` in `coding`, appending to `out`.
///
/// Never fails: writing a varint into a `Vec` cannot.
pub(crate) fn write(out: &mut Vec<u8>, bits: &[bool], coding: PresenceCoding) {
    match coding {
        PresenceCoding::Bitmap => {
            let start = out.len();
            out.resize(start + bits.len().div_ceil(8), 0);
            for (i, &bit) in bits.iter().enumerate() {
                if bit {
                    out[start + i / 8] |= 1 << (i % 8);
                }
            }
        }
        PresenceCoding::Runs => {
            for (_, len) in runs(bits) {
                out.write_varint(len).expect("writing to a Vec cannot fail");
            }
        }
        PresenceCoding::Sparse => {
            let summary = out.len();
            out.resize(summary + bits.len().div_ceil(8).div_ceil(8), 0);
            for (i, chunk) in bits.chunks(8).enumerate() {
                let byte = chunk
                    .iter()
                    .enumerate()
                    .fold(0u8, |acc, (j, &on)| acc | u8::from(on) << j);
                if byte != 0 {
                    out[summary + i / 8] |= 1 << (i % 8);
                    out.push(byte);
                }
            }
        }
    }
}

/// What `bits` would take in `coding`, without writing it.
#[must_use]
pub(crate) fn size(bits: &[bool], coding: PresenceCoding) -> usize {
    fn varint_len(mut v: u64) -> usize {
        let mut n = 1;
        while v >= 0x80 {
            v >>= 7;
            n += 1;
        }
        n
    }
    match coding {
        PresenceCoding::Bitmap => bits.len().div_ceil(8),
        PresenceCoding::Runs => runs(bits).map(|(_, len)| varint_len(len)).sum(),
        PresenceCoding::Sparse => {
            let non_zero = bits.chunks(8).filter(|c| c.iter().any(|&b| b)).count();
            bits.len().div_ceil(8).div_ceil(8) + non_zero
        }
    }
}

/// The smallest coding for `bits`, ties broken toward the lowest code so that the same
/// mask always leaves the same bytes.
#[must_use]
pub(crate) fn smallest(bits: &[bool]) -> PresenceCoding {
    PresenceCoding::ALL
        .into_iter()
        .min_by_key(|&c| (size(bits, c), c as u8))
        .expect("ALL is not empty")
}

/// Read `count` presence bits stored in `coding`, charging what it builds to `budget`.
///
/// A bitmap is borrowed from the tile bytes, and needs `ceil(count/8)` of them to be there
/// at all, so the input bounds it. Runs and indices are built, and a handful of bytes can
/// ask for any `count` the layer declared, so the allocation is reserved before it is made
/// rather than trusted.
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
        PresenceCoding::Sparse => {
            let bytes = n.div_ceil(8);
            let (input, summary) = take(input, count.div_ceil(8).div_ceil(8))?;
            let summary = summary.view_bits::<Lsb0>();
            if summary[bytes..].any() {
                return Err(PresenceSparseSummary(count));
            }
            let (input, stored) = take(input, u32::try_from(summary.count_ones())?)?;
            budget.reserve_bits(count)?;
            let mut plain = vec![0u8; bytes];
            let mut stored = stored.iter();
            for at in summary[..bytes].iter_ones() {
                let byte = *stored.next().expect("one stored byte per set summary bit");
                if byte == 0 {
                    return Err(PresenceSparseByte(count));
                }
                plain[at] = byte;
            }
            let mut bits = BitVec::<u8, Lsb0>::from_vec(plain);
            bits.truncate(n);
            Ok((input, Cow::Owned(bits)))
        }
        PresenceCoding::Runs => {
            budget.reserve_bits(count)?;
            let mut bits = BitVec::<u8, Lsb0>::repeat(false, n);
            let mut input = input;
            let mut at = 0usize;
            let mut present = false;
            while at < n {
                let len: u64;
                (input, len) = parse_varint(input)?;
                let end = usize::try_from(len)
                    .ok()
                    .and_then(|len| at.checked_add(len))
                    .filter(|&end| end <= n)
                    .ok_or(PresenceRunOverflow(count))?;
                if present {
                    bits[at..end].fill(true);
                }
                at = end;
                present = !present;
            }
            // `at` can only leave the loop equal to `n`, but a mask that ends early would
            // have run out of input first, which `parse_varint` reports.
            Ok((input, Cow::Owned(bits)))
        }
    }
}

/// Split off the bytes `count` bits take in `coding`, without building them.
///
/// Every coding delimits itself, so this is how a reader finds where a stream's payload ends.
pub(crate) fn split(input: &[u8], count: u32, coding: PresenceCoding) -> MltRefResult<'_, &[u8]> {
    let n = count.into_usize();
    let len = match coding {
        PresenceCoding::Bitmap => n.div_ceil(8),
        PresenceCoding::Sparse => {
            let summary = n.div_ceil(8).div_ceil(8);
            let head = input.get(..summary).ok_or(MltError::UnableToTake(count))?;
            summary + head.view_bits::<Lsb0>().count_ones()
        }
        PresenceCoding::Runs => {
            let mut rest = input;
            let mut at = 0usize;
            while at < n {
                let len: u64;
                (rest, len) = parse_varint(rest)?;
                at = usize::try_from(len)
                    .ok()
                    .and_then(|len| at.checked_add(len))
                    .filter(|&end| end <= n)
                    .ok_or(PresenceRunOverflow(count))?;
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
    let bits = match coding {
        PresenceCoding::Bitmap => payload
            .get(..n.div_ceil(8))
            .ok_or(MltError::UnableToTake(count))?
            .view_bits::<Lsb0>()[..n]
            .count_ones(),
        PresenceCoding::Sparse => {
            let bytes = n.div_ceil(8);
            let summary = bytes.div_ceil(8);
            let stored = payload
                .get(summary..)
                .ok_or(MltError::UnableToTake(count))?;
            let mut stored = stored.iter();
            let mut total = 0usize;
            for at in payload.view_bits::<Lsb0>()[..bytes].iter_ones() {
                let byte = *stored.next().ok_or(MltError::UnableToTake(count))?;
                let valid = (n - at * 8).min(8);
                total += (byte & (u8::MAX >> (8 - valid))).count_ones() as usize;
            }
            total
        }
        PresenceCoding::Runs => {
            let mut rest = payload;
            let mut at = 0usize;
            let mut present = false;
            let mut total = 0usize;
            while at < n {
                let len: u64;
                (rest, len) = parse_varint(rest)?;
                let len = usize::try_from(len).map_err(|_| PresenceRunOverflow(count))?;
                at = at
                    .checked_add(len)
                    .filter(|&end| end <= n)
                    .ok_or(PresenceRunOverflow(count))?;
                if present {
                    total += len;
                }
                present = !present;
            }
            total
        }
    };
    Ok(u32::try_from(bits)?)
}

/// Read `bits` back out of whatever [`write`] put in, for a round-trip check.
#[cfg(test)]
pub(crate) fn round_trip(bits: &[bool], coding: PresenceCoding) -> Vec<bool> {
    let mut buf = Vec::new();
    write(&mut buf, bits, coding);
    assert_eq!(buf.len(), size(bits, coding), "size disagrees with write");
    let count = u32::try_from(bits.len()).unwrap();
    let (rest, out) = read(&buf, count, coding, &mut Unmetered).unwrap();
    assert!(rest.is_empty(), "{coding:?} left {} bytes", rest.len());
    out.iter().by_vals().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cases() -> Vec<Vec<bool>> {
        vec![
            vec![],
            vec![true],
            vec![false],
            vec![true; 9],
            vec![false; 9],
            // one contiguous present block, which Runs is for
            (0..100).map(|i| (20..60).contains(&i)).collect(),
            // a single present feature far in
            (0..1000).map(|i| i == 777).collect(),
            // alternating, the worst case for both, where Bitmap wins
            (0..64).map(|i| i % 2 == 0).collect(),
            // ends on a present run, so the trailing run is written
            (0..17).map(|i| i > 12).collect(),
            // scattered bits in a few bytes of a long mask, which Sparse is for
            (0..4096)
                .map(|i| matches!(i, 5 | 6 | 1000 | 3000))
                .collect(),
            // a bitmap whose summary ends mid-byte
            (0..70).map(|i| i % 13 == 0).collect(),
        ]
    }

    #[test]
    fn every_coding_round_trips() {
        for bits in cases() {
            for coding in PresenceCoding::ALL {
                assert_eq!(round_trip(&bits, coding), bits, "{coding:?} on {bits:?}");
            }
        }
    }

    #[test]
    fn smallest_is_the_smallest() {
        for bits in cases() {
            let picked = smallest(&bits);
            let best = PresenceCoding::ALL
                .into_iter()
                .map(|c| size(&bits, c))
                .min()
                .unwrap();
            assert_eq!(size(&bits, picked), best, "on {bits:?}");
        }
    }

    #[test]
    fn sparse_beats_every_other_coding_on_clustered_scatter() {
        let bits: Vec<bool> = (0..1920).map(|i| (i / 8) % 12 == 5 && i % 2 == 0).collect();
        assert_eq!(smallest(&bits), PresenceCoding::Sparse);
    }

    #[test]
    fn a_sparse_summary_past_the_bitmap_is_rejected() {
        assert!(matches!(
            read(&[0b10, 1], 8, PresenceCoding::Sparse, &mut Unmetered),
            Err(PresenceSparseSummary(8))
        ));
    }

    #[test]
    fn a_stored_zero_byte_is_rejected() {
        assert!(matches!(
            read(&[0b1, 0], 8, PresenceCoding::Sparse, &mut Unmetered),
            Err(PresenceSparseByte(8))
        ));
    }

    #[test]
    fn a_missing_sparse_byte_is_rejected() {
        assert!(read(&[0b1], 8, PresenceCoding::Sparse, &mut Unmetered).is_err());
    }

    #[test]
    fn a_tie_goes_to_the_lowest_code() {
        // Every coding costs one byte here, so the bitmap has to win.
        assert_eq!(smallest(&[true]), PresenceCoding::Bitmap);
    }

    #[test]
    fn runs_beat_a_bitmap_on_one_block() {
        let bits: Vec<bool> = (0..4096).map(|i| (100..200).contains(&i)).collect();
        assert_eq!(smallest(&bits), PresenceCoding::Runs);
        assert!(size(&bits, PresenceCoding::Runs) < size(&bits, PresenceCoding::Bitmap));
    }

    #[test]
    fn a_run_past_the_end_is_rejected() {
        let mut buf = Vec::new();
        buf.write_varint(0u64).unwrap();
        buf.write_varint(99u64).unwrap();
        assert!(read(&buf, 8, PresenceCoding::Runs, &mut Unmetered).is_err());
    }

    #[test]
    fn a_run_overflowing_the_position_is_rejected() {
        let mut buf = Vec::new();
        buf.write_varint(1u64).unwrap();
        buf.write_varint(u64::MAX).unwrap();
        assert!(matches!(
            read(&buf, 8, PresenceCoding::Runs, &mut Unmetered),
            Err(PresenceRunOverflow(8))
        ));
    }

    #[test]
    fn only_codes_zero_to_two_name_a_coding() {
        let named: Vec<u8> = (0..=u8::MAX)
            .filter(|&c| PresenceCoding::from_code(c).is_some())
            .collect();
        assert_eq!(named, [0, 1, 2]);
    }

    #[test]
    fn split_takes_exactly_what_write_made() {
        for bits in cases() {
            for coding in PresenceCoding::ALL {
                let mut buf = Vec::new();
                write(&mut buf, &bits, coding);
                buf.extend([0xAA, 0xBB]);
                let count = u32::try_from(bits.len()).unwrap();
                let (rest, payload) = split(&buf, count, coding).unwrap();
                assert_eq!(rest, [0xAA, 0xBB], "{coding:?} on {bits:?}");
                assert_eq!(payload.len(), size(&bits, coding), "{coding:?} on {bits:?}");
            }
        }
    }

    #[test]
    fn popcount_agrees_with_read() {
        for bits in cases() {
            for coding in PresenceCoding::ALL {
                let mut buf = Vec::new();
                write(&mut buf, &bits, coding);
                let count = u32::try_from(bits.len()).unwrap();
                let expected = u32::try_from(bits.iter().filter(|&&b| b).count()).unwrap();
                assert_eq!(
                    popcount(&buf, count, coding).unwrap(),
                    expected,
                    "{coding:?} on {bits:?}"
                );
            }
        }
    }

    #[test]
    fn popcount_ignores_padding_bits_past_the_count() {
        assert_eq!(
            popcount(&[0b1111_1111], 3, PresenceCoding::Bitmap).unwrap(),
            3
        );
        assert_eq!(
            popcount(&[0b1, 0b1111_1111], 3, PresenceCoding::Sparse).unwrap(),
            3
        );
    }

    #[test]
    fn split_rejects_a_truncated_payload() {
        assert!(split(&[0b11, 1], 16, PresenceCoding::Sparse).is_err());
        assert!(split(&[1], 16, PresenceCoding::Bitmap).is_err());
        assert!(split(&[3], 16, PresenceCoding::Runs).is_err());
    }

    /// Counts the bits a reader was asked to build, standing in for a real budget.
    struct Counted(u32);
    impl PresenceBudget for Counted {
        fn reserve_bits(&mut self, count: u32) -> MltResult<()> {
            self.0 += count;
            Ok(())
        }
    }

    #[rstest::rstest]
    #[case::runs(PresenceCoding::Runs)]
    fn a_tiny_payload_cannot_ask_for_a_huge_allocation_unmetered(#[case] coding: PresenceCoding) {
        // Two bytes naming four billion features: the budget has to hear about it
        // before anything is allocated, or a tile of nothing OOMs the reader.
        let mut buf = Vec::new();
        buf.write_varint(0u64).unwrap();
        let mut budget = Counted(0);
        let _ = read(&buf, u32::MAX, coding, &mut budget);
        assert_eq!(
            budget.0,
            u32::MAX,
            "{coding:?} allocated without charging the budget"
        );
    }

    #[test]
    fn a_sparse_summary_is_charged_to_the_budget_before_the_bitmap_is_built() {
        let mut budget = Counted(0);
        let summary = [0u8; 64];
        let (rest, bits) = read(&summary, 4096, PresenceCoding::Sparse, &mut budget).unwrap();
        assert_eq!((rest.len(), bits.len(), budget.0), (0, 4096, 4096));
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

    #[rstest::rstest]
    #[case::runs(PresenceCoding::Runs)]
    #[case::sparse(PresenceCoding::Sparse)]
    fn a_refused_budget_is_an_error(#[case] coding: PresenceCoding) {
        assert!(matches!(
            read(&[0], 8, coding, &mut Refused),
            Err(MltError::MemoryLimitExceeded { requested: 8, .. })
        ));
    }

    #[test]
    fn a_short_bitmap_is_rejected() {
        assert!(matches!(
            read(&[0], 9, PresenceCoding::Bitmap, &mut Unmetered),
            Err(MltError::UnableToTake(2))
        ));
    }
}
