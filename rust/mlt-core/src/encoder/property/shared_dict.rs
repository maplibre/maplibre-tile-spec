//! Optimizer that groups string columns into shared dictionaries using `MinHash`
//! similarity, then hands off to per-column auto-encoders.

use std::collections::HashMap;

use integer_encoding::VarIntWriter as _;
use probabilistic_collections::SipHasherBuilder;
use probabilistic_collections::similarity::MinHash;
use union_find::{QuickUnionUf, UnionBySize, UnionFind as _};

use crate::MltError::DictIndexOutOfBounds;
use crate::codecs::fsst::compress_fsst_with;
use crate::decoder::strings::{decode_shared_dict_range, encode_shared_dict_range};
use crate::decoder::{PropValue, TileLayer};
use crate::encoder::model::{StrEncoding, StreamCtx};
use crate::encoder::property::strings::{fsst_try_train, write_fsst_data, write_raw_str_data};
use crate::encoder::{
    EncodedStream, Encoder, StagedSharedDict, StagedSharedDictItem, write_u32_stream,
};
use crate::errors::AsMltError as _;
use crate::utils::{AsUsize as _, BinarySerializer as _, checked_sum3, strings_to_lengths};
use crate::{ColumnType, DictRange, DictionaryType, LengthType, MltResult, OffsetType, StreamType};

/// Number of [`MinHash`] permutations. 128 gives ~9 % error on Jaccard estimates.
const MINHASH_PERMUTATIONS: usize = 128;

/// String columns whose estimated Jaccard similarity exceeds this threshold are
/// grouped into a single shared dictionary.
const MINHASH_SIMILARITY_THRESHOLD: f64 = 0.075;

/// A group of string columns to be merged into a single [`crate::encoder::StagedProperty::SharedDict`].
#[derive(Debug, Clone)]
pub struct StringGroup {
    pub prefix: String,
    pub columns: Vec<(String, usize)>,
}

struct StringProfile<'a> {
    col_idx: usize,
    name: &'a str,
    /// `MinHash` over exact string values.
    exact_hashes: Vec<u64>,
    /// `MinHash` over byte trigrams (empty when all strings are shorter than 3 bytes).
    trigram_hashes: Vec<u64>,
}

