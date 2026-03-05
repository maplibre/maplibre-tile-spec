//! Optimizer that analyses a batch of [`DecodedProperty`] values in a
//! "Profile -> Group -> Compete -> Select" pipeline and produces a
//! [`MultiPropertyEncoder`] with near-optimal per-column encoding settings.

use std::collections::hash_set::IntoIter;
use std::collections::{HashMap, HashSet};

use fsst::Compressor;
use probabilistic_collections::similarity::MinHash;
use union_find::{QuickUnionUf, UnionBySize, UnionFind as _};

use crate::v01::property::strings::StrEncoder;
use crate::v01::property::{
    DecodedProperty, MultiPropertyEncoder, PresenceStream, PropValue, PropertyEncoder,
    ScalarEncoder,
};
use crate::v01::stream::IntEncoder;

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

/// Analyses a batch of [`DecodedProperty`] values and produces a
/// [`MultiPropertyEncoder`] with near-optimal per-column encoding settings.
///
/// # Pipeline
///
/// 1. **Profile**
///    single pass over every string column: collect unique non-null values
///    and compute a `MinHash` signature over them.
/// 2. **Group**
///    compare [`MinHash`] signatures with Jaccard similarity; columns
///    whose similarity exceeds [`MINHASH_SIMILARITY_THRESHOLD`] are clustered
///    into a shared dictionary.  The struct column name is derived from the
///    longest common prefix of the grouped property names.
/// 3. **Compete & Select** - choose the best `IntEncoder` for integer columns
///    via `auto_u32` / `auto_u64` pruning-competition; decide between
///    `Plain` and `Fsst` string encodings using an FSST viability probe;
///    set `PresenceStream::Absent` for columns that have no null values.
pub struct PropertyOptimizer;

impl PropertyOptimizer {
    /// Analyse `properties` and return a configured [`MultiPropertyEncoder`].
    #[must_use]
    pub fn optimize(properties: &[DecodedProperty]) -> MultiPropertyEncoder {
        if properties.is_empty() {
            return MultiPropertyEncoder::new(Vec::new(), HashMap::new());
        }

        // One MinHash instance is shared across all string columns so that
        // `get_similarity_from_hashes` produces consistent Jaccard estimates.
        let min_hash = MinHash::<IntoIter<&str>, &str>::new(MINHASH_PERMUTATIONS);

        let str_profiles = profile_string_columns(properties, &min_hash);
        let (groups, col_to_group) =
            group_string_columns_by_min_hash_similarity(&min_hash, &str_profiles);
        let (shared_dicts, group_struct_names) =
            name_and_configure_multi_member_groups(properties, &groups);
        let encoders =
            build_per_property_encoders(properties, &groups, &col_to_group, &group_struct_names);

        MultiPropertyEncoder::new(encoders, shared_dicts)
    }
}

fn build_per_property_encoders(
    properties: &[DecodedProperty],
    groups: &[Vec<usize>],
    col_to_group: &HashMap<usize, usize>,
    group_struct_names: &[Option<String>],
) -> Vec<PropertyEncoder> {
    let encoders: Vec<PropertyEncoder> = properties
        .iter()
        .enumerate()
        .map(|(idx, prop)| {
            build_encoder(
                idx,
                prop,
                col_to_group,
                groups,
                group_struct_names,
                properties,
            )
        })
        .collect();
    encoders
}

