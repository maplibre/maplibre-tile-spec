//! Optimizer that analyses a batch of [`DecodedProperty`] values and produces
//! [`Vec<PropertyEncoder>`] with near-optimal per-column encoding settings.
//!
//! # Pipeline
//!
//! 1. **Profile & Group** - compute `MinHash` signatures for string columns and
//!    cluster similar columns into shared dictionaries using union-find.
//! 2. **Transform** - merge grouped string columns into `PropValue::SharedDict`.
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
use crate::v01::property::strings::{
    SharedDictEncoder, SharedDictItemEncoder, StrEncoder, build_decoded_shared_dict,
};
use crate::v01::property::{
    DecodedProperty, DecodedSharedDict, DecodedSharedDictItem, PresenceStream, PropertyEncoder,
    ScalarEncoder,
};
use crate::v01::stream::IntEncoder;
use crate::v01::{OwnedEncodedProperty, OwnedProperty};
use crate::{FromDecoded as _, MltError};

/// Number of [`MinHash`] permutations. 128 gives ~7 % error on Jaccard estimates.
const MINHASH_PERMUTATIONS: usize = 128;

/// String columns whose estimated Jaccard similarity exceeds this threshold are
/// grouped into a single shared dictionary.
pub const MINHASH_SIMILARITY_THRESHOLD: f64 = 0.6;

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

impl ManualOptimisation for Vec<OwnedProperty> {
    type UsedEncoder = Vec<PropertyEncoder>;

