//! Nested property columns, the shredded trees that only a v2 layer carries.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::ops::Range;

use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;
use bitvec::view::BitView as _;
use usize_cast::IntoUsize as _;

use crate::codecs::varint::parse_varint;
use crate::decoder::root02::{ColumnValues, parse_column_values, parse_strings};
use crate::decoder::stream::header02;
use crate::decoder::stream::header02::{Count02, HAS_EXPLICIT_COUNT, PhysicalBits, StreamCtx02};
use crate::decoder::{
    BoolLogical, Interior02, LogicalEncoding, MValues, NodeKind02, NodePresence, NodeType02,
    ParsedStrings, PhysicalEncoding, RawPresence, RawStream, RawStrings,
};
use crate::tile::{MAX_NESTED_DEPTH, NestedKind, NestedValue, PropValue};
use crate::utils::{parse_string, parse_u8};
use crate::{Decode, DecodeState, Decoder, Lazy, MltError, MltRefResult, MltResult, Parser};

/// A nested column, parameterized by decode state, mirroring `Property`.
pub type Nested<'a, S = Lazy> = <S as DecodeState>::LazyOrParsed<RawNested<'a>, ParsedNested<'a>>;

/// What one node of a tree costs the parser's budget, so a deep or wide tree is
/// charged for even before its streams are.
const NODE_COST: u32 = 64;

/// Encoding byte of a raw packed bitmap, which is the `Bool` family's logical `None` over physical `01`.
const PACKED_BITMAP: u8 = PhysicalBits::WithLen as u8;

/// A raw nested column as read directly from the tile.
#[derive(Debug, Clone, PartialEq)]
pub struct RawNested<'a> {
    name: &'a str,
    presence: RawPresence<'a>,
    /// The column's value count, which is what its root is handed.
    value_count: u32,
    root: RawInterior<'a>,
}

/// A raw node of a nested tree: more nodes, or the values a leaf holds.
#[derive(Debug, Clone, PartialEq)]
pub enum RawNode<'a> {
    Interior(RawInterior<'a>),
    Leaf(RawLeaf<'a>),
}

/// A raw interior node, which holds other nodes rather than values.
#[derive(Debug, Clone, PartialEq)]
pub enum RawInterior<'a> {
    Struct(RawStruct<'a>),
    List(RawList<'a>),
    Map(RawMap<'a>),
}

/// A raw struct node: a fixed set of named, individually typed fields.
#[derive(Debug, Clone, PartialEq)]
pub struct RawStruct<'a> {
    presence: Option<RawStream<'a>>,
    fields: Vec<(&'a str, RawNode<'a>)>,
}

/// A raw list node: one length per present list, then the element node.
#[derive(Debug, Clone, PartialEq)]
pub struct RawList<'a> {
    presence: Option<RawStream<'a>>,
    lengths: RawStream<'a>,
    element: Box<RawNode<'a>>,
}

/// A raw map node: one length per present map, the keys of every entry, then the value node.
#[derive(Debug, Clone, PartialEq)]
pub struct RawMap<'a> {
    presence: Option<RawStream<'a>>,
    lengths: RawStream<'a>,
    /// Boxed, since a string column's stream set dwarfs every other node's fields.
    keys: Box<RawStrings<'a>>,
    value: Box<RawNode<'a>>,
}

/// A raw leaf node, holding the stream set a column of its data type holds.
#[derive(Debug, Clone, PartialEq)]
pub struct RawLeaf<'a> {
    presence: Option<RawStream<'a>>,
    /// Boxed, since a string column's stream set dwarfs every other node's fields.
    values: Box<ColumnValues<'a>>,
}

impl RawNested<'_> {
    #[must_use]
    pub fn name(&self) -> &str {
        self.name
    }
}

// ── Parsing ───────────────────────────────────────────────────────────────────