fn name_and_configure_multi_member_groups(
    properties: &[DecodedProperty],
    groups: &[Vec<usize>],
) -> (HashMap<String, StrEncoder>, Vec<Option<String>>) {
    // `struct_name` is the actual column name written to the tile.  It is
    // always derived from the longest common prefix of the grouped property
    // names
    //
    // If two independent groups share the same prefix (rare in practice),
    // the second group is silently dissolved: its members are encoded as
    // individual scalar columns.
    let mut used_names: HashSet<String> = HashSet::new();
    let mut shared_dicts: HashMap<String, StrEncoder> = HashMap::new();
    // `group_struct_names[g] = Some(name)` iff the group has ≥ 2 members
    // AND its prefix did not collide with a previously registered group.
    let mut group_struct_names: Vec<Option<String>> = vec![None; groups.len()];

    for (g_idx, group) in groups.iter().enumerate() {
        if group.len() < 2 {
            continue;
        }
        let names: Vec<&str> = group
            .iter()
            .map(|&ci| properties[ci].name.as_str())
            .collect();
        let struct_name = common_prefix_name(&names);

        // If the prefix is already taken, fall back to scalar for this group.
        if used_names.contains(&struct_name) {
            continue;
        }
        used_names.insert(struct_name.clone());

        let str_enc = shared_dict_str_encoder(group, properties);
        shared_dicts.insert(struct_name.clone(), str_enc);
        group_struct_names[g_idx] = Some(struct_name);
    }
    (shared_dicts, group_struct_names)
}

/// group string columns by [`MinHash`] similarity
fn group_string_columns_by_min_hash_similarity(
    min_hash: &MinHash<IntoIter<&str>, &str>,
    str_profiles: &[StringProfile],
) -> (Vec<Vec<usize>>, HashMap<usize, usize>) {
    // Each inner Vec holds `col_idx` values sorted in property order.
    let groups: Vec<Vec<usize>> = group_strings(str_profiles, min_hash);

    // col_idx -> index into `groups`
    let mut col_to_group: HashMap<usize, usize> = HashMap::new();
    for (g_idx, group) in groups.iter().enumerate() {
        for &col_idx in group {
            col_to_group.insert(col_idx, g_idx);
        }
    }
    (groups, col_to_group)
}

/// Profile every string column
fn profile_string_columns(
    properties: &[DecodedProperty],
    min_hash: &MinHash<IntoIter<&str>, &str>,
) -> Vec<StringProfile> {
    let str_profiles: Vec<StringProfile> = properties
        .iter()
        .enumerate()
        .filter_map(|(idx, prop)| {
            if let PropValue::Str(values) = &prop.values {
                Some(profile_string_column(idx, values, min_hash))
            } else {
                None
            }
        })
        .collect();
    str_profiles
}

/// Profile a single string column by computing a [`MinHash`] signature over
/// its unique non-null values.
fn profile_string_column(
    col_idx: usize,
    values: &[Option<String>],
    min_hash: &MinHash<IntoIter<&str>, &str>,
) -> StringProfile {
    let mut unique_set: HashSet<&str> = HashSet::new();
    for s in values.iter().flatten() {
        unique_set.insert(s.as_str());
    }

    // Guard against all-null columns - MinHash panics on an empty iterator.
    let min_hashes = if unique_set.is_empty() {
        Vec::new()
    } else {
        min_hash.get_min_hashes(unique_set.into_iter())
    };

    StringProfile {
        col_idx,
        min_hashes,
    }
}

