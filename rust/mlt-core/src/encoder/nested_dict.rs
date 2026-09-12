//! Groups the string leaves of a layer's nested columns into shared dictionaries.

use std::collections::HashMap;

use crate::encoder::StagedValues;
use crate::encoder::nested::{StagedInterior, StagedNested, StagedNode};
use crate::encoder::property::shared_dict::{
    cluster_by_similarity, common_prefix_name, profile_all,
};

/// Where a leaf sits: its column's name, then its path inside that column.
type LeafAt = (String, String);

/// One corpus a layer's nested string leaves share.
pub(crate) struct NestedCorpus {
    pub(crate) name: String,
    /// Distinct entries, sorted, so every storage shape the corpus races numbers them alike.
    pub(crate) entries: Vec<String>,
}

/// A member leaf's link into one of the layer's corpora.
pub(crate) struct SharedLeaf {
    pub(crate) corpus: u32,
    /// One code per value the leaf marks present.
    pub(crate) codes: Vec<u32>,
}

/// The corpora a layer's nested string leaves share, and each member leaf's codes.
#[derive(Default)]
pub(crate) struct NestedDicts {
    pub(crate) corpora: Vec<NestedCorpus>,
    members: HashMap<LeafAt, SharedLeaf>,
}

impl NestedDicts {
    /// Cluster one column's string leaves by value similarity, one corpus per group.
    ///
    /// The corpora a column plans are written inside it, so the shape race costs each
    /// candidate whole rather than letting one externalise its dictionary.
    /// A struct whose string fields all land in one corpus has its map rewrite pointed at
    /// that same corpus, so both shapes read one corpus rather than one paying for a
    /// dictionary the other does not.
    pub(crate) fn plan(columns: &[StagedNested]) -> Self {
        let mut leaves: Vec<(LeafAt, &[String])> = Vec::new();
        for column in columns {
            collect_interior(&column.root, column.name(), "", &mut leaves);
        }
        let profiles = profile_all(leaves.iter().enumerate().map(|(i, ((_, path), values))| {
            let mut distinct: Vec<&str> = values.iter().map(String::as_str).collect();
            distinct.sort_unstable();
            distinct.dedup();
            (i, path.as_str(), distinct)
        }));

        let mut dicts = Self::default();
        let mut code_tables: Vec<HashMap<&str, u32>> = Vec::new();
        let mut joined: HashMap<&LeafAt, u32> = HashMap::new();
        for group in cluster_by_similarity(profiles) {
            let mut entries: Vec<&str> = group
                .iter()
                .flat_map(|p| p.unique_values.iter().copied())
                .collect();
            entries.sort_unstable();
            entries.dedup();
            let code_of: HashMap<&str, u32> = entries.iter().copied().zip(0_u32..).collect();

            let corpus = u32::try_from(dicts.corpora.len()).expect("a corpus per group fits u32");
            for profile in &group {
                let (at, values) = &leaves[profile.col_idx];
                let codes = values.iter().map(|v| code_of[v.as_str()]).collect();
                joined.insert(at, corpus);
                dicts
                    .members
                    .insert(at.clone(), SharedLeaf { corpus, codes });
            }
            dicts.corpora.push(NestedCorpus {
                name: common_prefix_name(&group),
                entries: entries.iter().map(|s| (*s).to_owned()).collect(),
            });
            code_tables.push(code_of);
        }

        let rewrites: Vec<(LeafAt, u32, Vec<u32>)> = columns
            .iter()
            .flat_map(|column| {
                let mut out = Vec::new();
                map_rewrites(
                    &column.root,
                    column.name(),
                    "",
                    &joined,
                    &code_tables,
                    &mut out,
                );
                out
            })
            .collect();
        for (at, corpus, codes) in rewrites {
            dicts.members.insert(at, SharedLeaf { corpus, codes });
        }
        dicts
    }

    /// The corpus link of the leaf at `path` inside `column`, or [`None`] when it has none.
    pub(crate) fn get(&self, column: &str, path: &str) -> Option<&SharedLeaf> {
        self.members.get(&(column.to_owned(), path.to_owned()))
    }

    /// The plan `root` alone needs: the corpora its own leaves index, renumbered from zero.
    ///
    /// A shape writes only what it reads, so the corpora it stores are the ones the race
    /// charges it for, and a shape that reads none writes nothing at all.
    pub(crate) fn restrict(&self, column: &str, root: &StagedInterior) -> Self {
        let mut paths = Vec::new();
        leaf_paths(root, "", &mut paths);

        let mut keep: Vec<u32> = paths
            .iter()
            .filter_map(|path| self.get(column, path).map(|link| link.corpus))
            .collect();
        keep.sort_unstable();
        keep.dedup();
        let renumbered: HashMap<u32, u32> = keep.iter().copied().zip(0_u32..).collect();

        let members = paths
            .into_iter()
            .filter_map(|path| {
                let link = self.get(column, &path)?;
                let link = SharedLeaf {
                    corpus: renumbered[&link.corpus],
                    codes: link.codes.clone(),
                };
                Some(((column.to_owned(), path), link))
            })
            .collect();
        let corpora = keep
            .into_iter()
            .map(|i| {
                let corpus = &self.corpora[i as usize];
                NestedCorpus {
                    name: corpus.name.clone(),
                    entries: corpus.entries.clone(),
                }
            })
            .collect();
        Self { corpora, members }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.corpora.is_empty()
    }
}