    fn manual_optimisation(&mut self, encoder: Self::UsedEncoder) -> Result<(), MltError> {
        let mut decoded = Vec::with_capacity(self.len());
        for d in &mut *self {
            let d = borrowme::borrow(d).decode()?;
            decoded.push(d);
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
    type Profile = ();

    fn profile_driven_optimisation(
        &mut self,
        _profile: &Self::Profile,
    ) -> Result<Self::UsedEncoder, MltError> {
        Err(MltError::NotImplemented(
            "ProfileOptimisation::profile_driven_optimisation",
        ))
    }
}

impl AutomaticOptimisation for Vec<OwnedProperty> {
    type UsedEncoder = Vec<PropertyEncoder>;
    fn automatic_encoding_optimisation(&mut self) -> Result<Self::UsedEncoder, MltError> {
        let mut decoded = Vec::with_capacity(self.len());
        for d in &mut *self {
            let d = borrowme::borrow(d).decode()?;
            decoded.push(d);
        }

        let enc = PropertyOptimizer::optimize(&mut decoded);
        *self = Vec::<OwnedEncodedProperty>::from_decoded(&decoded, enc.clone())?
            .into_iter()
            .map(OwnedProperty::Encoded)
            .collect();
        Ok(enc)
    }
}

/// Analyzes a batch of [`DecodedProperty`] values and produces
/// [`Vec<PropertyEncoder>`] with near-optimal per-column encoding settings.
pub struct PropertyOptimizer;

impl PropertyOptimizer {
    /// Analyze `properties` and return a configured [`Vec<PropertyEncoder>`].
    ///
    /// This method mutates `properties` by combining similar string columns
    /// into `PropValue::SharedDict` values.
    #[must_use]
    pub fn optimize(properties: &mut Vec<DecodedProperty<'_>>) -> Vec<PropertyEncoder> {
        if properties.is_empty() {
            return Vec::new();
        }

        // Group similar string columns into SharedDicts, mutating properties in place
        group_string_columns(properties);

        // Build encoders for all properties
        properties.iter().map(build_encoder).collect()
    }
}

/// Group similar string columns into `PropValue::SharedDict`.
///
/// This function:
/// 1. Profiles string columns by computing `MinHash` signatures
/// 2. Groups similar columns using union-find
/// 3. Transforms grouped columns into `PropValue::SharedDict`
/// 4. Removes the merged columns from the properties vector
fn group_string_columns(properties: &mut Vec<DecodedProperty<'_>>) {
    // Profile string columns: compute MinHash signatures
    let min_hash = MinHash::<IntoIter<&str>, &str>::new(MINHASH_PERMUTATIONS);
    let profiles = profile_string_columns(properties, &min_hash);
    if !profiles.is_empty() {
        // Group by MinHash similarity using union-find
        let groups = compute_string_groups(&profiles, &min_hash);

        // Transform multi-member groups into SharedDicts
        merge_str_to_shared_dicts(properties, &groups);
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
            if let DecodedProperty::Str(_, values) = prop {
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

/// Transform multi-member groups into `PropValue::SharedDict`.
///
/// For each group with 2+ members:
/// - Computes the common prefix name
/// - Builds `SharedDictItem` for each child
/// - Replaces the first property with `PropValue::SharedDict`
/// - Removes the other properties from the vector
fn merge_str_to_shared_dicts(properties: &mut Vec<DecodedProperty<'_>>, groups: &[Vec<usize>]) {
    let mut indices_to_remove: HashSet<usize> = HashSet::new();

    for group in groups {
        if group.len() < 2 {
            continue;
        }

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
                let DecodedProperty::Str(_, values) = prop else {
                    unreachable!("group should only contain Str columns");
                };
                (suffix, borrowme::ToOwned::to_owned(values))
            })
            .collect::<Vec<_>>();
        let (shared_dict, items) = build_decoded_shared_dict(items)
            .expect("building decoded shared dictionary from string columns should succeed");

        // Replace first property with SharedDict
        let first_idx = group[0];
        properties[first_idx] = DecodedProperty::SharedDict(prefix, shared_dict, items);

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
        DecodedProperty::Bool(_, v) => {
            PropertyEncoder::Scalar(ScalarEncoder::bool(presence_stream(has_nulls(v))))
        }
        DecodedProperty::F32(_, v) => {
            PropertyEncoder::Scalar(ScalarEncoder::float(presence_stream(has_nulls(v))))
        }
        DecodedProperty::F64(_, v) => {
            PropertyEncoder::Scalar(ScalarEncoder::float(presence_stream(has_nulls(v))))
        }
        DecodedProperty::I8(_, v) => {
            let presence = presence_stream(has_nulls(v));
            // FIXME: inaccurate, but encoders don't support i8 widely. Sometimes, plain might be more efficient for this, but is estimated less effective
            let non_null = v
                .iter()
                .flatten()
                .copied()
                .map(i32::from)
                .collect::<Vec<i32>>();
            let enc = encode_zigzag(&non_null);
            PropertyEncoder::Scalar(ScalarEncoder::int(presence, IntEncoder::auto_u32(&enc)))
        }
        DecodedProperty::U8(_, v) => {
            let presence = presence_stream(has_nulls(v));
            // FIXME: inaccurate, but encoders don't support u8 widely. Sometimes, plain might be more efficient for this, but is estimated less effective
            let non_null: Vec<u32> = v.iter().flatten().copied().map(u32::from).collect();
            PropertyEncoder::Scalar(ScalarEncoder::int(
                presence,
                IntEncoder::auto_u32(&non_null),
            ))
        }
        DecodedProperty::I32(_, v) => {
            let presence = presence_stream(has_nulls(v));
            let non_null = v.iter().flatten().copied().collect::<Vec<i32>>();
            let enc = encode_zigzag(&non_null);
            PropertyEncoder::Scalar(ScalarEncoder::int(presence, IntEncoder::auto_u32(&enc)))
        }
        DecodedProperty::U32(_, v) => {
            let presence = presence_stream(has_nulls(v));
            let non_null: Vec<u32> = v.iter().flatten().copied().collect();
            PropertyEncoder::Scalar(ScalarEncoder::int(
                presence,
                IntEncoder::auto_u32(&non_null),
            ))
        }
        DecodedProperty::I64(_, v) => {
            let presence = presence_stream(has_nulls(v));
            let non_null = v.iter().flatten().copied().collect::<Vec<i64>>();
            let enc = encode_zigzag(&non_null);
            PropertyEncoder::Scalar(ScalarEncoder::int(presence, IntEncoder::auto_u64(&enc)))
        }
        DecodedProperty::U64(_, v) => {
            let non_null: Vec<u64> = v.iter().flatten().copied().collect();
            PropertyEncoder::Scalar(ScalarEncoder::int(
                presence_stream(has_nulls(v)),
                IntEncoder::auto_u64(&non_null),
            ))
        }
        DecodedProperty::Str(_, v) => {
            let presence = presence_stream(v.has_nulls());
            let owned_values = v.dense_values();
            let non_null: Vec<&str> = owned_values.iter().map(String::as_str).collect();
            scalar_str_encoder(presence, &non_null)
        }
        DecodedProperty::SharedDict(_, shared_dict, items) => {
            build_shared_dict_encoder(shared_dict, items)
        }
    }
}

/// Build a `SharedDictEncoder` for a `PropValue::SharedDict`.
fn build_shared_dict_encoder(
    shared_dict: &DecodedSharedDict<'_>,
    items: &[DecodedSharedDictItem],
) -> PropertyEncoder {
    // Collect all strings for FSST viability check
    let all_strings: Vec<&str> = shared_dict
        .spans()
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
            let presence = if item.presence.0.is_some() {
                PresenceStream::Present
            } else {
                PresenceStream::Absent
            };
            let offsets = compute_offset_encoder(shared_dict, item);
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
    shared_dict: &DecodedSharedDict<'_>,
    target_item: &DecodedSharedDictItem,
) -> IntEncoder {
    let dict_index: HashMap<(u32, u32), u32> =
        shared_dict.spans().into_iter().zip(0_u32..).collect();
    let offsets: Vec<u32> = target_item
        .spans
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v01::{DecodedStrings, PropValue};

    fn str_value(values: Vec<Option<String>>) -> PropValue {
        PropValue::Str(DecodedStrings::from(values))
    }

    fn make_prop(name: &str, values: PropValue) -> DecodedProperty<'static> {
        DecodedProperty::from_parts(name.to_owned(), values)
    }

    #[test]
    fn no_nulls_produces_absent_presence() {
        let mut props = vec![make_prop(
            "pop",
            PropValue::U32(vec![Some(1), Some(2), Some(3)]),
        )];
        let enc = PropertyOptimizer::optimize(&mut props);
        let PropertyEncoder::Scalar(scalar) = &enc[0] else {
            panic!("expected Scalar");
        };
        assert_eq!(scalar.presence, PresenceStream::Absent);
    }

    #[test]
    fn all_nulls_produces_present_presence() {
        let mut props = vec![make_prop("x", PropValue::I32(vec![None, None, None]))];
        let enc = PropertyOptimizer::optimize(&mut props);
        let PropertyEncoder::Scalar(scalar) = &enc[0] else {
            panic!("expected Scalar");
        };
        assert_eq!(scalar.presence, PresenceStream::Present);
    }

    #[test]
    fn sequential_u32_picks_delta() {
        use crate::v01::{LogicalEncoder, PhysicalEncoder};
        let data: Vec<Option<u32>> = (0u32..1_000).map(Some).collect();
        let mut props = vec![make_prop("id", PropValue::U32(data))];
        let enc = PropertyOptimizer::optimize(&mut props);
        let PropertyEncoder::Scalar(scalar) = &enc[0] else {
            panic!("expected Scalar");
        };
        let crate::v01::property::ScalarValueEncoder::Int(int_enc) = scalar.value else {
            panic!("expected Int encoder");
        };
        assert_eq!(int_enc.logical, LogicalEncoder::Delta);
        assert_eq!(int_enc.physical, PhysicalEncoder::FastPFOR);
    }

    #[test]
    fn constant_u32_picks_rle() {
        use crate::v01::LogicalEncoder;
        let data: Vec<Option<u32>> = vec![Some(42); 500];
        let mut props = vec![make_prop("val", PropValue::U32(data))];
        let enc = PropertyOptimizer::optimize(&mut props);
        let PropertyEncoder::Scalar(scalar) = &enc[0] else {
            panic!("expected Scalar");
        };
        let crate::v01::property::ScalarValueEncoder::Int(int_enc) = scalar.value else {
            panic!("expected Int encoder");
        };
        assert_eq!(int_enc.logical, LogicalEncoder::Rle);
    }

    #[test]
    fn similar_strings_grouped_into_shared_dict() {
        let vocab: Vec<Option<String>> = (0..20).map(|i| Some(format!("val{i}"))).collect();
        let mut props = vec![
            make_prop("addr:street", str_value(vocab.clone())),
            make_prop("addr:city", str_value(vocab)),
        ];
        let enc = PropertyOptimizer::optimize(&mut props);

        // Should produce one SharedDict property
        assert_eq!(props.len(), 1);
        assert_eq!(props[0].name(), "addr:");
        let DecodedProperty::SharedDict(_, _, items) = &props[0] else {
            panic!("expected SharedDict");
        };
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].suffix, "street");
        assert_eq!(items[1].suffix, "city");

        // Encoder should be SharedDict
        assert_eq!(enc.len(), 1);
        assert!(matches!(&enc[0], PropertyEncoder::SharedDict(_)));
    }

    #[test]
    fn dissimilar_strings_stay_scalar() {
        let mut props = vec![
            make_prop(
                "city:de",
                str_value(vec![
                    Some("Munich".to_string()),
                    Some("Manheim".to_string()),
                    Some("Garching".to_string()),
                ]),
            ),
            make_prop(
                "city:Colorado",
                str_value(vec![
                    Some("Black".to_string()),
                    Some("Red".to_string()),
                    Some("Gold".to_string()),
                ]),
            ),
        ];
        let enc = PropertyOptimizer::optimize(&mut props);

        // Should stay as two separate properties (dissimilar values)
        assert_eq!(props.len(), 2);
        assert!(matches!(&props[0], DecodedProperty::Str(..)));
        assert!(matches!(&props[1], DecodedProperty::Str(..)));
        assert!(matches!(&enc[0], PropertyEncoder::Scalar(_)));
        assert!(matches!(&enc[1], PropertyEncoder::Scalar(_)));
    }
}
