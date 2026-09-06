//! Front coding, which stores a sorted dictionary as shared-prefix lengths and the suffixes that follow them.

use usize_cast::IntoUsize as _;

use crate::MltError::{
    FrontCodedOddLengthCount, FrontCodedPrefixTooLong, FrontCodedSuffixOutOfBounds,
};
use crate::MltResult;

/// A sorted dictionary with each entry's prefix shared with its predecessor factored out.
///
/// Reconstruction is sequential: entry `i` is the first `prefix_lengths[i]` bytes of entry
/// `i - 1` followed by its own suffix.
/// Prefixes are measured in bytes and may split a character, since only the concatenation
/// has to be valid UTF-8.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FrontCoded {
    /// Bytes entry `i` shares with entry `i - 1`, always `0` for the first entry.
    pub prefix_lengths: Vec<u32>,
    /// Bytes of entry `i` that follow its shared prefix.
    pub suffix_lengths: Vec<u32>,
    /// The suffixes back to back.
    pub suffixes: Vec<u8>,
}

impl FrontCoded {
    /// The two length runs as one stream, all prefix lengths then all suffix lengths.
    pub(crate) fn to_lengths(&self) -> Vec<u32> {
        let mut lengths = Vec::with_capacity(self.prefix_lengths.len() * 2);
        lengths.extend_from_slice(&self.prefix_lengths);
        lengths.extend_from_slice(&self.suffix_lengths);
        lengths
    }
}

/// Factor out each entry's prefix shared with its predecessor.
///
/// `sorted` is expected in lexicographic order, which is what makes neighbours share prefixes.
pub(crate) fn front_code(sorted: &[&str]) -> MltResult<FrontCoded> {
    let mut coded = FrontCoded {
        prefix_lengths: Vec::with_capacity(sorted.len()),
        suffix_lengths: Vec::with_capacity(sorted.len()),
        suffixes: Vec::new(),
    };
    let mut previous: &[u8] = b"";
    for entry in sorted {
        let entry = entry.as_bytes();
        let shared = common_prefix_len(previous, entry);
        coded.prefix_lengths.push(u32::try_from(shared)?);
        coded
            .suffix_lengths
            .push(u32::try_from(entry.len() - shared)?);
        coded.suffixes.extend_from_slice(&entry[shared..]);
        previous = entry;
    }
    Ok(coded)
}

/// The two length runs of a front-coded dictionary, split out of the one stream that carries them.
///
/// [`Self::split`] is the only place the stream's even count is checked, so walking the pairs
/// cannot meet a prefix length that has no suffix length beside it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FrontLengths<'a> {
    prefixes: &'a [u32],
    suffixes: &'a [u32],
}

impl<'a> FrontLengths<'a> {
    /// Split the combined stream [`FrontCoded::to_lengths`] wrote back into its two runs.
    pub(crate) fn split(lengths: &'a [u32]) -> MltResult<Self> {
        if !lengths.len().is_multiple_of(2) {
            return Err(FrontCodedOddLengthCount(lengths.len()));
        }
        let (prefixes, suffixes) = lengths.split_at(lengths.len() / 2);
        Ok(Self { prefixes, suffixes })
    }

    /// How many entries the dictionary holds.
    fn count(self) -> usize {
        self.prefixes.len()
    }

    /// Each entry's shared prefix length beside its suffix length.
    fn pairs(self) -> impl Iterator<Item = (usize, usize)> + 'a {
        self.prefixes
            .iter()
            .zip(self.suffixes)
            .map(|(&prefix, &suffix)| (prefix.into_usize(), suffix.into_usize()))
    }
}

/// Rebuild a front-coded dictionary into its entries back to back, with each entry's byte length.
pub(crate) fn front_decode(
    lengths: FrontLengths<'_>,
    suffixes: &[u8],
) -> MltResult<(String, Vec<u32>)> {
    let mut entries = Vec::with_capacity(suffixes.len());
    let mut entry_lengths = Vec::with_capacity(lengths.count());
    // Where the previous entry starts in `entries`, so its prefix can be copied forward.
    let mut previous_start = 0_usize;
    // Shrinking the remainder rather than tracking an offset keeps the suffix cursor in bounds.
    let mut remaining = suffixes;
    for (index, (shared, suffix_len)) in lengths.pairs().enumerate() {
        let available = entries.len() - previous_start;
        if shared > available {
            return Err(FrontCodedPrefixTooLong {
                index,
                shared,
                available,
            });
        }
        let (suffix, rest) =
            remaining
                .split_at_checked(suffix_len)
                .ok_or(FrontCodedSuffixOutOfBounds {
                    index,
                    needed: suffix_len,
                    available: remaining.len(),
                })?;
        remaining = rest;

        let start = entries.len();
        entries.extend_from_within(previous_start..previous_start + shared);
        entries.extend_from_slice(suffix);
        previous_start = start;
        entry_lengths.push(u32::try_from(shared + suffix_len)?);
    }
    Ok((String::from_utf8(entries)?, entry_lengths))
}

