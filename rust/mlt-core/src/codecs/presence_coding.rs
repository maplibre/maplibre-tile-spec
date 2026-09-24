//! The three ways a v2 layer says which features have a value.
//!
//! A bitmap costs `ceil(N/8)` bytes whatever it holds, so it is the cheapest form only
//! while `N` is small or the values are scattered. A column that is present over one run
//! of features, or on a handful of them, carries far less information than that, and the
//! other two codings charge for the information rather than for the feature count.
//!
//! [`Runs`](PresenceCoding::Runs) and [`Indices`](PresenceCoding::Indices) are
//! self-delimiting, so nothing in the wire format writes a length for them.

use std::borrow::Cow;

use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;
use bitvec::vec::BitVec;
use bitvec::view::BitView as _;
use integer_encoding::VarIntWriter as _;
use usize_cast::IntoUsize as _;

use crate::MltError::{PresenceIndexOrder, PresenceRunOverflow, PresenceRunShort};
use crate::MltRefResult;
use crate::codecs::varint::parse_varint;
use crate::utils::take;

/// How a presence bitfield is stored, named by the presence nibble of a column type byte
/// and by the coding byte of a shared bitfield.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
#[repr(u8)]
pub enum PresenceCoding {
    /// `ceil(N/8)` bytes, LSB-first. Bits past `N` are padding and are ignored.
    Bitmap = 1,
    /// Alternating varint run lengths. The first run counts absent features and may be
    /// `0`; the lengths sum to `N`.
    Runs = 2,
    /// A varint count, then that many varints: the first present index, then the gap less
    /// one to each next one.
    Indices = 3,
}

impl PresenceCoding {
    /// Every coding, in the order a tie between them is broken.
    pub(crate) const ALL: [Self; 3] = [Self::Bitmap, Self::Runs, Self::Indices];

    /// The coding a byte names, or [`None`] for one this version has no meaning for.
    #[must_use]
    pub(crate) fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            1 => Some(Self::Bitmap),
            2 => Some(Self::Runs),
            3 => Some(Self::Indices),
            _ => None,
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
        PresenceCoding::Indices => {
            let count = bits.iter().filter(|&&b| b).count() as u64;
            out.write_varint(count)
                .expect("writing to a Vec cannot fail");
            let mut prev: Option<usize> = None;
            for (i, _) in bits.iter().enumerate().filter(|&(_, &b)| b) {
                let gap = match prev {
                    None => i as u64,
                    Some(p) => (i - p - 1) as u64,
                };
                out.write_varint(gap).expect("writing to a Vec cannot fail");
                prev = Some(i);
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
        PresenceCoding::Indices => {
            let mut total = 0;
            let mut prev: Option<usize> = None;
            let mut count = 0u64;
            for (i, _) in bits.iter().enumerate().filter(|&(_, &b)| b) {
                count += 1;
                total += varint_len(match prev {
                    None => i as u64,
                    Some(p) => (i - p - 1) as u64,
                });
                prev = Some(i);
            }
            varint_len(count) + total
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

/// Read `count` presence bits stored in `coding`.
///
/// A bitmap is borrowed from the tile bytes; the other two are built, since their bits are
/// not laid out in memory as bits.
pub(crate) fn read(
    input: &[u8],
    count: u32,
    coding: PresenceCoding,
) -> MltRefResult<'_, Cow<'_, BitSlice<u8, Lsb0>>> {
    let n = count.into_usize();
    match coding {
        PresenceCoding::Bitmap => {
            let (input, bytes) = take(input, count.div_ceil(8))?;
            Ok((input, Cow::Borrowed(&bytes.view_bits::<Lsb0>()[..n])))
        }
        PresenceCoding::Runs => {
            let mut bits = BitVec::<u8, Lsb0>::repeat(false, n);
            let mut input = input;
            let mut at = 0usize;
            let mut present = false;
            while at < n {
                let len: u64;
                (input, len) = parse_varint(input)?;
                let len = usize::try_from(len).map_err(|_| PresenceRunOverflow(count))?;
                let end = at.checked_add(len).ok_or(PresenceRunOverflow(count))?;
                if end > n {
                    return Err(PresenceRunOverflow(count));
                }
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
        PresenceCoding::Indices => {
            let (mut input, len) = parse_varint::<u32>(input)?;
            if len > count {
                return Err(PresenceRunShort(len, count));
            }
            let mut bits = BitVec::<u8, Lsb0>::repeat(false, n);
            let mut prev: Option<usize> = None;
            for _ in 0..len {
                let gap: u32;
                (input, gap) = parse_varint(input)?;
                let at = match prev {
                    None => gap.into_usize(),
                    // Gaps are stored less one, so this is the next index after `p`.
                    Some(p) => p
                        .checked_add(gap.into_usize())
                        .and_then(|v| v.checked_add(1))
                        .ok_or(PresenceIndexOrder(count))?,
                };
                if at >= n {
                    return Err(PresenceIndexOrder(count));
                }
                bits.set(at, true);
                prev = Some(at);
            }
            Ok((input, Cow::Owned(bits)))
        }
    }
}

/// Read `bits` back out of whatever [`write`] put in, for a round-trip check.
#[cfg(test)]
pub(crate) fn round_trip(bits: &[bool], coding: PresenceCoding) -> Vec<bool> {
    let mut buf = Vec::new();
    write(&mut buf, bits, coding);
    assert_eq!(buf.len(), size(bits, coding), "size disagrees with write");
    let count = u32::try_from(bits.len()).unwrap();
    let (rest, out) = read(&buf, count, coding).unwrap();
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
            // a single present feature far in, which Indices is for
            (0..1000).map(|i| i == 777).collect(),
            // alternating, the worst case for both, where Bitmap wins
            (0..64).map(|i| i % 2 == 0).collect(),
            // ends on a present run, so the trailing run is written
            (0..17).map(|i| i > 12).collect(),
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
    fn indices_beat_a_bitmap_on_a_sparse_mask() {
        let bits: Vec<bool> = (0..4096).map(|i| i % 1000 == 0).collect();
        assert_eq!(smallest(&bits), PresenceCoding::Indices);
    }

    #[test]
    fn a_run_past_the_end_is_rejected() {
        let mut buf = Vec::new();
        buf.write_varint(0u64).unwrap();
        buf.write_varint(99u64).unwrap();
        assert!(read(&buf, 8, PresenceCoding::Runs).is_err());
    }

    #[test]
    fn an_index_past_the_end_is_rejected() {
        let mut buf = Vec::new();
        buf.write_varint(1u64).unwrap();
        buf.write_varint(99u64).unwrap();
        assert!(read(&buf, 8, PresenceCoding::Indices).is_err());
    }

    #[test]
    fn more_indices_than_features_is_rejected() {
        let mut buf = Vec::new();
        buf.write_varint(9u64).unwrap();
        assert!(read(&buf, 8, PresenceCoding::Indices).is_err());
    }
}