/// Parse a nested column's body: the tree its root type byte named.
///
/// `value_count` is the column's value count, which is what the root is handed.
pub(crate) fn parse_nested<'a>(
    input: &'a [u8],
    name: &'a str,
    presence: RawPresence<'a>,
    kind: Interior02,
    value_count: u32,
    parser: &mut Parser,
) -> MltRefResult<'a, RawNested<'a>> {
    parser.reserve(NODE_COST)?;
    // A lengths stream's sum is only known once it is decoded, so nothing implies
    // a count at or below the first list or map on the path from the root.
    let count = if kind.has_lengths() {
        Count02::Explicit
    } else {
        Count02::Implied(value_count)
    };
    let (input, root) = parse_interior(input, name, kind, None, count, 1, parser)?;
    Ok((
        input,
        RawNested {
            name,
            presence,
            value_count,
            root,
        },
    ))
}

/// Parse one node below the root: its type byte, its name if it has one, its
/// presence stream, then its body.
///
/// `named` is true for a struct field and false for a list element or a map value.
fn parse_node<'a>(
    input: &'a [u8],
    path: &str,
    named: bool,
    parent_count: Count02,
    depth: usize,
    parser: &mut Parser,
) -> MltRefResult<'a, (&'a str, RawNode<'a>)> {
    if depth > MAX_NESTED_DEPTH {
        return Err(MltError::NestedTooDeep(depth));
    }
    parser.reserve(NODE_COST)?;
    let (input, typ_byte) = parse_u8(input)?;
    let typ = NodeType02::parse(typ_byte)?;
    let (input, name) = if named {
        parse_string(input)?
    } else {
        (input, "")
    };
    let path = if name.is_empty() {
        path.to_string()
    } else {
        format!("{path}.{name}")
    };

    // A list or a map ends the implied counts, its own streams included.
    let node_count = match typ.data.interior() {
        Some(kind) if kind.has_lengths() => Count02::Explicit,
        _ => parent_count,
    };
    let (input, presence) = match typ.presence {
        NodePresence::AllPresent => (input, None),
        NodePresence::Stream => {
            require_bitmap_presence(input, &path)?;
            require_explicit_count(input, &path, node_count)?;
            let (input, stream) =
                header02::parse_stream(input, StreamCtx02::NestedPresence, node_count, parser)?;
            (input, Some(stream))
        }
    };
    // A node's data streams run over the values it marks present, its presence
    // stream over every value its parent handed it.
    let body_count = match (node_count, &presence) {
        (Count02::Explicit, _) => Count02::Explicit,
        (Count02::Implied(_), Some(stream)) => Count02::Implied(presence_popcount(stream)?),
        (Count02::Implied(count), None) => Count02::Implied(count),
    };

    let (input, node) = match typ.data {
        NodeKind02::Leaf(values) => {
            require_explicit_count(input, &path, body_count)?;
            let (input, values) = parse_column_values(
                input,
                values,
                name,
                RawPresence::AllPresent,
                body_count,
                parser,
            )?;
            let values = Box::new(values);
            (input, RawNode::Leaf(RawLeaf { presence, values }))
        }
        NodeKind02::Struct | NodeKind02::List | NodeKind02::Map => {
            let kind = typ
                .data
                .interior()
                .expect("an interior node names an interior");
            let (input, interior) =
                parse_interior(input, &path, kind, presence, body_count, depth, parser)?;
            (input, RawNode::Interior(interior))
        }
    };
    Ok((input, (name, node)))
}

