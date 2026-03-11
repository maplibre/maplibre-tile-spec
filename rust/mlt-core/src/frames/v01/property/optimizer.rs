//! Optimizer that analyses a batch of [`DecodedProperty`] values and produces
//! [`Vec<PropertyEncoder>`] with near-optimal per-column encoding settings.
//!
//! # Pipeline
//!
//! 1. **Profile & Group** - compute `MinHash` signatures for string columns and
//!    cluster similar columns into shared dictionaries using union-find.
//! 2. **Transform** - merge grouped string columns into `DecodedProperty::SharedDict`.
//! 3. **Compete & Select** - choose the best `IntEncoder` for integer columns
//!    via `auto_u32` / `auto_u64` pruning-competition; decide between
//!    `Plain` and `Fsst` string encodings using an FSST viability probe;
//!    set `PresenceStream::Absent` for columns that have no null values.

use std::collections::hash_set::IntoIter;
use std::collections::{HashMap, HashSet};

use fsst::Compressor;
use probabilistic_collections::similarity::MinHash;
use union_find::{QuickUnionUf, UnionBySize, UnionFind as _};

use crate::optimizer::{AutomaticOptimisation, ManualOptimisation, ProfileOptimisation};
use crate::utils::encode_zigzag;
use crate::v01::property::strings::{build_decoded_shared_dict, collect_shared_dict_spans};
use crate::v01::property::{
    DecodedProperty, DecodedSharedDict, DecodedSharedDictItem, PresenceStream, PropertyEncoder,
    ScalarEncoder,
};
use crate::v01::stream::IntEncoder;
use crate::v01::{
    OwnedEncodedProperty, OwnedProperty, SharedDictEncoder, SharedDictItemEncoder, StrEncoder,
};
use crate::{FromDecoded as _, MltError};

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

/// Statistics collected during Phase 1 for a single string-typed column.
struct StringProfile {
    /// Index of this column in the original `properties` slice.
    col_idx: usize,
    /// `MinHash` signature computed over the set of unique non-null values.
    /// Empty when the column contains no non-null values (all-null column).
    min_hashes: Vec<u64>,
}

/// A pre-computed set of string column groupings derived from a representative
/// sample of tiles.
///
/// Building a profile once from sample tiles avoids re-running the expensive
/// `MinHash` similarity analysis on every subsequent tile; the profile's
/// pre-computed string groups are applied directly during the grouping step
/// instead.
///
/// Profiles from multiple samples are combined with [`PropertyProfile::merge`],
/// which takes the union of both sets of string groups.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyProfile {
    /// Pre-computed string column groupings by column name.
    ///
    /// Each inner vec contains 2 or more column names that should share a
    /// dictionary. An absent entry causes the caller to skip shared-dict
    /// merging for that group.
    string_groups: Vec<Vec<String>>,
}

impl PropertyProfile {
    #[doc(hidden)]
    #[must_use]
    pub fn new(string_groups: Vec<Vec<String>>) -> Self {
        Self { string_groups }
    }

    /// Build a profile from a sample of decoded properties.
    ///
    /// Runs `MinHash` similarity analysis over all string columns and records
    /// which column names should be grouped into shared dictionaries.
    #[must_use]
    pub fn from_sample(properties: &[DecodedProperty<'_>]) -> Self {
        let min_hash = MinHash::<IntoIter<&str>, &str>::new(MINHASH_PERMUTATIONS);
        let profiles = profile_string_columns(properties, &min_hash);

        let string_groups = if profiles.is_empty() {
            Vec::new()
        } else {
            compute_string_groups(&profiles, &min_hash)
                .into_iter()
                .filter(|g| g.len() >= 2)
                .map(|group| {
                    group
                        .iter()
                        .map(|&ci| properties[ci].name().to_owned())
                        .collect()
                })
                .collect()
        };

        Self { string_groups }
    }

    /// Merge two profiles by taking the union of their string groups.
    ///
    /// Groups that share at least one column name are merged together.
    /// Groups already present in `self` are not duplicated.
    #[must_use]
    pub fn merge(mut self, other: &Self) -> Self {
        'outer: for other_group in &other.string_groups {
            for self_group in &mut self.string_groups {
                if other_group.iter().any(|n| self_group.contains(n)) {
                    for name in other_group {
                        if !self_group.contains(name) {
                            self_group.push(name.clone());
                        }
                    }
                    continue 'outer;
                }
            }
            self.string_groups.push(other_group.clone());
        }
        self
    }
}

impl ManualOptimisation for Vec<OwnedProperty> {
    type UsedEncoder = Vec<PropertyEncoder>;