/// Analyze a [`TileLayer`] and return one [`StringGroup`] per cluster of similar
/// string columns.
#[must_use]
#[hotpath::measure]
pub fn group_string_properties(source: &TileLayer) -> Vec<StringGroup> {
    let exact_mh = MinHash::with_hashers(
        MINHASH_PERMUTATIONS,
        [
            SipHasherBuilder::from_seed(0, 0),
            SipHasherBuilder::from_seed(1, 1),
        ],
    );
    let trigram_mh = MinHash::with_hashers(
        MINHASH_PERMUTATIONS,
        [
            SipHasherBuilder::from_seed(0, 0),
            SipHasherBuilder::from_seed(1, 1),
        ],
    );

    let profiles: Vec<StringProfile<'_>> = source
        .property_names
        .iter()
        .enumerate()
        .filter_map(|(col_idx, name)| {
            let vals: Vec<&str> = source
                .features
                .iter()
                .filter_map(|f| match f.properties.get(col_idx) {
                    Some(PropValue::Str(Some(s))) => Some(s.as_str()),
                    _ => None,
                })
                .collect();
            if vals.is_empty() {
                return None;
            }
            let exact_hashes = exact_mh.get_min_hashes(vals.iter().copied());
            let trigrams: Vec<[u8; 3]> = vals
                .iter()
                .flat_map(|s| s.as_bytes().windows(3).map(|w| [w[0], w[1], w[2]]))
                .collect();
            let trigram_hashes = if trigrams.is_empty() {
                Vec::new()
            } else {
                trigram_mh.get_min_hashes(trigrams.into_iter())
            };
            Some(StringProfile {
                col_idx,
                name,
                exact_hashes,
                trigram_hashes,
            })
        })
        .collect();

    if profiles.is_empty() {
        return Vec::new();
    }

    cluster_by_similarity(profiles)
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

/// Estimate Jaccard similarity from two `MinHash` signature vectors.
#[allow(clippy::cast_precision_loss)]
fn minhash_similarity(a: &[u64], b: &[u64]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let matches = a.iter().zip(b).filter(|(x, y)| x == y).count();
    matches as f64 / a.len() as f64
}

fn cluster_by_similarity(profiles: Vec<StringProfile<'_>>) -> Vec<Vec<StringProfile<'_>>> {
    let n = profiles.len();
    let mut uf = QuickUnionUf::<UnionBySize>::new(n);

    for i in 0..n {
        for j in (i + 1)..n {
            let exact = minhash_similarity(&profiles[i].exact_hashes, &profiles[j].exact_hashes);
            let tri = minhash_similarity(&profiles[i].trigram_hashes, &profiles[j].trigram_hashes);
            if f64::max(exact, tri) > MINHASH_SIMILARITY_THRESHOLD {
                uf.union(i, j);
            }
        }
    }

    let mut groups_map = HashMap::<usize, Vec<StringProfile<'_>>>::new();
    for (i, profile) in profiles.into_iter().enumerate() {
        let root = uf.find(i);
        groups_map.entry(root).or_default().push(profile);
    }

    let mut groups: Vec<Vec<StringProfile<'_>>> = groups_map
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

    pub fn has_presence(&self) -> bool {
        self.has_presence
    }
    #[cfg(feature = "__private")]
    pub fn set_presence(&mut self, value: bool) {
        self.has_presence = value;
    }

    pub fn presence_bools(&self) -> impl ExactSizeIterator<Item = bool> + '_ {
        self.ranges
            .iter()
            .map(|&range| decode_shared_dict_range(range).is_some())
    }

    pub fn dense_spans(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        self.ranges
            .iter()
            .filter_map(|&range| decode_shared_dict_range(range))
    }
}

impl StagedSharedDict {
    /// Build a shared-dictionary column directly from raw per-column string data.
    ///
    /// Each column is a `(suffix, values)` pair where `values` is an iterator of
    /// optional strings (one per feature).  All unique non-null strings across every
    /// column are deduplicated into a shared byte corpus; per-feature byte-range offsets
    /// into that corpus are recorded in each shared-dictionary item.
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
                let mut present_items = 0;
                for opt_val in values {
                    match opt_val {
                        Some(value) => {
                            let idx = *dict_index
                                .get(value.as_ref())
                                .expect("StagedSharedDict::new: value must be present");
                            let (start, end) = dict_ranges[idx as usize];
                            ranges.push(encode_shared_dict_range(start, end)?);
                            present_items += 1;
                        }
                        None => ranges.push(DictRange::NULL),
                    }
                }
                let has_presence = present_items < ranges.len();
                Ok(StagedSharedDictItem {
                    suffix,
                    ranges,
                    has_presence,
                })
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
/// When [`Encoder::override_str_enc`] returns [`None`], auto-selects the corpus encoding (FSST if viable, else plain)
/// and uses automatic offset encoders.
/// When [`Some`], uses the caller-specified encoding and [`Encoder::override_int_enc`] for offsets.
///
/// The caller (staging) is responsible for not creating empty `StagedSharedDict` instances.
/// Always returns `true`.
#[hotpath::measure]
pub(crate) fn write_shared_dict(
    shared_dict: &StagedSharedDict,
    enc: &mut Encoder,
) -> MltResult<bool> {
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
    let str_enc_override = enc.override_str_enc(&shared_dict.prefix);
    let fsst_raw = match str_enc_override {
        Some(StrEncoding::Fsst | StrEncoding::FsstDict) => {
            let byte_slices: Vec<&[u8]> = dict.iter().map(|s| s.as_bytes()).collect();
            let compressor = fsst::Compressor::train(&byte_slices);
            Some(compress_fsst_with(&dict, &compressor))
        }
        Some(StrEncoding::Plain | StrEncoding::Dict) => None,
        None => {
            // Populate cache on first sort trial, reuse on subsequent.
            // Key includes the suffix as otherwise multiple groups could share the same prefix
            // (e.g. two "name:" groups for Arabic vs Cyrillic scripts).
            // Since the grouping is done only once, the order inside the items is deterministic, so we can just take the first suffix for the cache key.
            let first_suffix = shared_dict.items.first().map_or("", |i| &i.suffix);
            enc.fsst_cache
                .entry(format!(
                    "{prefix}{first_suffix}",
                    prefix = shared_dict.prefix
                ))
                .or_insert_with(|| fsst_try_train(&dict))
                .as_ref()
                .map(|c| compress_fsst_with(&dict, c))
        }
    };
    let dict_stream_count = if fsst_raw.is_some() { 4u32 } else { 2u32 };

    let children_count = u32::try_from(shared_dict.items.len())?;
    let optional_count = u32::try_from(
        shared_dict
            .items
            .iter()
            .filter(|p| p.has_presence())
            .count(),
    )?;
    let stream_len = checked_sum3(dict_stream_count, children_count, optional_count)?;

    // Write stream data: total count, corpus streams, then per-child streams.
    enc.write_varint(stream_len)?;
    if let Some(ref raw) = fsst_raw {
        write_fsst_data(raw, DictionaryType::Single, &shared_dict.prefix, enc)?;
    } else {
        let lengths = strings_to_lengths(&dict)?;
        let typ = StreamType::Length(LengthType::Dictionary);
        write_u32_stream(&lengths, &StreamCtx::prop(typ, &shared_dict.prefix), enc)?;
        write_raw_str_data(&dict, DictionaryType::Shared, enc)?;
    }

    ColumnType::SharedDict.write_to(&mut enc.meta)?;
    enc.meta.write_string(&shared_dict.prefix)?;
    enc.meta.write_varint(children_count)?;

    for item in &shared_dict.items {
        if item.has_presence() {
            enc.write_varint(2u32)?;
            ColumnType::OptStr.write_to(&mut enc.meta)?;
            let presence = EncodedStream::encode_presence(item.presence_bools())?;
            enc.write_boolean_stream(&presence)?;
        } else {
            enc.write_varint(1u32)?;
            ColumnType::Str.write_to(&mut enc.meta)?;
        }
        enc.meta.write_string(&item.suffix)?;

        let offsets: Vec<u32> = item
            .dense_spans()
            .map(|span| {
                dict_index
                    .get(&span)
                    .copied()
                    .ok_or(DictIndexOutOfBounds(span.0, dict_spans.len()))
            })
            .collect::<Result<_, _>>()?;
        let typ = StreamType::Offset(OffsetType::String);
        let ctx = StreamCtx::prop2(typ, &shared_dict.prefix, &item.suffix);
        write_u32_stream(&offsets, &ctx, enc)?;
    }

    enc.increment_column_count();
    Ok(true)
}