/// Parse the body of an interior node, which its own presence has already been read for.
fn parse_interior<'a>(
    input: &'a [u8],
    path: &str,
    kind: Interior02,
    presence: Option<RawStream<'a>>,
    count: Count02,
    depth: usize,
    parser: &mut Parser,
) -> MltRefResult<'a, RawInterior<'a>> {
    Ok(match kind {
        Interior02::Struct => {
            let (mut input, field_count) = parse_varint::<u32>(input)?;
            if field_count == 0 {
                return Err(MltError::EmptyStructNode);
            }
            parser.reserve(field_count.saturating_mul(NODE_COST))?;
            let mut fields: Vec<(&'a str, RawNode<'a>)> =
                Vec::with_capacity(field_count.into_usize());
            for _ in 0..field_count {
                let field;
                (input, field) = parse_node(input, path, true, count, depth + 1, parser)?;
                if fields.iter().any(|(name, _)| *name == field.0) {
                    return Err(MltError::DuplicateFieldName(field.0.to_string()));
                }
                fields.push(field);
            }
            (input, RawInterior::Struct(RawStruct { presence, fields }))
        }
        Interior02::List => {
            require_explicit_count(input, path, count)?;
            let (input, lengths) =
                header02::parse_stream(input, StreamCtx02::NestedLengths, count, parser)?;
            let (input, (_, element)) =
                parse_node(input, path, false, Count02::Explicit, depth + 1, parser)?;
            (
                input,
                RawInterior::List(RawList {
                    presence,
                    lengths,
                    element: Box::new(element),
                }),
            )
        }
        Interior02::Map => {
            require_explicit_count(input, path, count)?;
            let (input, lengths) =
                header02::parse_stream(input, StreamCtx02::NestedLengths, count, parser)?;
            require_explicit_count(input, path, Count02::Explicit)?;
            let (input, keys) = parse_strings(
                input,
                "",
                RawPresence::AllPresent,
                Count02::Explicit,
                parser,
            )?;
            let (input, (_, value)) =
                parse_node(input, path, false, Count02::Explicit, depth + 1, parser)?;
            (
                input,
                RawInterior::Map(RawMap {
                    presence,
                    lengths,
                    keys: Box::new(keys),
                    value: Box::new(value),
                }),
            )
        }
    })
}

/// Reject a stream whose header carries no count where nothing in its context implies one.
fn require_explicit_count(input: &[u8], path: &str, count: Count02) -> MltResult<()> {
    if count != Count02::Explicit {
        return Ok(());
    }
    let (_, enc_byte) = parse_u8(input)?;
    if enc_byte & HAS_EXPLICIT_COUNT == 0 {
        return Err(MltError::NestedImplicitCount {
            name: path.to_string(),
            byte: enc_byte,
        });
    }
    Ok(())
}

/// Reject a presence stream that is not the raw bitmap the format writes one as.
/// Its encoding byte must name the `Bool` family's logical `None` over physical `01`, and nothing else.
fn require_bitmap_presence(input: &[u8], path: &str) -> MltResult<()> {
    let (_, enc_byte) = parse_u8(input)?;
    if enc_byte & !HAS_EXPLICIT_COUNT != PACKED_BITMAP {
        return Err(MltError::NestedPresenceEncoding {
            name: path.to_string(),
            byte: enc_byte,
        });
    }
    Ok(())
}

/// How many values a presence stream marks present, read straight off its payload.
/// A raw packed bitmap needs no decode pass and no budget of its own.
pub(crate) fn presence_popcount(stream: &RawStream<'_>) -> MltResult<u32> {
    let encoding = stream.meta.encoding;
    if encoding.logical != LogicalEncoding::Bool(BoolLogical::None) {
        return Err(MltError::UnsupportedLogicalEncoding(
            encoding.logical,
            "a nested presence stream, which is a raw packed bitmap",
        ));
    }
    if encoding.physical != PhysicalEncoding::None {
        return Err(MltError::UnsupportedPhysicalEncodingForType(
            encoding.physical,
            "a nested presence stream, which is a raw packed bitmap",
        ));
    }
    let count = stream.meta.num_values.into_usize();
    let bytes = stream
        .data
        .get(..count.div_ceil(8))
        .ok_or(MltError::UnableToTake(stream.meta.num_values))?;
    Ok(u32::try_from(
        bytes.view_bits::<Lsb0>()[..count].count_ones(),
    )?)
}

// ── Decoded form ──────────────────────────────────────────────────────────────

/// A decoded nested column: which features carry a value, and the tree they carry it in.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedNested<'a> {
    name: &'a str,
    /// One bit per feature, or [`None`] when every feature carries a value.
    presence: Option<Cow<'a, BitSlice<u8, Lsb0>>>,
    root: ParsedInterior<'a>,
}

