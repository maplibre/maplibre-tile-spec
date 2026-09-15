//! Columnar staging form of a layer's nested columns, shredded into a tree of nodes.

use crate::encoder::StagedValues;

/// A nested column prepared for encoding: a name and the tree its values shred into.
///
/// The root's presence is the column's own, one bit per feature, so it is the mask
/// the layer's shared bitfields compete over.
#[derive(Debug, Clone, PartialEq)]
pub struct StagedNested {
    pub name: String,
    pub root: StagedInterior,
}

/// What a nested column's root may be.
///
/// A leaf root is an ordinary column, so it is not here.
#[derive(Debug, Clone, PartialEq)]
pub enum StagedInterior {
    Struct(StagedStruct),
    List(StagedList),
    Map(StagedMap),
}

/// One node of a staged tree.
#[derive(Debug, Clone, PartialEq)]
pub enum StagedNode {
    Interior(StagedInterior),
    Leaf(StagedLeaf),
}

/// A staged struct node: a fixed set of named, individually typed fields.
#[derive(Debug, Clone, PartialEq)]
pub struct StagedStruct {
    pub(crate) presence: Option<Vec<bool>>,
    pub(crate) fields: Vec<(String, StagedNode)>,
}

/// A staged list node: one length per present list, then one element node.
#[derive(Debug, Clone, PartialEq)]
pub struct StagedList {
    pub(crate) presence: Option<Vec<bool>>,
    pub(crate) lengths: Vec<u32>,
    pub(crate) element: Box<StagedNode>,
}

/// A staged map node: one length per present map, one key per entry, then one value node.
#[derive(Debug, Clone, PartialEq)]
pub struct StagedMap {
    pub(crate) presence: Option<Vec<bool>>,
    pub(crate) lengths: Vec<u32>,
    pub(crate) keys: Vec<String>,
    pub(crate) value: Box<StagedNode>,
}

/// A staged leaf node, holding the flat values a column of its type holds.
#[derive(Debug, Clone, PartialEq)]
pub struct StagedLeaf {
    pub(crate) presence: Option<Vec<bool>>,
    pub(crate) values: StagedValues,
}

