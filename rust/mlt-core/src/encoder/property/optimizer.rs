//! Optimizer that groups string columns into shared dictionaries using `MinHash`
//! similarity, then hands off to per-column auto-encoders.

use std::collections::HashMap;
use std::hash::Hash;

use probabilistic_collections::similarity::MinHash;
use union_find::{QuickUnionUf, UnionBySize, UnionFind as _};

use super::encode::write_properties;
use crate::MltResult;
use crate::decoder::{PropValue, TileLayer01};
use crate::encoder::Encoder;

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

/// Extension trait for consuming-style encoding of staged property columns directly
/// into an [`Encoder`].
///
/// The returned count reflects how many columns were actually written; all-null/empty
/// columns are omitted from the wire format.
pub trait EncodeProperties: Sized {
    /// Automatically select per-column encoders, encode, and write to `enc`,
    /// consuming `self`.
    fn write_to(self, enc: &mut Encoder) -> MltResult<u32>;
}

impl EncodeProperties for Vec<super::model::StagedProperty> {
    fn write_to(self, enc: &mut Encoder) -> MltResult<u32> {
        let before = enc.layer_column_count;
        write_properties(&self, None, enc)?;
        Ok(enc.layer_column_count - before)
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