/// A decoded node of a nested tree.
#[derive(Debug, Clone, PartialEq)]
pub enum ParsedNode<'a> {
    Interior(ParsedInterior<'a>),
    Leaf(ParsedLeaf<'a>),
}

/// A decoded interior node.
#[derive(Debug, Clone, PartialEq)]
pub enum ParsedInterior<'a> {
    Struct(ParsedStruct<'a>),
    List(ParsedList<'a>),
    Map(ParsedMap<'a>),
}

/// A decoded struct node.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedStruct<'a> {
    presence: Option<Vec<bool>>,
    fields: Vec<(&'a str, ParsedNode<'a>)>,
}

/// A decoded list node.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedList<'a> {
    presence: Option<Vec<bool>>,
    lengths: Vec<u32>,
    element: Box<ParsedNode<'a>>,
}

/// A decoded map node.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedMap<'a> {
    presence: Option<Vec<bool>>,
    lengths: Vec<u32>,
    keys: ParsedStrings<'a>,
    value: Box<ParsedNode<'a>>,
}

/// A decoded leaf node, holding one flat run of values.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedLeaf<'a> {
    presence: Option<Vec<bool>>,
    values: MValues<'a>,
}

impl<'a> Decode<ParsedNested<'a>> for RawNested<'a> {
    fn decode(self, dec: &mut Decoder) -> MltResult<ParsedNested<'a>> {
        let root = self.root.decode(dec)?;
        let parsed = ParsedNested {
            name: self.name,
            presence: self.presence.decode_bits(dec)?,
            root,
        };
        parsed.root.check_counts()?;
        let handed = parsed.root.parent_count();
        if handed != self.value_count.into_usize() {
            return Err(MltError::NestedRootCountMismatch {
                name: self.name.to_string(),
                expected: self.value_count,
                actual: u32::try_from(handed)?,
            });
        }
        Ok(parsed)
    }
}

impl<'a> Decode<ParsedNode<'a>> for RawNode<'a> {
    fn decode(self, dec: &mut Decoder) -> MltResult<ParsedNode<'a>> {
        Ok(match self {
            Self::Interior(interior) => ParsedNode::Interior(interior.decode(dec)?),
            Self::Leaf(leaf) => ParsedNode::Leaf(ParsedLeaf {
                presence: decode_node_presence(leaf.presence, dec)?,
                values: (*leaf.values).decode(dec)?,
            }),
        })
    }
}

impl<'a> Decode<ParsedInterior<'a>> for RawInterior<'a> {
    fn decode(self, dec: &mut Decoder) -> MltResult<ParsedInterior<'a>> {
        Ok(match self {
            Self::Struct(node) => {
                let presence = decode_node_presence(node.presence, dec)?;
                let mut fields = dec.alloc::<(&'a str, ParsedNode<'a>)>(node.fields.len())?;
                for (name, field) in node.fields {
                    fields.push((name, field.decode(dec)?));
                }
                ParsedInterior::Struct(ParsedStruct { presence, fields })
            }
            Self::List(node) => ParsedInterior::List(ParsedList {
                presence: decode_node_presence(node.presence, dec)?,
                lengths: node.lengths.decode_ints::<u32>(dec)?,
                element: Box::new(node.element.decode(dec)?),
            }),
            Self::Map(node) => ParsedInterior::Map(ParsedMap {
                presence: decode_node_presence(node.presence, dec)?,
                lengths: node.lengths.decode_ints::<u32>(dec)?,
                keys: node.keys.decode(dec)?,
                value: Box::new(node.value.decode(dec)?),
            }),
        })
    }
}

/// Decode a node's presence stream into one bool per value its parent handed it.
fn decode_node_presence(
    presence: Option<RawStream<'_>>,
    dec: &mut Decoder,
) -> MltResult<Option<Vec<bool>>> {
    presence.map(|stream| stream.decode_bools(dec)).transpose()
}

impl<'a> ParsedNested<'a> {
    #[must_use]
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// Whether feature `index` carries a value.
    #[must_use]
    pub fn is_present(&self, index: usize) -> bool {
        self.presence
            .as_deref()
            .is_none_or(|bits| bits.get(index).as_deref().copied().unwrap_or(false))
    }

    /// The shape every value of this column has, read back out of the tree.
    #[must_use]
    pub fn kind(&self) -> NestedKind {
        self.root.kind()
    }