impl StagedNested {
    #[must_use]
    pub fn new(name: impl Into<String>, root: StagedInterior) -> Self {
        Self {
            name: name.into(),
            root,
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The column's presence over features, or [`None`] when every feature has a value.
    #[must_use]
    pub(crate) fn presence(&self) -> Option<&Vec<bool>> {
        self.root.presence()
    }
}

impl StagedStruct {
    /// A struct of `fields`, present on every value its parent hands it when
    /// `presence` is [`None`].
    #[must_use]
    pub fn new<K: Into<String>>(
        presence: Option<Vec<bool>>,
        fields: impl IntoIterator<Item = (K, StagedNode)>,
    ) -> Self {
        Self {
            presence,
            fields: fields.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        }
    }
}

impl StagedList {
    #[must_use]
    pub fn new(presence: Option<Vec<bool>>, lengths: Vec<u32>, element: StagedNode) -> Self {
        Self {
            presence,
            lengths,
            element: Box::new(element),
        }
    }
}

impl StagedMap {
    #[must_use]
    pub fn new(
        presence: Option<Vec<bool>>,
        lengths: Vec<u32>,
        keys: Vec<String>,
        value: StagedNode,
    ) -> Self {
        Self {
            presence,
            lengths,
            keys,
            value: Box::new(value),
        }
    }
}

impl StagedLeaf {
    #[must_use]
    pub fn new(presence: Option<Vec<bool>>, values: StagedValues) -> Self {
        Self { presence, values }
    }
}

impl StagedInterior {
    /// The map that says the same thing as this node, for a struct that has one.
    #[must_use]
    pub(crate) fn as_map(&self) -> Option<Self> {
        match self {
            Self::Struct(node) => node.as_map().map(Self::Map),
            Self::List(_) | Self::Map(_) => None,
        }
    }

    #[must_use]
    pub(crate) fn presence(&self) -> Option<&Vec<bool>> {
        match self {
            Self::Struct(node) => node.presence.as_ref(),
            Self::List(node) => node.presence.as_ref(),
            Self::Map(node) => node.presence.as_ref(),
        }
    }

    /// How many values this node's parent hands it.
    #[must_use]
    pub(crate) fn parent_count(&self) -> usize {
        match self {
            Self::Struct(node) => node.presence.as_ref().map_or_else(
                || node.fields.first().map_or(0, |(_, f)| f.parent_count()),
                Vec::len,
            ),
            Self::List(node) => node.presence.as_ref().map_or(node.lengths.len(), Vec::len),
            Self::Map(node) => node.presence.as_ref().map_or(node.lengths.len(), Vec::len),
        }
    }
}

impl StagedNode {
    /// How many values this node's parent hands it.
    #[must_use]
    pub(crate) fn parent_count(&self) -> usize {
        match self {
            Self::Interior(interior) => interior.parent_count(),
            Self::Leaf(leaf) => leaf
                .presence
                .as_ref()
                .map_or_else(|| leaf.values.count(), Vec::len),
        }
    }

    /// How many values this node marks present, which is what its own streams hold.
    #[must_use]
    pub(crate) fn present_count(&self) -> usize {
        let presence = match self {
            Self::Interior(interior) => interior.presence(),
            Self::Leaf(leaf) => leaf.presence.as_ref(),
        };
        presence.map_or_else(
            || self.parent_count(),
            |bits| bits.iter().filter(|&&bit| bit).count(),
        )
    }

    /// The leaf values this node holds, or [`None`] when it holds other nodes.
    fn leaf(&self) -> Option<&StagedLeaf> {
        match self {
            Self::Leaf(leaf) => Some(leaf),
            Self::Interior(_) => None,
        }
    }
}

impl StagedStruct {
    /// How many values this struct marks present, which is what each of its fields is handed.
    fn present_count(&self) -> usize {
        match &self.presence {
            Some(bits) => bits.iter().filter(|&&bit| bit).count(),
            None => self
                .fields
                .first()
                .map_or(0, |(_, field)| field.parent_count()),
        }
    }

    /// The same values written as a map node: one key per entry rather than one presence stream per field.
    /// Only a struct whose fields are leaves of one type, each with something present, has one.
    #[must_use]
    pub(crate) fn as_map(&self) -> Option<StagedMap> {
        let leaves: Vec<&StagedLeaf> = self
            .fields
            .iter()
            .map(|(_, node)| node.leaf())
            .collect::<Option<_>>()?;
        let first = leaves.first()?;
        if leaves
            .iter()
            .any(|leaf| !leaf.values.same_kind(&first.values))
        {
            return None;
        }

        let rows = self.present_count();
        if leaves.iter().any(|leaf| present_entries(leaf, rows) == 0) {
            return None;
        }

        let mut cursors = vec![0_usize; leaves.len()];
        let mut lengths = Vec::with_capacity(rows);
        let mut keys = Vec::new();
        let mut picks = Vec::new();
        for row in 0..rows {
            let mut length = 0_u32;
            for (field, leaf) in leaves.iter().enumerate() {
                let present = leaf
                    .presence
                    .as_ref()
                    .is_none_or(|bits| bits.get(row).copied().unwrap_or(false));
                if !present {
                    continue;
                }
                picks.push((field, cursors[field]));
                cursors[field] += 1;
                keys.push(self.fields[field].0.clone());
                length += 1;
            }
            lengths.push(length);
        }

        let values = gather(&leaves, &picks);
        Some(StagedMap::new(
            self.presence.clone(),
            lengths,
            keys,
            StagedNode::Leaf(StagedLeaf::new(None, values)),
        ))
    }
}

/// How many of the `rows` values a field is handed it marks present.
fn present_entries(leaf: &StagedLeaf, rows: usize) -> usize {
    leaf.presence.as_ref().map_or(rows, |bits| {
        bits.iter().take(rows).filter(|&&bit| bit).count()
    })
}

macro_rules! impl_gather {
    (
        scalar { $($sv:ident),* $(,)? }
        string { $($gv:ident),* $(,)? }
    ) => {
        impl StagedValues {
            /// Whether both hold the same type of value, which a map node requires of its fields.
            #[must_use]
            pub(crate) fn same_kind(&self, other: &Self) -> bool {
                match (self, other) {
                    $((Self::$sv(_), Self::$sv(_)) => true,)*
                    $((Self::$gv(_), Self::$gv(_)) => true,)*
                    _ => false,
                }
            }
        }

        /// Read one value out of each `(field, dense index)` pair, in entry order.
        ///
        /// Every leaf holds the same type, which the caller has already checked.
        fn gather(leaves: &[&StagedLeaf], picks: &[(usize, usize)]) -> StagedValues {
            /// Pick the values of one type out of the leaves that hold them.
            macro_rules! pick {
                ($variant:ident) => {
                    StagedValues::$variant(
                        picks
                            .iter()
                            .map(|&(field, index)| match &leaves[field].values {
                                StagedValues::$variant(values) => values[index].clone(),
                                _ => unreachable!("every field holds the same type"),
                            })
                            .collect(),
                    )
                };
            }
            match leaves[0].values {
                $(StagedValues::$sv(_) => pick!($sv),)*
                $(StagedValues::$gv(_) => pick!($gv),)*
            }
        }
    };
}

with_kinds!(impl_gather);

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(presence: Option<Vec<bool>>, values: Vec<i32>) -> StagedNode {
        StagedNode::Leaf(StagedLeaf::new(presence, StagedValues::I32(values)))
    }

    #[test]
    fn a_struct_of_one_type_reshreds_into_a_map_row_by_row() {
        let node = StagedStruct::new(
            None,
            vec![
                ("a", leaf(Some(vec![true, false, true]), vec![1, 3])),
                ("b", leaf(Some(vec![false, true, true]), vec![2, 4])),
            ],
        );
        let map = node.as_map().expect("a map");
        assert_eq!(map.lengths, vec![1, 1, 2]);
        assert_eq!(map.keys, vec!["a", "b", "a", "b"]);
        let StagedNode::Leaf(value) = *map.value else {
            panic!("a leaf")
        };
        assert_eq!(value.presence, None);
        assert_eq!(value.values, StagedValues::I32(vec![1, 2, 3, 4]));
    }

    #[test]
    fn a_struct_whose_fields_hold_different_types_has_no_map() {
        let node = StagedStruct::new(
            None,
            vec![
                ("a", leaf(None, vec![1])),
                (
                    "b",
                    StagedNode::Leaf(StagedLeaf::new(
                        None,
                        StagedValues::Str(vec!["x".to_string()]),
                    )),
                ),
            ],
        );
        assert_eq!(node.as_map(), None);
    }

    #[test]
    fn a_struct_whose_field_is_present_nowhere_has_no_map() {
        let node = StagedStruct::new(
            None,
            vec![
                ("a", leaf(Some(vec![true, true]), vec![1, 2])),
                ("b", leaf(Some(vec![false, false]), vec![])),
            ],
        );
        assert_eq!(node.as_map(), None);
    }

    #[test]
    fn a_struct_present_on_no_value_has_no_map() {
        let node = StagedStruct::new(Some(vec![false, false]), vec![("a", leaf(None, vec![]))]);
        assert_eq!(node.as_map(), None);
    }

    #[test]
    fn a_struct_whose_field_holds_more_nodes_has_no_map() {
        let inner = StagedNode::Interior(StagedInterior::Struct(StagedStruct::new(
            None,
            vec![("deep", leaf(None, vec![1]))],
        )));
        let node = StagedStruct::new(None, vec![("a", inner)]);
        assert_eq!(node.as_map(), None);
    }
}