/// How many leading bytes two entries share.
fn common_prefix_len(a: &[u8], b: &[u8]) -> usize {
    a.iter().zip(b).take_while(|(x, y)| x == y).count()
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn decode(lengths: &[u32], suffixes: &[u8]) -> MltResult<(String, Vec<u32>)> {
        front_decode(FrontLengths::split(lengths)?, suffixes)
    }

    fn roundtrip(entries: &[&str]) -> Vec<String> {
        let coded = front_code(entries).unwrap();
        let (corpus, lengths) = decode(&coded.to_lengths(), &coded.suffixes).unwrap();
        let mut out = Vec::new();
        let mut at = 0_usize;
        for len in lengths {
            let len = len as usize;
            out.push(corpus[at..at + len].to_string());
            at += len;
        }
        out
    }

    #[rstest]
    #[case::empty(&[])]
    #[case::single(&["amsterdam"])]
    #[case::no_shared_prefix(&["alpha", "beta", "gamma"])]
    #[case::deep_shared_prefix(&["name", "name:de", "name:de:formal", "name:en"])]
    #[case::entry_is_a_prefix_of_the_next(&["san", "san jose", "san jose del cabo"])]
    #[case::empty_entry_first(&["", "a", "ab"])]
    #[case::repeated_neighbours(&["dup", "dup", "dup"])]
    #[case::multibyte_shared_prefix(&["Ünterföhring", "Ünterhaching", "Ünterschleißheim"])]
    #[case::shared_prefix_splits_a_character(&["日本橋", "日本語"])]
    #[case::suffixes_are_not_valid_utf8_alone(&["aé", "aê"])]
    #[case::unsorted_input(&["zebra", "apple", "zebu"])]
    fn front_coding_roundtrips(#[case] entries: &[&str]) {
        assert_eq!(roundtrip(entries), entries);
    }

    #[test]
    fn a_shared_prefix_is_stored_once() {
        let coded = front_code(&["name:de", "name:de:formal"]).unwrap();
        assert_eq!(coded.prefix_lengths, [0, 7]);
        assert_eq!(coded.suffix_lengths, [7, 7]);
        assert_eq!(coded.suffixes, b"name:de:formal");
    }

    #[test]
    fn lengths_carry_the_prefixes_then_the_suffixes() {
        let coded = front_code(&["ab", "abc"]).unwrap();
        assert_eq!(coded.to_lengths(), [0, 2, 2, 1]);
    }

    #[test]
    fn a_suffix_blob_need_not_be_valid_utf8() {
        let coded = front_code(&["aé", "aê"]).unwrap();
        assert_eq!(coded.suffixes, [0x61, 0xC3, 0xA9, 0xAA]);
        assert!(str::from_utf8(&coded.suffixes).is_err());
    }

    #[test]
    fn splitting_rejects_an_odd_lengths_stream() {
        let err = FrontLengths::split(&[0, 1, 2]).unwrap_err();
        assert!(matches!(err, FrontCodedOddLengthCount(3)), "{err:?}");
    }

    #[test]
    fn decode_rejects_a_prefix_longer_than_its_predecessor() {
        let err = decode(&[0, 9, 2, 1], b"abc").unwrap_err();
        assert!(
            matches!(
                err,
                FrontCodedPrefixTooLong {
                    index: 1,
                    shared: 9,
                    available: 2
                }
            ),
            "{err:?}"
        );
    }

    #[test]
    fn decode_rejects_a_suffix_running_past_the_blob() {
        let err = decode(&[0, 0, 2, 9], b"ab").unwrap_err();
        assert!(
            matches!(
                err,
                FrontCodedSuffixOutOfBounds {
                    index: 1,
                    needed: 9,
                    available: 0
                }
            ),
            "{err:?}"
        );
    }
}