    /// The value feature `index` carries, or the column's null when it carries none.
    pub fn row(&self, index: usize) -> MltResult<NestedValue> {
        if !self.is_present(index) {
            return Ok(self.root.kind().null_value());
        }
        let row = match self.presence.as_deref() {
            Some(bits) => bits[..index].count_ones(),
            None => index,
        };
        Ok(self
            .root
            .row(row)?
            .unwrap_or_else(|| self.root.kind().null_value()))
    }
}

impl ParsedInterior<'_> {
    /// How many values the parent of this node hands it.
    fn parent_count(&self) -> usize {
        match self {
            Self::Struct(node) => node
                .presence
                .as_ref()
                .map_or_else(|| node.present_count(), Vec::len),
            Self::List(node) => node.presence.as_ref().map_or(node.lengths.len(), Vec::len),
            Self::Map(node) => node.presence.as_ref().map_or(node.lengths.len(), Vec::len),
        }
    }

    /// Check every lengths stream against the node it counts, top down.
    fn check_counts(&self) -> MltResult<()> {
        match self {
            Self::Struct(node) => {
                let present = node.present_count();
                for (_, field) in &node.fields {
                    expect_count(present, field.parent_count())?;
                    field.check_counts()?;
                }
            }
            Self::List(node) => {
                let entries = sum_lengths(&node.lengths)?;
                expect_count(entries.into_usize(), node.element.parent_count())?;
                node.element.check_counts()?;
            }
            Self::Map(node) => {
                let entries = sum_lengths(&node.lengths)?;
                expect_count(entries.into_usize(), node.keys.feature_count())?;
                expect_count(entries.into_usize(), node.value.parent_count())?;
                node.value.check_counts()?;
            }
        }
        Ok(())
    }

    fn kind(&self) -> NestedKind {
        match self {
            Self::Struct(node) => NestedKind::Map(
                node.fields
                    .iter()
                    .map(|(name, field)| ((*name).to_string(), field.kind()))
                    .collect(),
            ),
            Self::List(node) => NestedKind::List(Box::new(node.element.kind())),
            // A map stores its keys per entry, so the shape it reads back as is
            // the keys it actually holds, all of one value type.
            Self::Map(node) => NestedKind::Map(
                (0..node.keys.feature_count())
                    .filter_map(|i| node.keys.get(u32::try_from(i).ok()?))
                    .map(|key| (key.to_string(), node.value.kind()))
                    .collect(),
            ),
        }
    }

    /// The value at `row` of this node, or [`None`] when the node marks it absent.
    fn row(&self, row: usize) -> MltResult<Option<NestedValue>> {
        match self {
            Self::Struct(node) => {
                let Some(own) = dense_index(node.presence.as_deref(), row) else {
                    return Ok(None);
                };
                let mut entries = BTreeMap::new();
                for (name, field) in &node.fields {
                    if let Some(value) = field.row(own)? {
                        entries.insert((*name).to_string(), value);
                    }
                }
                Ok(Some(NestedValue::Map(Some(entries))))
            }
            Self::List(node) => {
                let Some(own) = dense_index(node.presence.as_deref(), row) else {
                    return Ok(None);
                };
                let span = span_of(&node.lengths, own)?;
                let mut items = Vec::with_capacity(span.len());
                for entry in span {
                    items.push(
                        node.element
                            .row(entry)?
                            .unwrap_or_else(|| node.element.kind().null_value()),
                    );
                }
                Ok(Some(NestedValue::List(Some(items))))
            }
            Self::Map(node) => {
                let Some(own) = dense_index(node.presence.as_deref(), row) else {
                    return Ok(None);
                };
                let span = span_of(&node.lengths, own)?;
                let mut entries = BTreeMap::new();
                for entry in span {
                    let key = node.keys.get(u32::try_from(entry)?).ok_or(
                        MltError::NestedKeyOutOfRange {
                            entry,
                            len: node.keys.feature_count(),
                        },
                    )?;
                    if let Some(value) = node.value.row(entry)? {
                        entries.insert(key.to_string(), value);
                    }
                }
                Ok(Some(NestedValue::Map(Some(entries))))
            }
        }
    }
}