    fn manual_optimisation(&mut self, encoder: Self::UsedEncoder) -> Result<(), MltError> {
        let mut decoded = Vec::with_capacity(self.len());
        for d in &mut *self {
            let d = borrowme::borrow(d).decode()?;
            decoded.push(borrowme::ToOwned::to_owned(&d));
        }
        *self = Vec::<OwnedEncodedProperty>::from_decoded(&decoded, encoder)?
            .into_iter()
            .map(OwnedProperty::Encoded)
            .collect();
        Ok(())
    }
}

impl ProfileOptimisation for Vec<OwnedProperty> {
    type UsedEncoder = Vec<PropertyEncoder>;
    type Profile = PropertyProfile;

    fn profile_driven_optimisation(
        &mut self,
        profile: &Self::Profile,
    ) -> Result<Self::UsedEncoder, MltError> {
        let mut decoded = Vec::with_capacity(self.len());
        for d in &mut *self {
            let d = borrowme::borrow(d).decode()?;
            decoded.push(borrowme::ToOwned::to_owned(&d));
        }
        let enc = apply_profile(&mut decoded, profile);
        *self = Vec::<OwnedEncodedProperty>::from_decoded(&decoded, enc.clone())?
            .into_iter()
            .map(OwnedProperty::Encoded)
            .collect();
        Ok(enc)
    }
}

impl AutomaticOptimisation for Vec<OwnedProperty> {
    type UsedEncoder = Vec<PropertyEncoder>;

    fn automatic_encoding_optimisation(&mut self) -> Result<Self::UsedEncoder, MltError> {
        let mut decoded = Vec::with_capacity(self.len());
        for d in &mut *self {
            let d = borrowme::borrow(d).decode()?;
            decoded.push(borrowme::ToOwned::to_owned(&d));
        }
        let enc = optimize(&mut decoded);
        *self = Vec::<OwnedEncodedProperty>::from_decoded(&decoded, enc.clone())?
            .into_iter()
            .map(OwnedProperty::Encoded)
            .collect();
        Ok(enc)
    }
}

/// Analyze `properties` and return a configured [`Vec<PropertyEncoder>`].
///
/// This function mutates `properties` by combining similar string columns
/// into `DecodedProperty::SharedDict` values.
fn optimize(properties: &mut Vec<DecodedProperty<'_>>) -> Vec<PropertyEncoder> {
    let profile = PropertyProfile::from_sample(properties);
    apply_profile(properties, &profile)
}

/// Apply a profile to `properties`, using the pre-computed string groups
/// instead of re-running the `MinHash` similarity analysis.
///
/// The same encoder selection logic as [`optimize`] is applied after grouping.
fn apply_profile(
    properties: &mut Vec<DecodedProperty<'_>>,
    profile: &PropertyProfile,
) -> Vec<PropertyEncoder> {
    if properties.is_empty() {
        return Vec::new();
    }
    apply_string_groups(properties, &profile.string_groups);
    properties.iter().map(build_encoder).collect()
}

/// Apply pre-computed string groups to `properties` by matching column names.
///
/// Columns present in the profile's groups but absent from this tile are
/// silently skipped. Groups that resolve to fewer than 2 present columns are
/// also skipped.
fn apply_string_groups(properties: &mut Vec<DecodedProperty<'_>>, string_groups: &[Vec<String>]) {
    let matched_groups: Vec<Vec<usize>> = string_groups
        .iter()
        .filter_map(|group| {
            let mut indices: Vec<usize> = group
                .iter()
                .filter_map(|name| {
                    properties.iter().position(
                        |p| matches!(p, DecodedProperty::Str(v) if v.name == name.as_str()),
                    )
                })
                .collect();
            indices.sort_unstable();
            if indices.len() >= 2 {
                Some(indices)
            } else {
                None
            }
        })
        .collect();

    if !matched_groups.is_empty() {
        merge_str_to_shared_dicts(properties, &matched_groups);
    }
}

