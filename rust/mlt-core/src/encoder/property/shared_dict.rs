//! Optimizer that groups string columns into shared dictionaries using `MinHash`
//! similarity, then hands off to per-column auto-encoders.

use std::collections::HashMap;
use std::hash::Hash;

use integer_encoding::VarIntWriter as _;
use probabilistic_collections::similarity::MinHash;
use union_find::{QuickUnionUf, UnionBySize, UnionFind as _};

use crate::MltError::DictIndexOutOfBounds;
use crate::codecs::fsst::compress_fsst;
use crate::decoder::strings::{decode_shared_dict_range, encode_shared_dict_range};
use crate::decoder::{PropValue, TileLayer01};
use crate::encoder::model::ColumnKind::Property;
use crate::encoder::model::StrEncoding;
use crate::encoder::property::model::PresenceKind;
use crate::encoder::property::strings::{fsst_is_viable, write_fsst_data, write_raw_str_data};
use crate::encoder::{
    EncodedStream, Encoder, StagedSharedDict, StagedSharedDictItem, write_u32_stream,
};
use crate::errors::AsMltError as _;
use crate::utils::{AsUsize as _, BinarySerializer as _, checked_sum3, strings_to_lengths};
use crate::{ColumnType, DictionaryType, LengthType, MltResult, OffsetType, StreamType};

/// Number of [`MinHash`] permutations. 128 gives ~7 % error on Jaccard estimates.
const MINHASH_PERMUTATIONS: usize = 128;

/// String columns whose estimated Jaccard similarity exceeds this threshold are
/// grouped into a single shared dictionary.
const MINHASH_SIMILARITY_THRESHOLD: f64 = 0.6;

/// A group of string columns to be merged into a single [`crate::encoder::StagedProperty::SharedDict`].
#[derive(Debug, Clone)]
pub struct StringGroup {
    pub prefix: String,
    pub columns: Vec<(String, usize)>,
}

struct StringProfile<'a> {
    col_idx: usize,
    name: &'a str,
    min_hashes: Vec<u64>,
}

/// Analyze a [`TileLayer01`] and return one [`StringGroup`] per cluster of similar
/// string columns.
#[must_use]
pub fn group_string_properties(source: &TileLayer01) -> Vec<StringGroup> {
    let min_hash = MinHash::new(MINHASH_PERMUTATIONS);

    let profiles: Vec<StringProfile<'_>> = source
        .property_names
        .iter()
        .enumerate()
        .filter_map(|(col_idx, name)| {
            let mut vals = source
                .features
                .iter()
                .filter_map(move |f| match f.properties.get(col_idx) {
                    Some(PropValue::Str(Some(s))) => Some(s.as_str()),
                    _ => None,
                })
                .peekable();
            vals.peek()?;
            Some(StringProfile {
                col_idx,
                name,
                min_hashes: min_hash.get_min_hashes(vals),
            })
        })
        .collect();

    if profiles.is_empty() {
        return Vec::new();
    }

    cluster_by_similarity(profiles, &min_hash)
        .into_iter()
        .map(|group| {
            let prefix = common_prefix_name(&group);
            let columns = group
                .into_iter()
                .map(|p| {
                    let suffix = p.name.strip_prefix(&prefix).unwrap_or(p.name).to_owned();
                    (suffix, p.col_idx)
                })
                .collect();
            StringGroup { prefix, columns }
        })
        .collect()
}

fn cluster_by_similarity<'a, T: Iterator<Item = U>, U: Hash>(
    profiles: Vec<StringProfile<'a>>,
    min_hash: &MinHash<T, U>,
) -> Vec<Vec<StringProfile<'a>>> {
    let n = profiles.len();
    let mut uf = QuickUnionUf::<UnionBySize>::new(n);

    for i in 0..n {
        for j in (i + 1)..n {
            let sim = min_hash
                .get_similarity_from_hashes(&profiles[i].min_hashes, &profiles[j].min_hashes);
            if sim > MINHASH_SIMILARITY_THRESHOLD {
                uf.union(i, j);
            }
        }
    }

    let mut groups_map = HashMap::<usize, Vec<StringProfile<'a>>>::new();
    for (i, profile) in profiles.into_iter().enumerate() {
        let root = uf.find(i);
        groups_map.entry(root).or_default().push(profile);
    }

    let mut groups: Vec<Vec<StringProfile<'a>>> = groups_map
        .into_values()
        .filter_map(|mut v| {
            if v.len() >= 2 {
                v.sort_unstable_by_key(|p| p.col_idx);
                Some(v)
            } else {
                None
            }
        })
        .collect();

    groups.sort_unstable_by_key(|g| g[0].col_idx);
    groups
}

