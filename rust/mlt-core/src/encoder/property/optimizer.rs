//! Optimizer that analyzes a batch of [`StagedProperty`] values and produces
//! [`Vec<PropertyEncoder>`] with near-optimal per-column encoding settings.
//!
//! # Pipeline
//!
//! 1. **Profile & Group** - compute `MinHash` signatures for string columns and
//!    cluster similar columns into shared dictionaries using union-find.
//! 2. **Transform** - merge grouped string columns into `ParsedProperty::SharedDict`.
//! 3. **Compete & Select** - choose the best `IntEncoder` for integer columns
//!    via `auto_u32` / `auto_u64` pruning-competition; decide between
//!    `Plain` and `Fsst` string encodings using an FSST viability probe;
//!    emit a presence stream only for columns that contain null values (auto-detected).

use std::collections::HashMap;
use std::hash::Hash;

use fsst::Compressor;
use probabilistic_collections::similarity::MinHash;
use union_find::{QuickUnionUf, UnionBySize, UnionFind as _};

use super::encode::encode_properties;
use super::model::{EncodedProperty, SharedDictEncoder, SharedDictItemEncoder, StrEncoder};
use super::strings::collect_staged_shared_dict_spans;
use super::{
    PropertyEncoder, ScalarEncoder, StagedProperty, StagedSharedDict, StagedSharedDictItem,
};
use crate::MltResult;
use crate::codecs::zigzag::encode_zigzag;
use crate::encoder::stream::IntEncoder;
use crate::v01::{PropValue, TileLayer01};

/// Number of [`MinHash`] permutations. 128 gives ~7 % error on Jaccard estimates.
const MINHASH_PERMUTATIONS: usize = 128;

/// String columns whose estimated Jaccard similarity exceeds this threshold are
/// grouped into a single shared dictionary.
const MINHASH_SIMILARITY_THRESHOLD: f64 = 0.6;

/// Minimum total raw byte size of a column before attempting FSST compression.
/// Below this the symbol-table overhead dominates and FSST never wins.
const FSST_OVERHEAD_THRESHOLD: usize = 4_096;

/// Maximum number of strings sampled for the FSST viability probe.
const FSST_SAMPLE_STRINGS: usize = 512;

/// A group of string columns to be merged into a single [`StagedProperty::SharedDict`].
#[derive(Debug, Clone)]
pub struct StringGroup {
    /// Common prefix of all column names in this group.
    pub prefix: String,
    /// `(suffix, col_idx)` pairs: `suffix` is the column name after stripping
    /// `prefix`; `col_idx` is the index into [`crate::v01::TileLayer01::property_names`]
    /// from which this group was derived.
    pub columns: Vec<(String, usize)>,
}

struct StringProfile<'a> {
    col_idx: usize,
    name: &'a str,
    min_hashes: Vec<u64>,
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

/// Analyze a [`TileLayer01`] and return one [`StringGroup`] per cluster of similar
/// string columns.
///
/// Iterates [`TileLayer01::property_names`] and [`TileLayer01::features`] directly.
/// Only columns that carry at least one non-null [`PropValue::Str`] value are profiled;
/// columns where every feature value is absent or null are skipped.  For each pair of
/// remaining string columns whose estimated Jaccard similarity (over unique non-null
/// values) exceeds `MINHASH_SIMILARITY_THRESHOLD`, the pair is union in a
/// union-find structure; groups with fewer than two members are discarded.
///
/// Row order does not affect unique-value membership, so this can be called once
/// before sort-strategy trials and the result reused for all of them via
/// [`crate::v01::StagedLayer01::from_tile`].
///
/// Returns one [`StringGroup`] per `MinHash`-derived cluster of string columns that
/// should share a dictionary.  For each cluster, a common name prefix is computed
/// (via `common_prefix_name`) and stored in [`StringGroup::prefix`], and each
/// member column is recorded in [`StringGroup::columns`] as a `(suffix, index)`
/// pair, where `suffix` is the column name with the common prefix stripped.
///
/// [`StringGroup::columns`] indices refer to positions in [`TileLayer01::property_names`].
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
            // get_min_hashes would fail if the iterator is empty
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

