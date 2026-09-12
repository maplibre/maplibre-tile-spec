//! Columnar staging form of a layer's nested columns, shredded into a tree of nodes.

use crate::MltResult;
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

    /// The key set each row holds, read back out of the shapes.
    fn key_sets(shapes: &RowShapes) -> Vec<Vec<String>> {
        shapes
            .ids
            .iter()
            .map(|&id| {
                shapes.table[id as usize]
                    .iter()
                    .enumerate()
                    .filter(|&(_, held)| *held)
                    .map(|(bit, _)| shapes.keys[bit].clone())
                    .collect()
            })
            .collect()
    }

    fn str_map(lengths: Vec<u32>, keys: &[&str]) -> StagedMap {
        let values = StagedValues::I32((0..keys.len()).map(|_| 0).collect());
        StagedMap::new(
            None,
            lengths,
            keys.iter().map(|k| (*k).to_string()).collect(),
            StagedNode::Leaf(StagedLeaf::new(None, values)),
        )
    }

    #[test]
    fn map_shapes_read_back_as_the_key_set_of_every_row() {
        let node = str_map(vec![2, 1, 2], &["a", "b", "b", "a", "b"]);
        let shapes = node.row_shapes().expect("shapes");
        assert_eq!(shapes.keys, vec!["a", "b"]);
        assert_eq!(
            key_sets(&shapes),
            vec![vec!["a", "b"], vec!["b"], vec!["a", "b"]]
        );
    }

    #[test]
    fn map_shapes_table_holds_one_entry_per_distinct_key_set() {
        let node = str_map(vec![2, 1, 2], &["a", "b", "b", "a", "b"]);
        let shapes = node.row_shapes().expect("shapes");
        assert_eq!(shapes.table.len(), 2);
        assert_eq!(shapes.ids, vec![0, 1, 0]);
    }

    #[test]
    fn a_rows_entry_count_is_the_population_count_of_its_shape() {
        let node = str_map(vec![2, 1, 2], &["a", "b", "b", "a", "b"]);
        let shapes = node.row_shapes().expect("shapes");
        let counts: Vec<usize> = shapes
            .ids
            .iter()
            .map(|&id| shapes.table[id as usize].iter().filter(|&&b| b).count())
            .collect();
        let lengths: Vec<usize> = node.lengths.iter().map(|&l| l as usize).collect();
        assert_eq!(counts, lengths);
    }

    #[test]
    fn map_shapes_are_found_for_rows_that_run_in_sorted_key_order() {
        let node = str_map(
            vec![2, 3],
            &["name", "name_en", "name", "name_de", "name_en"],
        );
        let shapes = node.row_shapes().expect("shapes");
        assert_eq!(shapes.keys, vec!["name", "name_de", "name_en"]);
        assert_eq!(
            key_sets(&shapes),
            vec![vec!["name", "name_en"], vec!["name", "name_de", "name_en"]]
        );
    }

    #[test]
    fn map_shapes_are_found_for_rows_that_run_in_field_order() {
        let node = str_map(vec![2, 2], &["b", "a", "b", "a"]);
        let shapes = node.row_shapes().expect("shapes");
        assert_eq!(shapes.keys, vec!["b", "a"]);
        assert_eq!(key_sets(&shapes), vec![vec!["b", "a"], vec!["b", "a"]]);
    }

    #[test]
    fn a_map_row_that_repeats_a_key_has_no_shapes() {
        assert_eq!(str_map(vec![2], &["a", "a"]).row_shapes(), None);
    }

    #[test]
    fn a_map_of_one_key_has_no_shapes() {
        assert_eq!(str_map(vec![1, 1], &["a", "a"]).row_shapes(), None);
    }

    #[test]
    fn struct_shapes_read_back_as_the_presence_of_every_field() {
        let node = StagedStruct::new(
            None,
            vec![
                ("a", leaf(Some(vec![true, false, true]), vec![1, 3])),
                ("b", leaf(Some(vec![false, true, true]), vec![2, 4])),
            ],
        );
        let shapes = node.row_shapes().expect("shapes");
        assert_eq!(shapes.ids, vec![0, 1, 2]);
        let held = |field: usize| -> Vec<bool> {
            shapes
                .ids
                .iter()
                .map(|&id| shapes.table[id as usize][field])
                .collect()
        };
        assert_eq!(held(0), vec![true, false, true]);
        assert_eq!(held(1), vec![false, true, true]);
    }

    #[test]
    fn a_struct_whose_every_field_is_always_present_has_no_shapes() {
        let node = StagedStruct::new(
            None,
            vec![("a", leaf(None, vec![1])), ("b", leaf(None, vec![2]))],
        );
        assert_eq!(node.row_shapes(), None);
    }

    #[test]
    fn a_struct_of_one_field_has_no_shapes() {
        let node = StagedStruct::new(None, vec![("a", leaf(Some(vec![true, false]), vec![1]))]);
        assert_eq!(node.row_shapes(), None);
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

/// Which keys each row of a struct or a map holds, coded once per distinct key set.
///
/// A struct spends one presence bit per (row, field) and a map one key per entry.
/// Both say the same thing along the wrong axis: real key sets barely vary, so one
/// id per row into a table of key-set bitmaps replaces either.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RowShapes {
    /// The distinct key sets, each a bitmap over the key list, LSB-first.
    pub(crate) table: Vec<Vec<bool>>,
    /// One index into [`Self::table`] per row the node marks present.
    pub(crate) ids: Vec<u32>,
    /// The keys the bitmaps run over, in bit order. Empty for a struct, whose fields are the keys.
    pub(crate) keys: Vec<String>,
}