/// Returns the longest common byte prefix of `names`.
fn common_prefix_name(profiles: &[StringProfile<'_>]) -> String {
    debug_assert!(!profiles.is_empty());
    let first = profiles[0].name;
    let mut prefix_len = first.len();
    for p in &profiles[1..] {
        let new_len = first
            .chars()
            .zip(p.name.chars())
            .take_while(|(a, b)| a == b)
            .count();
        prefix_len = prefix_len.min(new_len);
        if prefix_len == 0 {
            return String::new();
        }
    }
    first[..first.floor_char_boundary(prefix_len)].to_owned()
}

impl StagedSharedDict {
    #[must_use]
    pub fn corpus(&self) -> &str {
        &self.data
    }

    #[must_use]
    pub fn get(&self, span: (u32, u32)) -> Option<&str> {
        self.corpus().get(span.0.as_usize()..span.1.as_usize())
    }
}

pub fn collect_staged_shared_dict_spans(items: &[StagedSharedDictItem]) -> Vec<(u32, u32)> {
    let mut spans = items
        .iter()
        .flat_map(StagedSharedDictItem::dense_spans)
        .collect::<Vec<_>>();
    spans.sort_unstable();
    spans.dedup();
    spans
}

impl StagedSharedDictItem {
    #[must_use]
    pub fn feature_count(&self) -> usize {
        self.ranges.len()
    }

    #[must_use]
    pub fn presence(&self) -> PresenceKind {
        let mut has_null = false;
        let mut has_present = false;
        for &range in &self.ranges {
            if decode_shared_dict_range(range).is_none() {
                has_null = true;
            } else {
                has_present = true;
            }
            if has_null && has_present {
                return PresenceKind::Mixed;
            }
        }
        match (has_null, has_present) {
            (false, false) => PresenceKind::Empty,
            (false, true) => PresenceKind::AllPresent,
            (true, false) => PresenceKind::AllNull,
            (true, true) => unreachable!("early return handles Mixed"),
        }
    }

    #[must_use]
    pub fn presence_bools(&self) -> Vec<bool> {
        self.ranges
            .iter()
            .map(|&range| decode_shared_dict_range(range).is_some())
            .collect()
    }

    #[must_use]
    pub fn dense_spans(&self) -> Vec<(u32, u32)> {
        self.ranges
            .iter()
            .filter_map(|&range| decode_shared_dict_range(range))
            .collect()
    }

    #[must_use]
    pub fn get<'a>(&self, shared_dict: &'a StagedSharedDict, i: usize) -> Option<&'a str> {
        self.ranges
            .get(i)
            .copied()
            .and_then(decode_shared_dict_range)
            .and_then(|span| shared_dict.get(span))
    }

    #[must_use]
    pub fn materialize(&self, shared_dict: &StagedSharedDict) -> Vec<Option<String>> {
        self.ranges
            .iter()
            .map(|&range| {
                decode_shared_dict_range(range)
                    .and_then(|span| shared_dict.get(span))
                    .map(str::to_string)
            })
            .collect()
    }
}

impl StagedSharedDict {
    /// Build a shared-dictionary column directly from raw per-column string data.
    ///
    /// Each column is a `(suffix, values)` pair where `values` is an iterator of
    /// optional strings (one per feature).  All unique non-null strings across every
    /// column are deduplicated into a shared byte corpus; per-feature byte-range offsets
    /// into that corpus are recorded in each [`StagedSharedDictItem`].
    pub fn new<S, I, T>(
        prefix: impl Into<String>,
        columns: impl IntoIterator<Item = (S, I)>,
    ) -> MltResult<Self>
    where
        S: Into<String>,
        I: IntoIterator<Item = Option<T>>,
        T: AsRef<str>,
    {
        let prefix = prefix.into();
        // Collect all columns so we can make two passes (dedup then assign ranges).
        let columns: Vec<(String, Vec<Option<T>>)> = columns
            .into_iter()
            .map(|(s, i)| (s.into(), i.into_iter().collect()))
            .collect();

        // First pass: build deduplicated corpus in insertion order.
        let mut dict_index = HashMap::<String, u32>::new();
        let mut dict_ranges = Vec::<(u32, u32)>::new();
        let mut data = String::new();

        for (_, values) in &columns {
            for value in values.iter().filter_map(Option::as_ref) {
                let s = value.as_ref();
                if !dict_index.contains_key(s) {
                    let idx = u32::try_from(dict_ranges.len()).or_overflow()?;
                    let offset = u32::try_from(data.len()).or_overflow()?;
                    let end = offset
                        .checked_add(u32::try_from(s.len()).or_overflow()?)
                        .or_overflow()?;
                    dict_index.insert(s.to_owned(), idx);
                    dict_ranges.push((offset, end));
                    data.push_str(s);
                }
            }
        }

        // Second pass: emit per-feature ranges for each column.
        let items = columns
            .into_iter()
            .map(|(suffix, values)| -> MltResult<StagedSharedDictItem> {
                let mut ranges = Vec::with_capacity(values.len());
                for opt_val in values {
                    match opt_val {
                        Some(value) => {
                            let idx = *dict_index
                                .get(value.as_ref())
                                .expect("StagedSharedDict::new: value must be present");
                            let (start, end) = dict_ranges[idx as usize];
                            ranges.push(encode_shared_dict_range(start, end)?);
                        }
                        None => ranges.push((-1, -1)),
                    }
                }
                Ok(StagedSharedDictItem { suffix, ranges })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            prefix,
            data,
            items,
        })
    }
}