/// Every path a node below `root` is written at, leaves and interiors alike.
fn leaf_paths(interior: &StagedInterior, path: &str, out: &mut Vec<String>) {
    let mut node = |node: &StagedNode, path: String| match node {
        StagedNode::Interior(inner) => leaf_paths(inner, &path, out),
        StagedNode::Leaf(_) => out.push(path),
    };
    match interior {
        StagedInterior::Struct(n) => {
            for (name, field) in &n.fields {
                node(field, format!("{path}.{name}"));
            }
        }
        StagedInterior::List(n) => node(&n.element, format!("{path}[]")),
        StagedInterior::Map(n) => node(&n.value, format!("{path}{{}}")),
    }
}

/// Collect the string leaves below one interior node, in the paths the writer computes.
fn collect_interior<'a>(
    interior: &'a StagedInterior,
    column: &str,
    path: &str,
    out: &mut Vec<(LeafAt, &'a [String])>,
) {
    match interior {
        StagedInterior::Struct(node) => {
            for (name, field) in &node.fields {
                collect_node(field, column, &format!("{path}.{name}"), out);
            }
        }
        StagedInterior::List(node) => {
            collect_node(&node.element, column, &format!("{path}[]"), out);
        }
        // A map's value leaf already holds one dictionary over every key. It joins a corpus
        // only as the rewrite of a struct that joined one, which `map_rewrites` records.
        StagedInterior::Map(_) => {}
    }
}

fn collect_node<'a>(
    node: &'a StagedNode,
    column: &str,
    path: &str,
    out: &mut Vec<(LeafAt, &'a [String])>,
) {
    match node {
        StagedNode::Interior(interior) => collect_interior(interior, column, path, out),
        StagedNode::Leaf(leaf) => {
            if let StagedValues::Str(values) = &leaf.values
                && !values.is_empty()
            {
                out.push(((column.to_owned(), path.to_owned()), values));
            }
        }
    }
}

/// Point each struct's map rewrite at the corpus its fields joined, where they all joined one.
///
/// The rewrite holds the same values as the fields it replaces, so the corpus already has
/// every entry it needs and only the entry order its own leaf reads them in is new.
fn map_rewrites(
    interior: &StagedInterior,
    column: &str,
    path: &str,
    joined: &HashMap<&LeafAt, u32>,
    code_tables: &[HashMap<&str, u32>],
    out: &mut Vec<(LeafAt, u32, Vec<u32>)>,
) {
    match interior {
        StagedInterior::Struct(node) => {
            for (name, field) in &node.fields {
                if let StagedNode::Interior(inner) = field {
                    map_rewrites(
                        inner,
                        column,
                        &format!("{path}.{name}"),
                        joined,
                        code_tables,
                        out,
                    );
                }
            }
            let corpus = one_corpus(node.fields.iter().map(|(name, _)| {
                joined
                    .get(&(column.to_owned(), format!("{path}.{name}")))
                    .copied()
            }));
            let Some(corpus) = corpus else { return };
            let Some(StagedInterior::Map(rewrite)) = interior.as_map() else {
                return;
            };
            let StagedNode::Leaf(leaf) = rewrite.value.as_ref() else {
                return;
            };
            let StagedValues::Str(values) = &leaf.values else {
                return;
            };
            let table = &code_tables[corpus as usize];
            let codes = values.iter().map(|v| table[v.as_str()]).collect();
            out.push(((column.to_owned(), format!("{path}{{}}")), corpus, codes));
        }
        StagedInterior::List(node) => {
            if let StagedNode::Interior(inner) = node.element.as_ref() {
                map_rewrites(
                    inner,
                    column,
                    &format!("{path}[]"),
                    joined,
                    code_tables,
                    out,
                );
            }
        }
        StagedInterior::Map(node) => {
            if let StagedNode::Interior(inner) = node.value.as_ref() {
                map_rewrites(
                    inner,
                    column,
                    &format!("{path}{{}}"),
                    joined,
                    code_tables,
                    out,
                );
            }
        }
    }
}

/// The one corpus every field joined, or [`None`] unless they all joined the same one.
fn one_corpus(mut fields: impl Iterator<Item = Option<u32>>) -> Option<u32> {
    let first = fields.next()??;
    fields.all(|c| c == Some(first)).then_some(first)
}