impl ParsedStruct<'_> {
    /// How many values this node marks present, which is what its fields are handed.
    fn present_count(&self) -> usize {
        match &self.presence {
            Some(bits) => bits.iter().filter(|&&bit| bit).count(),
            // With no mask of its own a struct is handed exactly what its fields hold.
            None => self.fields.first().map_or(0, |(_, f)| f.parent_count()),
        }
    }
}

impl ParsedNode<'_> {
    fn parent_count(&self) -> usize {
        match self {
            Self::Interior(interior) => interior.parent_count(),
            Self::Leaf(leaf) => leaf
                .presence
                .as_ref()
                .map_or_else(|| leaf.values.len(), Vec::len),
        }
    }

    fn check_counts(&self) -> MltResult<()> {
        match self {
            Self::Interior(interior) => interior.check_counts(),
            Self::Leaf(_) => Ok(()),
        }
    }

    fn kind(&self) -> NestedKind {
        match self {
            Self::Interior(interior) => interior.kind(),
            Self::Leaf(leaf) => NestedKind::Leaf(leaf.values.kind()),
        }
    }

    fn row(&self, row: usize) -> MltResult<Option<NestedValue>> {
        match self {
            Self::Interior(interior) => interior.row(row),
            Self::Leaf(leaf) => {
                let Some(own) = dense_index(leaf.presence.as_deref(), row) else {
                    return Ok(None);
                };
                let value = leaf.values.value(own)?;
                Ok(Some(NestedValue::Leaf(value)))
            }
        }
    }
}

/// Where `row` sits among the values a mask marks present, or [`None`] when it is absent.
fn dense_index(presence: Option<&[bool]>, row: usize) -> Option<usize> {
    match presence {
        None => Some(row),
        Some(bits) => bits
            .get(row)
            .copied()
            .unwrap_or(false)
            .then(|| bits[..row].iter().filter(|&&bit| bit).count()),
    }
}

/// The entries of value `row` of a lengths stream, as a range of entry indices.
fn span_of(lengths: &[u32], row: usize) -> MltResult<Range<usize>> {
    let start: usize = lengths[..row.min(lengths.len())]
        .iter()
        .map(|&len| len.into_usize())
        .sum();
    let len = lengths
        .get(row)
        .copied()
        .ok_or(MltError::NestedRowOutOfRange {
            row,
            len: lengths.len(),
        })?
        .into_usize();
    Ok(start..start + len)
}

fn sum_lengths(lengths: &[u32]) -> MltResult<u32> {
    lengths
        .iter()
        .try_fold(0_u32, |acc, &len| acc.checked_add(len))
        .ok_or(MltError::IntegerOverflow)
}

fn expect_count(expected: usize, actual: usize) -> MltResult<()> {
    if expected == actual {
        return Ok(());
    }
    Err(MltError::NestedCountMismatch {
        expected: u32::try_from(expected)?,
        actual: u32::try_from(actual)?,
    })
}

impl MValues<'_> {
    /// The value at `index`, as the row model holds it.
    pub(crate) fn value(&self, index: usize) -> MltResult<PropValue> {
        let missing = || MltError::NestedValueOutOfRange {
            index,
            len: self.len(),
        };
        /// One value out of a column's flat values.
        macro_rules! one {
            ($values:expr, $variant:ident) => {
                PropValue::$variant(Some(*$values.get(index).ok_or_else(missing)?))
            };
        }
        Ok(match self {
            Self::Bool(v) => one!(v, Bool),
            Self::I8(v) => one!(v, I8),
            Self::U8(v) => one!(v, U8),
            Self::I32(v) => one!(v, I32),
            Self::U32(v) => one!(v, U32),
            Self::I64(v) => one!(v, I64),
            Self::U64(v) => one!(v, U64),
            Self::F32(v) => one!(v, F32),
            Self::F64(v) => one!(v, F64),
            Self::Str(v) => {
                let value = v.get(u32::try_from(index)?).ok_or_else(missing)?;
                PropValue::Str(Some(value.to_string()))
            }
        })
    }
}
