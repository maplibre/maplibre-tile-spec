//! Front coding, which stores a sorted dictionary as shared-prefix lengths and the suffixes that follow them.

use usize_cast::IntoUsize as _;

use crate::MltError::MalformedFrontCoding;
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
    ///
    /// One stream keeps the dictionary's stream count the same as a plain dictionary's,
    /// which is what lets the blob's encoding byte alone name the layout.
    /// Grouping by kind rather than interleaving keeps each run homogeneous for the integer codecs.
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
/// Any order round-trips, only worse.
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

/// Rebuild a front-coded dictionary into its entries back to back, with each entry's byte length.
///
/// `lengths` is the combined stream [`FrontCoded::to_lengths`] wrote, so its length is twice
/// the entry count.
pub(crate) fn front_decode(lengths: &[u32], suffixes: &[u8]) -> MltResult<(String, Vec<u32>)> {
    if !lengths.len().is_multiple_of(2) {
        return Err(MalformedFrontCoding(
            "the lengths stream holds a prefix and a suffix length per entry, so it has an even count",
        ));
    }
    let count = lengths.len() / 2;
    let (prefix_lengths, suffix_lengths) = lengths.split_at(count);

    let mut entries = Vec::with_capacity(suffixes.len());
    let mut entry_lengths = Vec::with_capacity(count);
    // Where the previous entry starts in `entries`, so its prefix can be copied forward.
    let mut previous_start = 0_usize;
    let mut consumed = 0_usize;
    for (&shared, &suffix_len) in prefix_lengths.iter().zip(suffix_lengths) {
        let (shared, suffix_len) = (shared.into_usize(), suffix_len.into_usize());
        if shared > entries.len() - previous_start {
            return Err(MalformedFrontCoding(
                "an entry shares more bytes than its predecessor has",
            ));
        }
        let end = consumed
            .checked_add(suffix_len)
            .ok_or(MalformedFrontCoding("suffix lengths overflow"))?;
        let suffix = suffixes.get(consumed..end).ok_or(MalformedFrontCoding(
            "the suffixes run past the end of the blob",
        ))?;
        consumed = end;

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

    fn roundtrip(entries: &[&str]) -> Vec<String> {
        let coded = front_code(entries).unwrap();
        let (corpus, lengths) = front_decode(&coded.to_lengths(), &coded.suffixes).unwrap();
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
    fn decode_rejects_an_odd_lengths_stream() {
        assert!(front_decode(&[0, 1, 2], b"abc").is_err());
    }

    #[test]
    fn decode_rejects_a_prefix_longer_than_its_predecessor() {
        assert!(front_decode(&[0, 9, 2, 1], b"abc").is_err());
    }

    #[test]
    fn decode_rejects_a_suffix_running_past_the_blob() {
        assert!(front_decode(&[0, 0, 2, 9], b"ab").is_err());
    }
}
