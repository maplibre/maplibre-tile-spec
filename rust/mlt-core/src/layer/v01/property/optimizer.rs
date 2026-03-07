//! Optimizer that analyses a batch of [`DecodedProperty`] values in a
//! "Profile -> Group -> Compete -> Select" pipeline and produces a
//! [`MultiPropertyEncoder`] with near-optimal per-column encoding settings.

use std::collections::hash_set::IntoIter;
use std::collections::{HashMap, HashSet};

use fsst::Compressor;
use probabilistic_collections::similarity::MinHash;
use union_find::{QuickUnionUf, UnionBySize, UnionFind as _};

use crate::utils::encode_zigzag;
use crate::v01::property::strings::{SharedDictEncoder, SharedDictItemEncoder, StrEncoder};
use crate::v01::property::{
    DecodedProperty, MultiPropertyEncoder, PresenceStream, PropValue, PropertyEncoder,
    ScalarEncoder, SharedDictItem,
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

/// Analyzes a batch of [`DecodedProperty`] values and produces a
/// [`MultiPropertyEncoder`] with near-optimal per-column encoding settings.
///
/// # Pipeline
///
/// 1. **Profile**
///    single pass over every string column: collect unique non-null values
///    and compute a `MinHash` signature over them.
/// 2. **Group**
///    compare [`MinHash`] signatures with Jaccard similarity; columns
///    whose similarity exceeds `MINHASH_SIMILARITY_THRESHOLD` are clustered
///    into a shared dictionary.  The struct column name is derived from the
///    longest common prefix of the grouped property names.
/// 3. **Transform**
///    grouped string columns are combined into a single `PropValue::SharedDict`
///    property, removing the original individual properties from the input.
/// 4. **Compete & Select** - choose the best `IntEncoder` for integer columns
///    via `auto_u32` / `auto_u64` pruning-competition; decide between
///    `Plain` and `Fsst` string encodings using an FSST viability probe;
///    set `PresenceStream::Absent` for columns that have no null values.
pub struct PropertyOptimizer;

impl PropertyOptimizer {
    /// Analyze `properties` and return a configured [`MultiPropertyEncoder`].
    ///
    /// This method mutates `properties` by combining similar string columns
    /// into `PropValue::SharedDict` values.
    #[must_use]
    pub fn optimize(properties: &mut Vec<DecodedProperty>) -> MultiPropertyEncoder {
        if properties.is_empty() {
            return MultiPropertyEncoder::new(Vec::new());
        }

        // One MinHash instance is shared across all string columns so that
        // `get_similarity_from_hashes` produces consistent Jaccard estimates.
        let min_hash = MinHash::<IntoIter<&str>, &str>::new(MINHASH_PERMUTATIONS);

        // FIXME: we may want to simplify Group/GroupIndex - instead of having groupInfo
        //    index into groups, we should just create SharedDicts in the profile_string_columns.
        let str_profiles = profile_string_columns(properties, &min_hash);
        let groups = group_strings(&str_profiles, &min_hash);
        let group_info = name_and_configure_multi_member_groups(properties, &groups);

        // Transform properties: combine grouped string columns into SharedDict
        transform_grouped_strings(properties, &groups, &group_info);

        // Build encoders for the transformed properties
        let encoders = build_encoders(properties, &group_info);

        MultiPropertyEncoder::new(encoders)
    }
}

/// Information about a multi-member group that will become a `SharedDict`
struct GroupInfo {
    /// Index into the groups array
    group_idx: usize,
    /// The struct name (common prefix of property names)
    struct_name: String,
    /// The string encoder for the shared dictionary
    str_encoder: StrEncoder,
    /// Item encoders for each child in the group (in group order)
    item_encoders: Vec<SharedDictItemEncoder>,
}

/// Name and configure multi-member groups, returning info needed for transformation
fn name_and_configure_multi_member_groups(
    properties: &[DecodedProperty],
    groups: &[Vec<usize>],
) -> Vec<GroupInfo> {
    let mut group_infos: Vec<GroupInfo> = Vec::new();

    for (g_idx, group) in groups.iter().enumerate() {
        if group.len() < 2 {
            continue;
        }
        let names: Vec<&str> = group
            .iter()
            .map(|&ci| properties[ci].name.as_str())
            .collect();
        let struct_name = common_prefix_name(&names);
        let str_encoder = shared_dict_str_encoder(group, properties);

        // Build item encoders for each child
        let item_encoders: Vec<SharedDictItemEncoder> = group
            .iter()
            .map(|&col_idx| {
                let prop = &properties[col_idx];
                let PropValue::Str(values) = &prop.values else {
                    unreachable!("group should only contain Str columns");
                };
                let optional = to_presence(count_nulls(values));
                let offset = offset_encoder_for(group, properties, col_idx);
                SharedDictItemEncoder { optional, offset }
            })
            .collect();

        group_infos.push(GroupInfo {
            group_idx: g_idx,
            struct_name,
            str_encoder,
            item_encoders,
        });
    }
    group_infos
}

/// Transform properties by combining grouped string columns into `SharedDict`
fn transform_grouped_strings(
    properties: &mut Vec<DecodedProperty>,
    groups: &[Vec<usize>],
    group_infos: &[GroupInfo],
) {
    // Collect indices to remove (all but first in each multi-member group)
    let mut indices_to_remove: HashSet<usize> = HashSet::new();

    for info in group_infos {
        let group = &groups[info.group_idx];

        // Build SharedDictItem for each child
        let items: Vec<SharedDictItem> = group
            .iter()
            .map(|&col_idx| {
                let prop = &properties[col_idx];
                let suffix = prop
                    .name
                    .strip_prefix(&info.struct_name)
                    .unwrap_or(&prop.name)
                    .to_owned();
                let PropValue::Str(values) = &prop.values else {
                    unreachable!("group should only contain Str columns");
                };
                SharedDictItem {
                    suffix,
                    values: values.clone(),
                }
            })
            .collect();

        // Replace first property with SharedDict, mark others for removal
        let first_idx = group[0];
        properties[first_idx] = DecodedProperty {
            name: info.struct_name.clone(),
            values: PropValue::SharedDict(items),
        };

        for &col_idx in &group[1..] {
            indices_to_remove.insert(col_idx);
        }
    }

    // Remove properties in reverse order to preserve indices
    let mut indices: Vec<usize> = indices_to_remove.into_iter().collect();
    indices.sort_unstable();
    for idx in indices.into_iter().rev() {
        properties.remove(idx);
    }
}

/// Build encoders for the transformed properties
fn build_encoders(
    properties: &[DecodedProperty],
    group_infos: &[GroupInfo],
) -> Vec<PropertyEncoder> {
    // Build a map from struct_name to GroupInfo for SharedDict lookup
    let struct_name_to_info: HashMap<&str, &GroupInfo> = group_infos
        .iter()
        .map(|info| (info.struct_name.as_str(), info))
        .collect();

    properties
        .iter()
        .map(|prop| build_encoder_for_property(prop, &struct_name_to_info))
        .collect()
}

/// Profile every string column
fn profile_string_columns(
    properties: &[DecodedProperty],
    min_hash: &MinHash<IntoIter<&str>, &str>,
) -> Vec<StringProfile> {
    properties
        .iter()
        .enumerate()
        .filter_map(|(idx, prop)| {
            if let PropValue::Str(values) = &prop.values {
                Some(profile_string_column(idx, values, min_hash))
            } else {
                None
            }
        })
        .collect()
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

fn build_encoder_for_property(
    prop: &DecodedProperty,
    struct_name_to_info: &HashMap<&str, &GroupInfo>,
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
            let non_null = v
                .iter()
                .flatten()
                .copied()
                .map(i32::from)
                .collect::<Vec<i32>>();
            let enc = encode_zigzag(&non_null);
            PropertyEncoder::Scalar(ScalarEncoder::int(presence, IntEncoder::auto_u32(&enc)))
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
            let non_null = v.iter().flatten().copied().collect::<Vec<i32>>();
            let enc = encode_zigzag(&non_null);
            PropertyEncoder::Scalar(ScalarEncoder::int(presence, IntEncoder::auto_u32(&enc)))
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
            let non_null = v.iter().flatten().copied().collect::<Vec<i64>>();
            let enc = encode_zigzag(&non_null);
            PropertyEncoder::Scalar(ScalarEncoder::int(presence, IntEncoder::auto_u64(&enc)))
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
            // Standalone scalar string (not grouped)
            let presence = to_presence(count_nulls(v));
            let non_null: Vec<&str> = v.iter().flatten().map(String::as_str).collect();
            scalar_str_encoder(presence, &non_null)
        }
        PropValue::SharedDict(_) => {
            // This is a grouped SharedDict created by the optimizer
            if let Some(info) = struct_name_to_info.get(prop.name.as_str()) {
                SharedDictEncoder {
                    dict_encoder: info.str_encoder,
                    items: info.item_encoders.clone(),
                }
                .into()
            } else {
                // SharedDict not from optimizer (e.g., from decoder) - use defaults
                // This shouldn't happen in normal optimizer flow
                debug_assert!(false, "SharedDict without matching GroupInfo in optimizer");
                PropertyEncoder::Scalar(ScalarEncoder::bool(PresenceStream::Absent))
            }
        }
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
            .chars()
            .zip(name.chars())
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
        let mut props = vec![make_prop(
            "pop",
            PropValue::U32(vec![Some(1), Some(2), Some(3)]),
        )];
        let enc = PropertyOptimizer::optimize(&mut props);
        let PropertyEncoder::Scalar(scalar) = &enc.properties[0] else {
            panic!("expected Scalar");
        };
        assert_eq!(scalar.optional, PresenceStream::Absent);
    }

    #[test]
    fn all_nulls_produces_present_presence() {
        let mut props = vec![make_prop("x", PropValue::I32(vec![None, None, None]))];
        let enc = PropertyOptimizer::optimize(&mut props);
        let PropertyEncoder::Scalar(scalar) = &enc.properties[0] else {
            panic!("expected Scalar");
        };
        assert_eq!(scalar.optional, PresenceStream::Present);
    }

    #[test]
    fn sequential_u32_picks_delta() {
        use crate::v01::{LogicalEncoder, PhysicalEncoder};
        let data: Vec<Option<u32>> = (0u32..1_000).map(Some).collect();
        let mut props = vec![make_prop("id", PropValue::U32(data))];
        let enc = PropertyOptimizer::optimize(&mut props);
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
        let mut props = vec![make_prop("val", PropValue::U32(data))];
        let enc = PropertyOptimizer::optimize(&mut props);
        let PropertyEncoder::Scalar(scalar) = &enc.properties[0] else {
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
            make_prop("addr:street", PropValue::Str(vocab.clone())),
            make_prop("addr:city", PropValue::Str(vocab)),
        ];
        let enc = PropertyOptimizer::optimize(&mut props);

        // Should produce one SharedDict property
        assert_eq!(props.len(), 1);
        assert_eq!(props[0].name, "addr:");
        let PropValue::SharedDict(items) = &props[0].values else {
            panic!("expected SharedDict");
        };
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].suffix, "street");
        assert_eq!(items[1].suffix, "city");

        // Encoder should be SharedDict
        assert_eq!(enc.properties.len(), 1);
        assert!(matches!(&enc.properties[0], PropertyEncoder::SharedDict(_)));
    }

    #[test]
    fn dissimilar_strings_stay_scalar() {
        let mut props = vec![
            make_prop(
                "city:de",
                PropValue::Str(vec![
                    Some("Munich".to_string()),
                    Some("Manheim".to_string()),
                    Some("Garching".to_string()),
                ]),
            ),
            make_prop(
                "city:Colorado",
                PropValue::Str(vec![
                    Some("Black".to_string()),
                    Some("Red".to_string()),
                    Some("Gold".to_string()),
                ]),
            ),
        ];
        let enc = PropertyOptimizer::optimize(&mut props);

        // Should stay as two separate properties (dissimilar values)
        assert_eq!(props.len(), 2);
        assert!(matches!(&props[0].values, PropValue::Str(_)));
        assert!(matches!(&props[1].values, PropValue::Str(_)));
        assert!(matches!(&enc.properties[0], PropertyEncoder::Scalar(_)));
        assert!(matches!(&enc.properties[1], PropertyEncoder::Scalar(_)));
    }
}