/// Profile string columns by computing `MinHash` signatures.
fn profile_string_columns(
    properties: &[DecodedProperty<'_>],
    min_hash: &MinHash<IntoIter<&str>, &str>,
) -> Vec<StringProfile> {
    properties
        .iter()
        .enumerate()
        .filter_map(|(col_idx, prop)| {
            if let DecodedProperty::Str(values) = prop {
                let owned_values = values.dense_values();
                let unique_set: HashSet<&str> = owned_values.iter().map(String::as_str).collect();

                // Guard against all-null columns - MinHash panics on an empty iterator.
                let min_hashes = if unique_set.is_empty() {
                    Vec::new()
                } else {
                    min_hash.get_min_hashes(unique_set.into_iter())
                };
                Some(StringProfile {
                    col_idx,
                    min_hashes,
                })
            } else {
                None
            }
        })
        .collect()
}

/// Compute groups of similar string columns using union-find.
///
/// Returns groups as `Vec<Vec<usize>>` where each inner vec contains
/// column indices sorted by position.
fn compute_string_groups(
    profiles: &[StringProfile],
    min_hash: &MinHash<IntoIter<&str>, &str>,
) -> Vec<Vec<usize>> {
    let n = profiles.len();
    let mut uf = QuickUnionUf::<UnionBySize>::new(n);

    // Compare pairs and union similar columns
    for i in 0..n {
        if !profiles[i].min_hashes.is_empty() {
            for j in (i + 1)..n {
                if !profiles[j].min_hashes.is_empty() {
                    let sim = min_hash.get_similarity_from_hashes(
                        &profiles[i].min_hashes,
                        &profiles[j].min_hashes,
                    );
                    if sim > MINHASH_SIMILARITY_THRESHOLD {
                        uf.union(i, j);
                    }
                }
            }
        }
    }

    // Collect groups by root
    let mut groups_map = HashMap::<usize, Vec<usize>>::new();
    for (i, profile) in profiles.iter().enumerate() {
        let root = uf.find(i);
        groups_map.entry(root).or_default().push(profile.col_idx);
    }

    // Convert map to Vec<Vec<usize>>, sort inner lists, sort by first column
    let mut groups: Vec<Vec<usize>> = groups_map.into_values().collect();
    for g in &mut groups {
        g.sort_unstable();
    }
    groups.sort_unstable_by_key(|g| g[0]);

    groups
}

/// Transform multi-member groups into [`DecodedProperty::SharedDict`].
///
/// For each group with 2+ members:
/// - Computes the common prefix name
/// - Builds [`DecodedSharedDictItem`] for each child
/// - Replaces the first property with [`DecodedProperty::SharedDict`]
/// - Removes the other properties from the vector
fn merge_str_to_shared_dicts(properties: &mut Vec<DecodedProperty<'_>>, groups: &[Vec<usize>]) {
    let mut indices_to_remove: HashSet<usize> = HashSet::new();

    for group in groups {
        if group.len() < 2 {
            continue;
        }
        // TODO: technically we should only be dealing with (String + DecodedStrings) pairs here
        let names: Vec<&str> = group.iter().map(|&ci| properties[ci].name()).collect();
        let prefix = common_prefix_name(&names);

        let items = group
            .iter()
            .map(|&col_idx| {
                let prop = &properties[col_idx];
                let suffix = prop
                    .name()
                    .strip_prefix(&prefix)
                    .unwrap_or(prop.name())
                    .to_owned();
                let DecodedProperty::Str(values) = prop else {
                    unreachable!("group should only contain Str columns");
                };
                (suffix, borrowme::ToOwned::to_owned(values))
            })
            .collect::<Vec<_>>();
        let shared_dict = build_decoded_shared_dict(prefix.clone(), items)
            .expect("building decoded shared dictionary from string columns should succeed");

        // Replace first property with SharedDict
        let first_idx = group[0];
        properties[first_idx] = DecodedProperty::SharedDict(shared_dict);

        // Mark other properties for removal
        for &col_idx in &group[1..] {
            indices_to_remove.insert(col_idx);
        }
    }

    // Remove merged properties in reverse order to preserve indices
    let mut indices: Vec<usize> = indices_to_remove.into_iter().collect();
    indices.sort_unstable();
    for idx in indices.into_iter().rev() {
        properties.remove(idx);
    }
}

/// Build an encoder for any property type.
fn build_encoder(prop: &DecodedProperty<'_>) -> PropertyEncoder {
    match prop {
        DecodedProperty::Bool(v) => {
            PropertyEncoder::Scalar(ScalarEncoder::bool(presence_stream(has_nulls(&v.values))))
        }
        DecodedProperty::F32(v) => {
            PropertyEncoder::Scalar(ScalarEncoder::float(presence_stream(has_nulls(&v.values))))
        }
        DecodedProperty::F64(v) => {
            PropertyEncoder::Scalar(ScalarEncoder::float(presence_stream(has_nulls(&v.values))))
        }
        DecodedProperty::I8(v) => {
            let presence = presence_stream(has_nulls(&v.values));
            // FIXME: inaccurate, but encoders don't support i8 widely. Sometimes, plain might be more efficient for this, but is estimated less effective
            let non_null = v
                .values
                .iter()
                .flatten()
                .copied()
                .map(i32::from)
                .collect::<Vec<i32>>();
            let enc = encode_zigzag(&non_null);
            PropertyEncoder::Scalar(ScalarEncoder::int(presence, IntEncoder::auto_u32(&enc)))
        }
        DecodedProperty::U8(v) => {
            let presence = presence_stream(has_nulls(&v.values));
            // FIXME: inaccurate, but encoders don't support u8 widely. Sometimes, plain might be more efficient for this, but is estimated less effective
            let non_null: Vec<u32> = v.values.iter().flatten().copied().map(u32::from).collect();
            PropertyEncoder::Scalar(ScalarEncoder::int(
                presence,
                IntEncoder::auto_u32(&non_null),
            ))
        }
        DecodedProperty::I32(v) => {
            let presence = presence_stream(has_nulls(&v.values));
            let non_null = v.values.iter().flatten().copied().collect::<Vec<i32>>();
            let enc = encode_zigzag(&non_null);
            PropertyEncoder::Scalar(ScalarEncoder::int(presence, IntEncoder::auto_u32(&enc)))
        }
        DecodedProperty::U32(v) => {
            let presence = presence_stream(has_nulls(&v.values));
            let non_null: Vec<u32> = v.values.iter().flatten().copied().collect();
            PropertyEncoder::Scalar(ScalarEncoder::int(
                presence,
                IntEncoder::auto_u32(&non_null),
            ))
        }
        DecodedProperty::I64(v) => {
            let presence = presence_stream(has_nulls(&v.values));
            let non_null = &v.values.iter().flatten().copied().collect::<Vec<i64>>();
            let enc = encode_zigzag(non_null);
            PropertyEncoder::Scalar(ScalarEncoder::int(presence, IntEncoder::auto_u64(&enc)))
        }
        DecodedProperty::U64(v) => {
            let non_null: Vec<u64> = v.values.iter().flatten().copied().collect();
            PropertyEncoder::Scalar(ScalarEncoder::int(
                presence_stream(has_nulls(&v.values)),
                IntEncoder::auto_u64(&non_null),
            ))
        }
        DecodedProperty::Str(v) => {
            let presence = presence_stream(v.has_nulls());
            let owned_values = v.dense_values();
            let non_null: Vec<&str> = owned_values.iter().map(String::as_str).collect();
            scalar_str_encoder(presence, &non_null)
        }
        DecodedProperty::SharedDict(shared_dict) => {
            build_shared_dict_encoder(shared_dict, &shared_dict.items)
        }
    }
}

/// Build a `SharedDictEncoder` for a `DecodedProperty::SharedDict`.
fn build_shared_dict_encoder(
    shared_dict: &DecodedSharedDict<'_>,
    items: &[DecodedSharedDictItem],
) -> PropertyEncoder {
    let dict_spans = collect_shared_dict_spans(items);
    // Collect all strings for FSST viability check
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

    // Build item encoders
    let item_encoders: Vec<SharedDictItemEncoder> = items
        .iter()
        .map(|item| {
            let presence = presence_stream(item.has_nulls());
            let offsets = compute_offset_encoder(items, item);
            SharedDictItemEncoder { presence, offsets }
        })
        .collect();

    SharedDictEncoder {
        dict_encoder,
        items: item_encoders,
    }
    .into()
}

/// Compute the optimal `IntEncoder` for the offset stream of one item
/// in a shared dictionary.
fn compute_offset_encoder(
    items: &[DecodedSharedDictItem],
    target_item: &DecodedSharedDictItem,
) -> IntEncoder {
    let dict_index: HashMap<(u32, u32), u32> = collect_shared_dict_spans(items)
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

fn has_nulls<T>(values: &[Option<T>]) -> bool {
    values.iter().any(Option::is_none)
}

fn presence_stream(has_nulls: bool) -> PresenceStream {
    if has_nulls {
        PresenceStream::Present
    } else {
        PresenceStream::Absent
    }
}

/// Returns the longest common byte prefix of `names`.
fn common_prefix_name(names: &[&str]) -> String {
    debug_assert!(!names.is_empty());
    let first = names[0];
    let mut prefix_len = first.len();
    for name in &names[1..] {
        let new_len = first
            .chars()
            .zip(name.chars())
            .take_while(|(a, b)| a == b)
            .count();
        prefix_len = prefix_len.min(new_len);
        if prefix_len == 0 {
            break;
        }
    }
    let prefix_len = first.floor_char_boundary(prefix_len);
    let raw = &first[..prefix_len];
    if raw.is_empty() {
        String::new()
    } else {
        raw.to_owned()
    }
}

/// Choose between `Plain` and `Fsst` for a standalone string column.
fn scalar_str_encoder(presence: PresenceStream, non_null: &[&str]) -> PropertyEncoder {
    let lengths: Vec<u32> = non_null
        .iter()
        .map(|s| u32::try_from(s.len()).unwrap_or(u32::MAX))
        .collect();
    if fsst_is_viable(non_null) {
        PropertyEncoder::Scalar(ScalarEncoder::str_fsst(
            presence,
            IntEncoder::varint(),
            IntEncoder::auto_u32(&lengths),
        ))
    } else {
        PropertyEncoder::Scalar(ScalarEncoder::str(presence, IntEncoder::auto_u32(&lengths)))
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