/// Cluster string profiles into groups using greedy [`MinHash`] Jaccard merging.
///
/// Returns a `Vec` of groups; each group is a sorted list of `col_idx` values.
/// Every string column appears in exactly one group. Singleton groups
/// (length 1) represent standalone scalar string columns.
fn group_strings(
    profiles: &[StringProfile],
    min_hash: &MinHash<IntoIter<&str>, &str>,
) -> Vec<Vec<usize>> {
    let n = profiles.len();
    let mut uf = QuickUnionUf::<UnionBySize>::new(n);

    // Compare every pair of columns and union if similarity exceeds threshold
    for i in 0..n {
        if profiles[i].min_hashes.is_empty() {
            continue; // skip empty columns
        }
        for j in (i + 1)..n {
            if profiles[j].min_hashes.is_empty() {
                continue;
            }

            let sim = min_hash
                .get_similarity_from_hashes(&profiles[i].min_hashes, &profiles[j].min_hashes);

            if sim > MINHASH_SIMILARITY_THRESHOLD {
                uf.union(i, j);
            }
        }
    }

    // Collect groups by root
    let mut groups_map = HashMap::<_, Vec<usize>>::new();
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

/// Choose the shared-dictionary `StrEncoder` for a multi-member group.
///
/// Runs the FSST viability probe over the combined corpus of all children.
fn shared_dict_str_encoder(group: &[usize], properties: &[DecodedProperty]) -> StrEncoder {
    let mut all_strings: Vec<&str> = Vec::new();
    for &col_idx in group {
        if let PropValue::Str(values) = &properties[col_idx].values {
            for s in values.iter().flatten() {
                all_strings.push(s.as_str());
            }
        }
    }
    if fsst_is_viable(&all_strings) {
        // VarInt works well for both the symbol-table length stream
        // (≤ 255 small values) and the original string-length stream.
        StrEncoder::fsst(IntEncoder::varint(), IntEncoder::varint())
    } else {
        let lengths: Vec<u32> = all_strings
            .iter()
            .map(|s| u32::try_from(s.len()).unwrap_or(u32::MAX))
            .collect();
        StrEncoder::Plain {
            string_lengths: IntEncoder::auto_u32(&lengths),
        }
    }
}

fn build_encoder(
    idx: usize,
    prop: &DecodedProperty,
    col_to_group: &HashMap<usize, usize>,
    groups: &[Vec<usize>],
    group_struct_names: &[Option<String>],
    properties: &[DecodedProperty],
) -> PropertyEncoder {
    match &prop.values {
        PropValue::Bool(v) => {
            PropertyEncoder::Scalar(ScalarEncoder::bool(to_presence(count_nulls(v))))
        }
        PropValue::F32(v) => {
            PropertyEncoder::Scalar(ScalarEncoder::float(to_presence(count_nulls(v))))
        }
        PropValue::F64(v) => {
            PropertyEncoder::Scalar(ScalarEncoder::float(to_presence(count_nulls(v))))
        }
        PropValue::I8(v) => {
            let presence = to_presence(count_nulls(v));
            // FIXME: inaccurate, but encoders don't support i8 widely. Sometimes, plain might be more efficient for this, but is estimated less effective
            let non_null: Vec<u32> = v
                .iter()
                .flatten()
                .copied()
                .map(i32::from)
                .map(i32::cast_unsigned)
                .collect();
            PropertyEncoder::Scalar(ScalarEncoder::int(
                presence,
                IntEncoder::auto_u32(&non_null),
            ))
        }
        PropValue::U8(v) => {
            let presence = to_presence(count_nulls(v));
            // FIXME: inaccurate, but encoders don't support u8 widely. Sometimes, plain might be more efficient for this, but is estimated less effective
            let non_null: Vec<u32> = v.iter().flatten().copied().map(u32::from).collect();
            PropertyEncoder::Scalar(ScalarEncoder::int(
                presence,
                IntEncoder::auto_u32(&non_null),
            ))
        }
        PropValue::I32(v) => {
            let presence = to_presence(count_nulls(v));
            let non_null: Vec<u32> = v.iter().flatten().map(|&x| x.cast_unsigned()).collect();
            PropertyEncoder::Scalar(ScalarEncoder::int(
                presence,
                IntEncoder::auto_u32(&non_null),
            ))
        }
        PropValue::U32(v) => {
            let presence = to_presence(count_nulls(v));
            let non_null: Vec<u32> = v.iter().flatten().copied().collect();
            PropertyEncoder::Scalar(ScalarEncoder::int(
                presence,
                IntEncoder::auto_u32(&non_null),
            ))
        }
        PropValue::I64(v) => {
            let presence = to_presence(count_nulls(v));
            let non_null: Vec<u64> = v.iter().flatten().map(|&x| x.cast_unsigned()).collect();
            PropertyEncoder::Scalar(ScalarEncoder::int(
                presence,
                IntEncoder::auto_u64(&non_null),
            ))
        }
        PropValue::U64(v) => {
            let presence = to_presence(count_nulls(v));
            let non_null: Vec<u64> = v.iter().flatten().copied().collect();
            PropertyEncoder::Scalar(ScalarEncoder::int(
                presence,
                IntEncoder::auto_u64(&non_null),
            ))
        }
        PropValue::Str(v) => {
            let num_nulls = count_nulls(v);
            let presence = to_presence(num_nulls);
            let g_idx = col_to_group[&idx];
            let group = &groups[g_idx];

            if let Some(struct_name) = &group_struct_names[g_idx] {
                // Replicate the dictionary build order (property order within
                // the group) to compute the actual offset array, then select
                // the best IntEncoder for it.
                let offset = offset_encoder_for(group, properties, idx);
                // The decoder reconstructs the original property name as
                // `struct_name + child_name`, so child_name must be the
                // suffix that follows the common prefix, not the full name.
                let child_suffix = prop
                    .name
                    .strip_prefix(struct_name.as_str())
                    .unwrap_or(&prop.name)
                    .to_owned();
                PropertyEncoder::shared_dict(struct_name.clone(), child_suffix, presence, offset)
            } else {
                // Standalone scalar string
                let non_null: Vec<&str> = v.iter().flatten().map(String::as_str).collect();
                scalar_str_encoder(presence, &non_null)
            }
        }
        PropValue::SharedDict => {
            // `SharedDict` columns are produced by the encoder, not consumed
            // as decoder input.  Hitting this branch means the caller passed
            // invalid data, which is a programming error.
            debug_assert!(
                false,
                "PropValue::SharedDict must not appear in decoded input"
            );
            PropertyEncoder::Scalar(ScalarEncoder::bool(PresenceStream::Absent))
        }
    }
}

/// Choose between `Plain` and `Fsst` for a standalone string column.
fn scalar_str_encoder(presence: PresenceStream, non_null: &[&str]) -> PropertyEncoder {
    let lengths: Vec<u32> = non_null
        .iter()
        .map(|s| u32::try_from(s.len()).unwrap_or(u32::MAX))
        .collect();
    if fsst_is_viable(non_null) {
        // symbol_lengths stream is small (≤ 255 entries); VarInt is sufficient.
        PropertyEncoder::Scalar(ScalarEncoder::str_fsst(
            presence,
            IntEncoder::varint(),
            IntEncoder::auto_u32(&lengths),
        ))
    } else {
        PropertyEncoder::Scalar(ScalarEncoder::str(presence, IntEncoder::auto_u32(&lengths)))
    }
}

/// Compute the optimal `IntEncoder` for the offset stream of one child
/// inside a shared-dictionary group.
///
/// The dictionary insertion order mirrors the encoding pass (properties
/// iterated in their original index order within the group), so the offsets
/// computed here match those written during actual encoding.
fn offset_encoder_for(
    group: &[usize],
    properties: &[DecodedProperty],
    child_col_idx: usize,
) -> IntEncoder {
    // Build the shared dictionary in property order (group is sorted by col_idx).
    let mut dict_index: HashMap<&str, u32> = HashMap::new();
    let mut dict_size = 0u32;
    for &col_idx in group {
        if let PropValue::Str(values) = &properties[col_idx].values {
            for s in values.iter().flatten() {
                if !dict_index.contains_key(s.as_str()) {
                    dict_index.insert(s.as_str(), dict_size);
                    dict_size = dict_size.saturating_add(1);
                }
            }
        }
    }

    // Map this child's non-null values to their dictionary offsets.
    // Every non-null string must already be present in `dict_index` because
    // the dict was built from the union of all group columns, including this
    // child.  An absent key would indicate a bug in the grouping logic.
    let offsets: Vec<u32> = if let PropValue::Str(values) = &properties[child_col_idx].values {
        values
            .iter()
            .flatten()
            .map(|s| {
                *dict_index
                    .get(s.as_str())
                    .expect("non-null string missing from shared dictionary")
            })
            .collect()
    } else {
        Vec::new()
    };

    if offsets.is_empty() {
        IntEncoder::plain()
    } else {
        IntEncoder::auto_u32(&offsets)
    }
}

fn count_nulls<T>(values: &[Option<T>]) -> usize {
    values.iter().filter(|v| v.is_none()).count()
}

fn to_presence(num_nulls: usize) -> PresenceStream {
    if num_nulls == 0 {
        PresenceStream::Absent
    } else {
        PresenceStream::Present
    }
}

/// Returns the longest common byte prefix of `names`.
///
/// Falls back to the first name when the stripped prefix is empty.
fn common_prefix_name(names: &[&str]) -> String {
    debug_assert!(!names.is_empty());
    let first = names[0];
    let mut prefix_len = first.len();
    for name in &names[1..] {
        let new_len = first
            .bytes()
            .zip(name.bytes())
            .take_while(|(a, b)| a == b)
            .count();
        // Accumulate the minimum - do not overwrite with a potentially longer
        // match against `first` when a shorter one was already found.
        prefix_len = prefix_len.min(new_len);
        if prefix_len == 0 {
            break;
        }
    }
    let prefix_len = first.floor_char_boundary(prefix_len);
    let raw = &first[..prefix_len];
    if raw.is_empty() {
        first.to_owned()
    } else {
        raw.to_owned()
    }
}

/// Returns `true` when FSST compression is likely to save space on `strings`.
///
/// Trains a compressor on a sample and checks whether
/// `symbol_overhead + compressed_size < plain_size`.
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

    fn make_prop(name: &str, values: PropValue) -> DecodedProperty {
        DecodedProperty {
            name: name.to_owned(),
            values,
        }
    }

    #[test]
    fn no_nulls_produces_absent_presence() {
        let props = vec![make_prop(
            "pop",
            PropValue::U32(vec![Some(1), Some(2), Some(3)]),
        )];
        let enc = PropertyOptimizer::optimize(&props);
        let PropertyEncoder::Scalar(scalar) = &enc.properties[0] else {
            panic!("expected Scalar");
        };
        assert_eq!(scalar.optional, PresenceStream::Absent);
    }

    #[test]
    fn all_nulls_produces_present_presence() {
        let props = vec![make_prop("x", PropValue::I32(vec![None, None, None]))];
        let enc = PropertyOptimizer::optimize(&props);
        let PropertyEncoder::Scalar(scalar) = &enc.properties[0] else {
            panic!("expected Scalar");
        };
        assert_eq!(scalar.optional, PresenceStream::Present);
    }

    #[test]
    fn sequential_u32_picks_delta() {
        use crate::v01::{LogicalEncoder, PhysicalEncoder};
        let data: Vec<Option<u32>> = (0u32..1_000).map(Some).collect();
        let props = vec![make_prop("id", PropValue::U32(data))];
        let enc = PropertyOptimizer::optimize(&props);
        let PropertyEncoder::Scalar(scalar) = &enc.properties[0] else {
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
        let props = vec![make_prop("val", PropValue::U32(data))];
        let enc = PropertyOptimizer::optimize(&props);
        let PropertyEncoder::Scalar(scalar) = &enc.properties[0] else {
            panic!("expected Scalar");
        };
        let crate::v01::property::ScalarValueEncoder::Int(int_enc) = scalar.value else {
            panic!("expected Int encoder");
        };
        assert_eq!(int_enc.logical, LogicalEncoder::Rle);
    }

    #[test]
    fn struct_name_is_common_prefix() {
        let vocab: Vec<Option<String>> = (0..20).map(|i| Some(format!("val{i}"))).collect();
        let props = vec![
            make_prop("addr:street", PropValue::Str(vocab.clone())),
            make_prop("addr:city", PropValue::Str(vocab)),
        ];
        let enc = PropertyOptimizer::optimize(&props);
        let PropertyEncoder::SharedDict(sd) = &enc.properties[0] else {
            panic!("expected SharedDict");
        };
        assert_eq!(sd.struct_name, "addr:");
    }
}