/// Extension trait for consuming-style encoding of staged property columns.
///
/// Each method returns `Vec<Option<EncodedProperty>>` — a `None` entry means
/// the corresponding column was skipped (empty or all-null).
pub trait EncodeProperties: Sized {
    /// Encode with a specific encoder, consuming `self`.
    fn encode(self, encoder: Vec<PropertyEncoder>) -> MltResult<Vec<Option<EncodedProperty>>>;
    /// Automatic encoding, consuming `self`.
    fn encode_auto(self) -> MltResult<(Vec<Option<EncodedProperty>>, Vec<PropertyEncoder>)>;
}

impl EncodeProperties for Vec<StagedProperty> {
    fn encode(self, encoder: Vec<PropertyEncoder>) -> MltResult<Vec<Option<EncodedProperty>>> {
        encode_properties(&self, encoder)
    }

    fn encode_auto(self) -> MltResult<(Vec<Option<EncodedProperty>>, Vec<PropertyEncoder>)> {
        let enc = optimize(&self);
        let encoded = encode_properties(&self, enc.clone())?;
        Ok((encoded, enc))
    }
}

/// Analyze `properties` and return a configured [`Vec<PropertyEncoder>`].
fn optimize(properties: &[StagedProperty]) -> Vec<PropertyEncoder> {
    properties.iter().map(build_encoder).collect()
}

/// Build an encoder for any staged property type.
fn build_encoder(prop: &StagedProperty) -> PropertyEncoder {
    match prop {
        StagedProperty::Bool(_) => PropertyEncoder::Scalar(ScalarEncoder::bool()),
        StagedProperty::F32(_) | StagedProperty::F64(_) => {
            PropertyEncoder::Scalar(ScalarEncoder::float())
        }
        StagedProperty::I8(v) => {
            // FIXME: inaccurate, but encoders don't support i8 widely. Sometimes, plain might be more efficient for this, but is estimated less effective
            let non_null = v
                .values
                .iter()
                .flatten()
                .copied()
                .map(i32::from)
                .collect::<Vec<i32>>();
            let enc = encode_zigzag(&non_null);
            PropertyEncoder::Scalar(ScalarEncoder::int(IntEncoder::auto_u32(&enc)))
        }
        StagedProperty::U8(v) => {
            // FIXME: inaccurate, but encoders don't support u8 widely. Sometimes, plain might be more efficient for this, but is estimated less effective
            let non_null: Vec<u32> = v.values.iter().flatten().copied().map(u32::from).collect();
            PropertyEncoder::Scalar(ScalarEncoder::int(IntEncoder::auto_u32(&non_null)))
        }
        StagedProperty::I32(v) => {
            let non_null = v.values.iter().flatten().copied().collect::<Vec<i32>>();
            let enc = encode_zigzag(&non_null);
            PropertyEncoder::Scalar(ScalarEncoder::int(IntEncoder::auto_u32(&enc)))
        }
        StagedProperty::U32(v) => {
            let non_null: Vec<u32> = v.values.iter().flatten().copied().collect();
            PropertyEncoder::Scalar(ScalarEncoder::int(IntEncoder::auto_u32(&non_null)))
        }
        StagedProperty::I64(v) => {
            let non_null = &v.values.iter().flatten().copied().collect::<Vec<i64>>();
            let enc = encode_zigzag(non_null);
            PropertyEncoder::Scalar(ScalarEncoder::int(IntEncoder::auto_u64(&enc)))
        }
        StagedProperty::U64(v) => {
            let non_null: Vec<u64> = v.values.iter().flatten().copied().collect();
            PropertyEncoder::Scalar(ScalarEncoder::int(IntEncoder::auto_u64(&non_null)))
        }
        StagedProperty::Str(v) => {
            let owned_values = v.dense_values();
            let non_null: Vec<&str> = owned_values.iter().map(String::as_str).collect();
            scalar_str_encoder(&non_null)
        }
        StagedProperty::SharedDict(shared_dict) => build_shared_dict_encoder(shared_dict),
    }
}

