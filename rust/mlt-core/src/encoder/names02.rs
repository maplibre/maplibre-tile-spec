//! The table of names a tile's v2 layers share, and the record that carries it.
//!
//! Layer and column names are the one thing v2 repeats verbatim across a tile:
//! six layers of the same tileset name the same `class`, `name`, `subclass`
//! columns over and over. A tile that carries a table of the names more than one
//! of its layers writes replaces each of those with a one-byte index.
//!
//! Names used once are left where they are, since a table entry plus an index
//! costs more than the name did. Telling the two apart takes one bit, which the
//! name field's leading varint carries: `2 * length` for a name that follows,
//! `2 * index + 1` for one the table holds.

use std::collections::HashMap;

use integer_encoding::VarIntWriter as _;

use crate::encoder::optimizer::{LayerStats, SharedDictRole};
use crate::tile::TileLayer;
use crate::utils::BinarySerializer as _;
use crate::{MltError, MltResult};

/// Wire tag of the record holding a tile's name table.
pub(crate) const NAME_TABLE_TAG: u8 = 3;

/// The names of one tile that more than one layer or column writes.
#[derive(Debug, Default)]
pub struct NameTable {
    /// The entries in index order, hottest first, so the busiest names index in one byte.
    entries: Vec<String>,
    index: HashMap<String, u32>,
}

impl NameTable {
    /// Plan the table for a tile from its layers and their analyses.
    ///
    /// Counting from the analysis rather than from the bytes is what keeps the
    /// table stable: sort trials and encoding alternatives write the same names
    /// several times over, and only the winning pass survives.
    #[must_use]
    pub(crate) fn plan(layers: &[(TileLayer, LayerStats)]) -> Self {
        let mut uses: HashMap<&str, u32> = HashMap::new();
        for (layer, stats) in layers {
            for name in wire_names(layer, stats) {
                *uses.entry(name).or_default() += 1;
            }
        }
        let mut hot: Vec<(&str, u32)> = uses.into_iter().filter(|&(_, n)| n > 1).collect();
        // Most-used first, then by name, so the table does not depend on hashing order.
        hot.sort_unstable_by_key(|&(name, n)| (std::cmp::Reverse(n), name));
        let entries: Vec<String> = hot.into_iter().map(|(name, _)| name.to_owned()).collect();
        let index = entries
            .iter()
            .enumerate()
            .map(|(i, name)| (name.clone(), u32::try_from(i).expect("index fits")))
            .collect();
        Self { entries, index }
    }

    /// Where `name` sits in the table, if the table holds it.
    #[must_use]
    pub(crate) fn index_of(&self, name: &str) -> Option<u32> {
        self.index.get(name).copied()
    }

    /// Whether the table is worth a record of its own.
    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The complete `[varint size + 1][tag][varint count][entries]` record.
    pub(crate) fn to_record(&self) -> MltResult<Vec<u8>> {
        let mut body = Vec::new();
        body.write_varint(u32::try_from(self.entries.len())?)?;
        for entry in &self.entries {
            body.write_string(entry)?;
        }
        let mut out = Vec::with_capacity(body.len() + 6);
        out.write_varint(u32::try_from(body.len() + 1)?)
            .map_err(MltError::from)?;
        out.push(NAME_TABLE_TAG);
        out.append(&mut body);
        Ok(out)
    }
}

/// Write a name field: `2 * index + 1` when the table holds it, otherwise
/// `2 * length` and the bytes.
pub(crate) fn write_name02(
    data: &mut Vec<u8>,
    table: Option<&NameTable>,
    name: &str,
) -> MltResult<()> {
    let Some(table) = table else {
        return data.write_string(name).map_err(MltError::from);
    };
    if let Some(index) = table.index_of(name) {
        data.write_varint(2 * index + 1)?;
    } else {
        data.write_varint(2 * u32::try_from(name.len())?)?;
        data.extend_from_slice(name.as_bytes());
    }
    Ok(())
}

/// Every name a layer writes on the wire, in no particular order.
///
/// Mirrors what `StagedLayer::from_tile` stages: a shared dictionary writes its
/// prefix once and a suffix per member, an all-null column writes nothing at all.
fn wire_names<'a>(layer: &'a TileLayer, stats: &'a LayerStats) -> Vec<&'a str> {
    use crate::encoder::optimizer::Presence;

    let mut names = vec![layer.name()];
    for (col, prop) in stats.properties.iter().enumerate() {
        let Some(name) = layer.property_names().get(col) else {
            continue;
        };
        match prop.stats.shared_dict() {
            SharedDictRole::Owner(prefix) => {
                // The prefix is written once for the group, then a suffix per member.
                let at = name.len() - name.strip_prefix(&prefix).unwrap_or(name).len();
                names.push(&name[..at]);
                names.push(&name[at..]);
            }
            SharedDictRole::Member(owner) => {
                let SharedDictRole::Owner(prefix) = stats.properties[owner].stats.shared_dict()
                else {
                    continue;
                };
                let at = name.len() - name.strip_prefix(&prefix).unwrap_or(name).len();
                names.push(&name[at..]);
            }
            SharedDictRole::None if prop.presence != Presence::AllNull => names.push(name),
            SharedDictRole::None => {}
        }
    }
    names
}