impl RowShapes {
    /// The table as one run of `shape_count * width` bits.
    pub(crate) fn table_bits(&self) -> Vec<bool> {
        self.table.concat()
    }

    /// Intern one row's key set, returning its id.
    fn intern(table: &mut Vec<Vec<bool>>, shape: Vec<bool>) -> MltResult<u32> {
        let id = table
            .iter()
            .position(|known| *known == shape)
            .unwrap_or_else(|| {
                table.push(shape);
                table.len() - 1
            });
        Ok(u32::try_from(id)?)
    }
}

impl StagedStruct {
    /// This struct's field presence as one shape id per row, or [`None`] when there is nothing to code.
    ///
    /// A struct whose every field is present on every row already stores no presence at all.
    pub(crate) fn row_shapes(&self) -> Option<RowShapes> {
        if self.fields.len() < 2 || self.fields.iter().all(|(_, f)| f.presence().is_none()) {
            return None;
        }
        let rows = self.present_count();
        let mut table = Vec::new();
        let mut ids = Vec::with_capacity(rows);
        for row in 0..rows {
            let shape = self
                .fields
                .iter()
                .map(|(_, field)| {
                    field
                        .presence()
                        .is_none_or(|bits| bits.get(row).copied().unwrap_or(false))
                })
                .collect();
            ids.push(RowShapes::intern(&mut table, shape).ok()?);
        }
        Some(RowShapes {
            table,
            ids,
            keys: Vec::new(),
        })
    }
}

impl StagedMap {
    /// This map's per-entry keys as one shape id per row, or [`None`] when no key order says it.
    ///
    /// A bitmap names each key once, in key-list order, so it can only stand in for
    /// a row whose entries run in that order and repeat no key. Two orders are worth
    /// trying: sorted, which is the order a map built from the model arrives in, and
    /// first-seen, which is the order a struct reshreds its fields in.
    pub(crate) fn row_shapes(&self) -> Option<RowShapes> {
        let mut sorted: Vec<String> = self.keys.clone();
        sorted.sort_unstable();
        sorted.dedup();
        if sorted.len() < 2 {
            return None;
        }
        self.shapes_over(&sorted).or_else(|| {
            let mut seen: Vec<String> = Vec::with_capacity(sorted.len());
            for key in &self.keys {
                if !seen.contains(key) {
                    seen.push(key.clone());
                }
            }
            self.shapes_over(&seen)
        })
    }

    /// The shapes this map's rows have over `keys`, or [`None`] when a row does not run in that order.
    fn shapes_over(&self, keys: &[String]) -> Option<RowShapes> {
        let index: Vec<usize> = self
            .keys
            .iter()
            .map(|key| keys.iter().position(|known| known == key))
            .collect::<Option<_>>()?;

        let mut table = Vec::new();
        let mut ids = Vec::with_capacity(self.lengths.len());
        let mut at = 0_usize;
        for &length in &self.lengths {
            let entries = index.get(at..at + length as usize)?;
            if entries.windows(2).any(|pair| pair[0] >= pair[1]) {
                return None;
            }
            let mut shape = vec![false; keys.len()];
            for &key in entries {
                shape[key] = true;
            }
            ids.push(RowShapes::intern(&mut table, shape).ok()?);
            at += length as usize;
        }
        Some(RowShapes {
            table,
            ids,
            keys: keys.to_vec(),
        })
    }
}

impl StagedInterior {
    /// This node's children's structure as one shape id per row, for the kinds that have one.
    pub(crate) fn row_shapes(&self) -> Option<RowShapes> {
        match self {
            Self::Struct(node) => node.row_shapes(),
            Self::Map(node) => node.row_shapes(),
            Self::List(_) => None,
        }
    }
}

impl StagedNode {
    /// This node's presence over the values its parent hands it.
    fn presence(&self) -> Option<&Vec<bool>> {
        match self {
            Self::Interior(interior) => interior.presence(),
            Self::Leaf(leaf) => leaf.presence.as_ref(),
        }
    }
}