/// Build a `SharedDictEncoder` for a `StagedProperty::SharedDict`.
fn build_shared_dict_encoder(shared_dict: &StagedSharedDict) -> PropertyEncoder {
    let dict_spans = collect_staged_shared_dict_spans(&shared_dict.items);
    let all_strings: Vec<&str> = dict_spans
        .iter()
        .filter_map(|&span| shared_dict.get(span))
        .collect();

    let dict_encoder = if fsst_is_viable(&all_strings) {
        StrEncoder::fsst(IntEncoder::varint(), IntEncoder::varint())
    } else {
        let lengths: Vec<u32> = all_strings
            .iter()
            .map(|s| u32::try_from(s.len()).unwrap_or(u32::MAX))
            .collect();
        StrEncoder::Plain {
            string_lengths: IntEncoder::auto_u32(&lengths),
        }
    };

    let item_encoders: Vec<SharedDictItemEncoder> = shared_dict
        .items
        .iter()
        .map(|item| SharedDictItemEncoder::new(compute_offset_encoder(&shared_dict.items, item)))
        .collect();

    SharedDictEncoder {
        dict_encoder,
        items: item_encoders,
    }
    .into()
}

/// Compute the optimal `IntEncoder` for the offset stream of one item
/// in a staged shared dictionary.
fn compute_offset_encoder(
    items: &[StagedSharedDictItem],
    target_item: &StagedSharedDictItem,
) -> IntEncoder {
    let dict_index: HashMap<(u32, u32), u32> = collect_staged_shared_dict_spans(items)
        .into_iter()
        .zip(0_u32..)
        .collect();
    let offsets: Vec<u32> = target_item
        .dense_spans()
        .iter()
        .map(|span| {
            *dict_index
                .get(span)
                .expect("non-null string span missing from shared dictionary")
        })
        .collect();

    if offsets.is_empty() {
        IntEncoder::plain()
    } else {
        IntEncoder::auto_u32(&offsets)
    }
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

/// Choose between `Plain` and `Fsst` for a standalone string column.
fn scalar_str_encoder(non_null: &[&str]) -> PropertyEncoder {
    let lengths: Vec<u32> = non_null
        .iter()
        .map(|s| u32::try_from(s.len()).unwrap_or(u32::MAX))
        .collect();
    if fsst_is_viable(non_null) {
        PropertyEncoder::Scalar(ScalarEncoder::str_fsst(
            IntEncoder::varint(),
            IntEncoder::auto_u32(&lengths),
        ))
    } else {
        PropertyEncoder::Scalar(ScalarEncoder::str(IntEncoder::auto_u32(&lengths)))
    }
}

/// Returns `true` when FSST compression is likely to save space on `strings`.
fn fsst_is_viable(strings: &[&str]) -> bool {
    if strings.is_empty() {
        return false;
    }
    let sample = if strings.len() <= FSST_SAMPLE_STRINGS {
        strings
    } else {
        &strings[..FSST_SAMPLE_STRINGS]
    };
    let plain_size: usize = sample.iter().map(|s| s.len()).sum();
    if plain_size < FSST_OVERHEAD_THRESHOLD {
        return false;
    }
    let byte_slices: Vec<&[u8]> = sample.iter().map(|s| s.as_bytes()).collect();
    let compressor = Compressor::train(&byte_slices);
    let symbols = compressor.symbol_table();
    let symbol_lengths = compressor.symbol_lengths();
    let symbol_overhead: usize = symbol_lengths
        .iter()
        .take(symbols.len())
        .map(|&l| usize::from(l))
        .sum();
    let compressed_size: usize = sample
        .iter()
        .map(|s| compressor.compress(s.as_bytes()).len())
        .sum();
    symbol_overhead + compressed_size < plain_size
}