/// Encode a shared-dictionary property and write it to `enc`.
///
/// When [`Encoder::get_str_encoding`] returns [`None`], auto-selects the corpus encoding (FSST if viable, else plain)
/// and uses automatic offset encoders.
/// When [`Some`], uses the caller-specified encoding and [`Encoder::get_int_encoder`] for offsets.
///
/// Returns `false` when every child column is [`PresenceKind::Empty`] or
/// [`PresenceKind::AllNull`] — the whole shared-dict property is skipped.
pub(crate) fn write_shared_dict(
    shared_dict: &StagedSharedDict,
    enc: &mut Encoder,
) -> MltResult<bool> {
    if shared_dict
        .items
        .iter()
        .all(|item| matches!(item.presence(), PresenceKind::Empty | PresenceKind::AllNull))
    {
        return Ok(false);
    }

    let dict_spans = collect_staged_shared_dict_spans(&shared_dict.items);
    let dict: Vec<&str> = dict_spans
        .iter()
        .map(|&span| {
            shared_dict
                .get(span)
                .ok_or(DictIndexOutOfBounds(span.0, dict_spans.len()))
        })
        .collect::<Result<_, _>>()?;
    let dict_index: HashMap<(u32, u32), u32> = dict_spans.iter().copied().zip(0_u32..).collect();

    // Decide corpus encoding upfront to determine the stream count for the varint header.
    // FSST uses 4 streams; plain uses 2.
    let use_fsst = match enc.get_str_encoding(&shared_dict.prefix) {
        Some(StrEncoding::Fsst | StrEncoding::FsstDict) => true,
        Some(StrEncoding::Plain | StrEncoding::Dict) => false,
        None => fsst_is_viable(&dict),
    };
    let dict_stream_count = if use_fsst { 4u32 } else { 2u32 };

    // Determine per-child presence upfront (needed before writing meta + stream count).
    let force_presence = enc.override_presence(Property, &shared_dict.prefix, None);
    let child_has_presence: Vec<bool> = shared_dict
        .items
        .iter()
        .map(|item| {
            item.presence().needs_presence_stream()
                || (force_presence && matches!(item.presence(), PresenceKind::AllPresent))
        })
        .collect();

    // Write column metadata.
    let children_count = u32::try_from(shared_dict.items.len())?;
    let optional_count = u32::try_from(child_has_presence.iter().filter(|&&x| x).count())?;
    let stream_len = checked_sum3(dict_stream_count, children_count, optional_count)?;

    ColumnType::SharedDict.write_to(&mut enc.meta)?;
    enc.meta.write_string(&shared_dict.prefix)?;
    enc.meta.write_varint(children_count)?;
    for (item, &has_presence) in shared_dict.items.iter().zip(&child_has_presence) {
        use ColumnType as CT;
        CT::write_one_of(has_presence, CT::OptStr, CT::Str, &mut enc.meta)?;
        enc.meta.write_string(&item.suffix)?;
    }

    // Write stream data: total count, corpus streams, then per-child streams.
    enc.write_varint(stream_len)?;
    if use_fsst {
        let raw = compress_fsst(&dict);
        write_fsst_data(&raw, DictionaryType::Single, &shared_dict.prefix, enc)?;
    } else {
        let lengths = strings_to_lengths(&dict)?;
        let typ = StreamType::Length(LengthType::Dictionary);
        write_u32_stream(&lengths, typ, Property, &shared_dict.prefix, "lengths", enc)?;
        write_raw_str_data(&dict, DictionaryType::Shared, enc)?;
    }

    for (item, &has_presence) in shared_dict.items.iter().zip(&child_has_presence) {
        let offsets: Vec<u32> = item
            .dense_spans()
            .iter()
            .map(|span| {
                dict_index
                    .get(span)
                    .copied()
                    .ok_or(DictIndexOutOfBounds(span.0, dict_spans.len()))
            })
            .collect::<Result<_, _>>()?;

        enc.write_varint(1 + u32::from(has_presence))?;
        if has_presence {
            let presence = EncodedStream::encode_presence(&item.presence_bools())?;
            enc.write_boolean_stream(&presence)?;
        }
        write_u32_stream(
            &offsets,
            StreamType::Offset(OffsetType::String),
            Property,
            &shared_dict.prefix,
            &item.suffix,
            enc,
        )?;
    }

    enc.increment_column_count();
    Ok(true)
}
