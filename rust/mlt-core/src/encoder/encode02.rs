//! Layer envelope and column writers for tag `0x02` (v2) layers.
//!
//! A v2 layer body is: header (`name`, `extent`, `feature_count`, layout byte),
//! the layer's shared presence bitfields, geometry section, `column_count`
//! varint, then each counted column as
//! `[type byte][name?][presence bitfield?][data stream]` - metadata and data
//! merged, unlike v1's split sections.
//!
//! The type byte packs the presence nibble over the data type nibble, so
//! nullability is a property of the column rather than a separate `Opt` code.
//! Columns that are null on exactly the same features share one bitfield, stored
//! once at the layer root - see [`SharedPresence`].
//!
//! Stream payload encodings (and their size competitions) are shared with v1
//! via [`Codecs::write_int_stream`]; only the envelope, presence
//! representation, and stream headers differ.

use std::cmp::Reverse;
use std::collections::HashMap;

use integer_encoding::VarIntWriter as _;

use crate::decoder::stream::header02::{Count02, Family, StrLayout};
use crate::decoder::{
    BoolLogical, ColumnType02, DataType02, DictionaryType, LayerLayout, LengthType,
    LogicalEncoding, NodeKind02, NodePresence, NodeType02, PhysicalEncoding, Presence02,
    StreamMeta, StreamType, ValueType02,
};
use crate::encoder::geometry::encode02::encode_geometry02;
use crate::encoder::model::{StagedLayer, StrAt, StreamCtx};
use crate::encoder::nested_dict::{NestedDicts, SharedLeaf};
use crate::encoder::{
    Codecs, Encoder, StagedId, StagedInterior, StagedLeaf, StagedMValue, StagedNested, StagedNode,
    StagedOptScalar, StagedProperty, StagedSharedDictItem, StagedStrings, StagedValues,
    write_stream_payload,
};
use crate::utils::BinarySerializer as _;
use crate::{MltError, MltResult, OffsetType};

/// The presence masks the layer stores once for several columns to read.
///
/// Planned before any bytes are written, because the layout byte declares how
/// many masks there are and the block right after it holds them - both come
/// before the geometry section.
///
/// A mask only earns a slot when more than one column reads it; a mask read once is
/// no cheaper shared than inline. Each child of a shared dictionary counts as a
/// reader in its own right, since each writes its own bitfield otherwise. The layout
/// byte fits at most [`LayerLayout::MAX_SHARED_PRESENCE`] of them, so when more
/// groups qualify the most-read ones win, since every extra reader saves exactly
/// one copy of the same `ceil(feature_count/8)` bytes.
#[derive(Debug)]
pub(crate) struct SharedPresence {
    /// The masks in wire index order.
    masks: Vec<Vec<bool>>,
    /// Wire index of each mask, for resolving a column's nibble.
    index: HashMap<Vec<bool>, u8>,
}

impl SharedPresence {
    /// Group the layer's optional columns and dictionary children by mask and keep the shared ones.
    fn plan(
        id: &StagedId,
        properties: &[StagedProperty],
        nested: &[StagedNested],
        m_values: &[StagedMValue],
    ) -> Self {
        // (column count, index of the first column with this mask) per mask.
        let mut groups: HashMap<Vec<bool>, (usize, usize)> = HashMap::new();
        for (column, mask) in column_masks(id, properties, nested, m_values).enumerate() {
            let group = groups.entry(mask).or_insert((0, column));
            group.0 += 1;
        }

        let mut shared: Vec<(Vec<bool>, usize, usize)> = groups
            .into_iter()
            .filter(|&(_, (count, _))| count > 1)
            .map(|(mask, (count, first))| (mask, count, first))
            .collect();
        // Most-shared first, then first-seen, so the winners do not depend on
        // `HashMap` iteration order.
        shared.sort_unstable_by_key(|&(_, count, first)| (Reverse(count), first));
        shared.truncate(usize::from(LayerLayout::MAX_SHARED_PRESENCE));
        // Wire order is first-seen order, so the bitfields appear in the order
        // the columns that read them do.
        shared.sort_unstable_by_key(|&(_, _, first)| first);

        let masks: Vec<Vec<bool>> = shared.into_iter().map(|(mask, _, _)| mask).collect();
        let index = masks
            .iter()
            .enumerate()
            .map(|(i, mask)| {
                (
                    mask.clone(),
                    u8::try_from(i).expect("at most MAX_SHARED_PRESENCE masks"),
                )
            })
            .collect();
        Self { masks, index }
    }

    /// How many masks the layout byte declares.
    fn count(&self) -> u8 {
        u8::try_from(self.masks.len()).expect("at most MAX_SHARED_PRESENCE masks")
    }

    /// Where an optional column's nulls live: this layer's shared bitfield when
    /// another column has the same mask, the column's own bitfield otherwise.
    pub(crate) fn nibble_for(&self, mask: &[bool]) -> Presence02 {
        self.index
            .get(mask)
            .map_or(Presence02::Inline, |&i| Presence02::Shared(i))
    }

    /// Write the bitfields in index order, right after the layout byte.
    fn write_to(&self, enc: &mut Encoder) {
        for mask in &self.masks {
            write_presence_bits(enc.data_mut(), mask);
        }
    }
}

/// The presence mask of everything that writes one, in the order it is written:
/// the ID column first, then the properties, then the m-values, a shared
/// dictionary contributing one mask per child at its parent column's position.
///
/// Owned, because a string column derives its mask from its lengths rather than storing one.
/// Columns that cannot be null contribute nothing - there is no mask to share.
fn column_masks<'a>(
    id: &'a StagedId,
    properties: &'a [StagedProperty],
    nested: &'a [StagedNested],
    m_values: &'a [StagedMValue],
) -> impl Iterator<Item = Vec<bool>> + 'a {
    /// `presence` of an optional staged column.
    fn mask<T: Copy + PartialEq>(v: &StagedOptScalar<T>) -> Vec<bool> {
        v.presence.clone()
    }

    let id = match id {
        StagedId::OptU32(v) => Some(mask(v)),
        StagedId::OptU64(v) => Some(mask(v)),
        StagedId::None | StagedId::U32(_) | StagedId::U64(_) => None,
    };
    let props = properties.iter().flat_map(|prop| {
        use StagedProperty as D;
        match prop {
            D::OptBool(v) => vec![mask(v)],
            D::OptI8(v) => vec![mask(v)],
            D::OptU8(v) => vec![mask(v)],
            D::OptI32(v) => vec![mask(v)],
            D::OptU32(v) => vec![mask(v)],
            D::OptI64(v) => vec![mask(v)],
            D::OptU64(v) => vec![mask(v)],
            D::OptF32(v) => vec![mask(v)],
            D::OptF64(v) => vec![mask(v)],
            D::OptStr(v) => vec![v.presence_bools().collect()],
            // A shared dictionary holds no values of its own, but each of its
            // children is null on its own features, so each is a sharer in its
            // own right - listed here, where its parent column sits.
            D::SharedDict(v) => v
                .items
                .iter()
                .filter_map(StagedSharedDictItem::optional_presence)
                .collect(),
            // A column with no null mask has no presence bits.
            D::Bool(_)
            | D::I8(_)
            | D::U8(_)
            | D::I32(_)
            | D::U32(_)
            | D::I64(_)
            | D::U64(_)
            | D::F32(_)
            | D::F64(_)
            | D::Str(_) => vec![],
        }
    });
    // An m-value column is null on features, exactly as a property column is, so
    // the two share a bitfield whenever their masks agree.
    // A nested column's root is null on features exactly as a flat column is, so
    // its mask competes for a shared bitfield alongside them.
    let nested = nested.iter().filter_map(|n| n.presence().cloned());
    let m_values = m_values.iter().filter_map(|m| m.presence.clone());
    id.into_iter().chain(props).chain(nested).chain(m_values)
}

/// Append `bits` as `ceil(len/8)` LSB-first packed bytes - the layout v2 uses for
/// both presence bitfields and bool column data.
pub(crate) fn write_presence_bits(data: &mut Vec<u8>, bits: &[bool]) {
    let start = data.len();
    data.resize(start + bits.len().div_ceil(8), 0);
    for (i, &bit) in bits.iter().enumerate() {
        if bit {
            data[start + i / 8] |= 1 << (i % 8);
        }
    }
}

/// Encode and serialize a staged layer as a v2 (tag `0x02`) body into `enc`.
///
/// The v2 counterpart of the v1 `StagedLayer::encode_into` body; dispatched
/// from there based on [`EncoderConfig::wire_version`](crate::encoder::EncoderConfig::wire_version).
pub(crate) fn encode_into02(
    layer: StagedLayer,
    mut enc: Encoder,
    codecs: &mut Codecs,
) -> MltResult<Encoder> {
    let StagedLayer {
        name,
        extent,
        id,
        geometry,
        properties,
        m_values,
        nested,
    } = layer;

    let feature_count = u32::try_from(geometry.feature_count())?;
    enc.count_context = Count02::Implied(feature_count);

    // ── Layer layout byte + shared presence bitfields ─────────────────────
    let shared = SharedPresence::plan(&id, &properties, &nested, &m_values);
    let geometry = encode_geometry02(geometry)?;
    // The geometry layout is only settled once its vertex streams are written, so
    // the byte is reserved here and patched below.
    let layout_pos = enc.data().len();
    enc.data_mut().push(0);
    shared.write_to(&mut enc);

    // ── Geometry section (not part of column_count) ───────────────────────
    let geo_layout = geometry.write_to(&mut enc, codecs)?;
    // Only the geometry layout says whether a feature's vertex count can be read
    // back, which is the one thing an m-value column cannot do without.
    if !m_values.is_empty() && !geo_layout.allows_m_values() {
        return Err(MltError::MValuesNeedVertexCounts(geo_layout.into()));
    }
    enc.data_mut()[layout_pos] =
        LayerLayout::new(geo_layout, shared.count(), !m_values.is_empty()).to_byte();

    // ── Counted columns ───────────────────────────────────────────────────
    let column_count = usize::from(!matches!(id, StagedId::None)) + properties.len() + nested.len();
    enc.data_mut().write_varint(u32::try_from(column_count)?)?;

    write_id02(&id, &shared, &mut enc, codecs)?;
    for prop in &properties {
        write_prop02(prop, &shared, &mut enc, codecs)?;
    }
    for column in &nested {
        write_nested02(column, &shared, &mut enc, codecs)?;
    }

    // ── M-value section (not part of column_count either) ─────────────────
    if !m_values.is_empty() {
        enc.data_mut()
            .write_varint(u32::try_from(m_values.len())?)?;
        for m_value in &m_values {
            write_m_value02(m_value, &shared, &mut enc, codecs)?;
        }
    }

    enc.write_header02(&name, extent.get(), feature_count)?;
    Ok(enc)
}

/// Write a column's type byte and, for named columns, its name - inline in the
/// data section (v2 has no separate metadata section).
fn begin_col02(
    enc: &mut Encoder,
    presence: Presence02,
    typ: DataType02,
    name: Option<&str>,
) -> MltResult<()> {
    // Every v2 column starts here, so this is where its data stream's family is fixed.
    // A string column's writer re-fixes it per stream, once it has picked a layout.
    enc.family_context = family_of(typ);

    let data = enc.data_mut();
    data.push(ColumnType02::new(presence, typ).to_byte());
    debug_assert_eq!(typ.has_name(), name.is_some());
    if let Some(name) = name {
        data.write_string(name)?;
    }
    Ok(())
}

/// Write an optional column's header - type byte, name, and its own presence
/// bitfield unless the layer already stores that mask as a shared one - then run
/// `write_data` with [`Encoder::count_context`] set to the presence popcount, the
/// count a decoder infers for the optional column's data stream.
fn write_opt_col02<F>(
    enc: &mut Encoder,
    shared: &SharedPresence,
    typ: DataType02,
    name: Option<&str>,
    presence: &[bool],
    write_data: F,
) -> MltResult<()>
where
    F: FnOnce(&mut Encoder) -> MltResult<()>,
{
    let nibble = shared.nibble_for(presence);
    begin_col02(enc, nibble, typ, name)?;
    if nibble == Presence02::Inline {
        write_presence_bits(enc.data_mut(), presence);
    }

    let popcount = u32::try_from(presence.iter().filter(|&&p| p).count())?;
    let feature_count = enc.count_context;
    enc.count_context = Count02::Implied(popcount);
    let result = write_data(enc);
    enc.count_context = feature_count;
    result
}

/// Write a boolean data stream as a raw LSB-first packed bitfield - one bit per
/// value, `ceil(len/8)` bytes, framed as a `logical=None` / `physical=None`
/// stream. This mirrors how v2 presence bitfields are stored and is up to 8×
/// smaller than one byte per value; [`crate::decoder::RawStream::decode_bools`] reads it back
/// via the same bitmap unpacker as v1's byte-RLE bools.
fn write_bool_bitfield(enc: &mut Encoder, values: &[bool]) -> MltResult<()> {
    write_bool_stream02(enc, values, StreamType::Data(DictionaryType::None))
}

/// Write `values` as a raw LSB-first packed bitfield in the role `stream_type` names.
fn write_bool_stream02(
    enc: &mut Encoder,
    values: &[bool],
    stream_type: StreamType,
) -> MltResult<()> {
    let mut packed = Vec::with_capacity(values.len().div_ceil(8));
    write_presence_bits(&mut packed, values);
    let meta = StreamMeta::new2(
        stream_type,
        LogicalEncoding::Bool(BoolLogical::None),
        PhysicalEncoding::None,
        values.len(),
    )?;
    enc.family_context = Family::Bool;
    write_stream_payload(enc, meta, false, &packed)
}

/// The family a v2 column's data stream is numbered in.
/// A string column's leading stream is an integer one whose extension bits name the layout,
/// which only its writer knows.
fn family_of(typ: DataType02) -> Family {
    match typ {
        DataType02::Bool => Family::Bool,
        DataType02::F32 | DataType02::F64 => Family::Float,
        DataType02::Id
        | DataType02::LongId
        | DataType02::I8
        | DataType02::U8
        | DataType02::I32
        | DataType02::U32
        | DataType02::I64
        | DataType02::U64
        | DataType02::Str
        // A nested column's own type byte precedes no stream, so its family is
        // whatever the first node writer sets.
        | DataType02::Struct
        | DataType02::List
        | DataType02::Map => Family::Int,
    }
}

fn write_id02(
    id: &StagedId,
    shared: &SharedPresence,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    use DataType02 as DT;
    use Presence02::AllPresent;
    let ctx = StreamCtx::id(StreamType::Data(DictionaryType::None));
    match id {
        StagedId::None => Ok(()),
        StagedId::U32(v) => {
            begin_col02(enc, AllPresent, DT::Id, None)?;
            codecs.write_int_stream(&v.values, &ctx, enc)
        }
        StagedId::OptU32(v) => write_opt_col02(enc, shared, DT::Id, None, &v.presence, |enc| {
            codecs.write_int_stream(&v.values, &ctx, enc)
        }),
        StagedId::U64(v) => {
            begin_col02(enc, AllPresent, DT::LongId, None)?;
            codecs.write_int_stream(&v.values, &ctx, enc)
        }
        StagedId::OptU64(v) => write_opt_col02(enc, shared, DT::LongId, None, &v.presence, |enc| {
            codecs.write_int_stream(&v.values, &ctx, enc)
        }),
    }
}

/// Encode a single property column, dispatching on variant.
///
/// The v2 counterpart of the v1 `write_prop`: the column header goes inline
/// into the data section, presence is a raw bitfield, and bool data is written
/// as an ordinary 0/1 integer stream (racing raw vs RLE) instead of v1's
/// special bool-RLE bitset.
fn write_prop02(
    prop: &StagedProperty,
    shared: &SharedPresence,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    use DataType02 as DT;
    use Presence02::AllPresent;
    use StagedProperty as D;

    /// Non-optional scalar: `[type][name][data stream]`.
    macro_rules! scalar {
        ($ct:ident, $v:expr) => {{
            begin_col02(enc, AllPresent, DT::$ct, Some(&$v.name))?;
            codecs.write_int_stream(&$v.values, &StreamCtx::prop_data(&$v.name), enc)
        }};
    }
    /// Optional scalar: `[type][name][presence bitfield?][data stream]`.
    macro_rules! opt_scalar {
        ($ct:ident, $v:expr) => {{
            write_opt_col02(enc, shared, DT::$ct, Some(&$v.name), &$v.presence, |enc| {
                codecs.write_int_stream(&$v.values, &StreamCtx::prop_data(&$v.name), enc)
            })
        }};
    }
    /// Optional float, whose data stream is written by its own codec.
    macro_rules! opt_float {
        ($ct:ident, $v:expr) => {{
            write_opt_col02(enc, shared, DT::$ct, Some(&$v.name), &$v.presence, |enc| {
                codecs.write_float_stream(&$v.values, &StreamCtx::prop_data(&$v.name), enc)
            })
        }};
    }

    match prop {
        D::Bool(v) => {
            begin_col02(enc, AllPresent, DT::Bool, Some(&v.name))?;
            write_bool_bitfield(enc, &v.values)
        }
        D::OptBool(v) => {
            write_opt_col02(enc, shared, DT::Bool, Some(&v.name), &v.presence, |enc| {
                write_bool_bitfield(enc, &v.values)
            })
        }
        D::F32(v) => {
            begin_col02(enc, AllPresent, DT::F32, Some(&v.name))?;
            codecs.write_float_stream(&v.values, &StreamCtx::prop_data(&v.name), enc)
        }
        D::OptF32(v) => opt_float!(F32, v),
        D::F64(v) => {
            begin_col02(enc, AllPresent, DT::F64, Some(&v.name))?;
            codecs.write_float_stream(&v.values, &StreamCtx::prop_data(&v.name), enc)
        }
        D::OptF64(v) => opt_float!(F64, v),
        D::I8(v) => scalar!(I8, v),
        D::OptI8(v) => opt_scalar!(I8, v),
        D::U8(v) => scalar!(U8, v),
        D::OptU8(v) => opt_scalar!(U8, v),
        D::I32(v) => scalar!(I32, v),
        D::OptI32(v) => opt_scalar!(I32, v),
        D::U32(v) => scalar!(U32, v),
        D::OptU32(v) => opt_scalar!(U32, v),
        D::I64(v) => scalar!(I64, v),
        D::OptI64(v) => opt_scalar!(I64, v),
        D::U64(v) => scalar!(U64, v),
        D::OptU64(v) => opt_scalar!(U64, v),
        D::Str(v) => {
            begin_col02(enc, AllPresent, DT::Str, Some(&v.name))?;
            codecs.write_str_col02(v, StrAt::flat(&v.name), enc)
        }
        D::OptStr(v) => {
            let presence: Vec<bool> = v.presence_bools().collect();
            write_opt_col02(enc, shared, DT::Str, Some(&v.name), &presence, |enc| {
                codecs.write_str_col02(v, StrAt::flat(&v.name), enc)
            })
        }
        D::SharedDict(v) => codecs.write_shared_dict02(v, shared, enc),
    }
}

macro_rules! impl_m_value_type02 {
    (
        scalar { $($sv:ident),* $(,)? }
        string { $($gv:ident),* $(,)? }
    ) => {
        /// The type byte an m-value column of these values is written with.
        fn m_value_type02(values: &StagedValues) -> DataType02 {
            match values {
                $(StagedValues::$sv(_) => DataType02::$sv,)*
                $(StagedValues::$gv(_) => DataType02::$gv,)*
            }
        }
    };
}

with_kinds!(impl_m_value_type02);

/// Write one m-value column: `[type byte][name][presence bitfield?][data streams]`.
///
/// Its streams hold one value per vertex rather than per feature, a count only the
/// geometry knows, so the column is written with no count context at all and every
/// stream states its own.
fn write_m_value02(
    m_value: &StagedMValue,
    shared: &SharedPresence,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    use StagedValues as V;

    let name = m_value.name();
    let typ = m_value_type02(m_value.values());
    let nibble = match &m_value.presence {
        Some(mask) => shared.nibble_for(mask),
        None => Presence02::AllPresent,
    };
    begin_col02(enc, nibble, typ, Some(name))?;
    if let (Presence02::Inline, Some(mask)) = (nibble, &m_value.presence) {
        write_presence_bits(enc.data_mut(), mask);
    }

    let features = enc.count_context;
    // Only the geometry knows how many vertices an m-value column runs over, so
    // nothing lets a decoder infer its counts and each of its streams writes one.
    enc.count_context = Count02::Explicit;
    let ctx = StreamCtx::prop_data(name);
    let result = match m_value.values() {
        V::Bool(v) => write_bool_bitfield(enc, v),
        V::I8(v) => codecs.write_int_stream(v, &ctx, enc),
        V::U8(v) => codecs.write_int_stream(v, &ctx, enc),
        V::I32(v) => codecs.write_int_stream(v, &ctx, enc),
        V::U32(v) => codecs.write_int_stream(v, &ctx, enc),
        V::I64(v) => codecs.write_int_stream(v, &ctx, enc),
        V::U64(v) => codecs.write_int_stream(v, &ctx, enc),
        V::F32(v) => codecs.write_float_stream(v, &ctx, enc),
        V::F64(v) => codecs.write_float_stream(v, &ctx, enc),
        V::Str(v) => codecs.write_str_col02(&m_value.strings(v), StrAt::flat(name), enc),
    };
    // Restore what the columns after this one imply their counts from.
    enc.count_context = features;
    result
}

macro_rules! impl_value_type02 {
    (
        scalar { $($sv:ident),* $(,)? }
        string { $($gv:ident),* $(,)? }
    ) => {
        /// The value type a leaf of these values is written with.
        fn value_type02(values: &StagedValues) -> ValueType02 {
            match values {
                $(StagedValues::$sv(_) => ValueType02::$sv,)*
                $(StagedValues::$gv(_) => ValueType02::$gv,)*
            }
        }
    };
}

with_kinds!(impl_value_type02);

/// Write one nested column, keeping whichever of its wire shapes is smaller.
///
/// A struct whose fields all hold one type says the same thing as a map, so both
/// are written and the shorter one is kept, exactly as a string column's layouts race.
fn write_nested02(
    column: &StagedNested,
    shared: &SharedPresence,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    let name = column.name();
    let planned = NestedDicts::plan(std::slice::from_ref(column));
    let alone = NestedDicts::default();
    let as_map = column.root.as_map();

    let mut alt = enc.try_alternatives();
    for root in [Some(&column.root), as_map.as_ref()].into_iter().flatten() {
        alt.with(|enc| write_nested_root02(name, root, shared, enc, codecs, &alone))?;
        // A shape that indexes no corpus writes what the plain candidate already wrote.
        let dicts = planned.restrict(name, root);
        if !dicts.is_empty() {
            alt.with(|enc| write_nested_root02(name, root, shared, enc, codecs, &dicts))?;
        }
    }
    Ok(())
}

/// Write a nested column's root: `[type byte][name][presence bitfield?][body]`.
///
/// The root reads the layer's presence nibble rather than a node type byte, so it
/// may share a bitfield with any other column.
fn write_nested_root02(
    name: &str,
    root: &StagedInterior,
    shared: &SharedPresence,
    enc: &mut Encoder,
    codecs: &mut Codecs,
    dicts: &NestedDicts,
) -> MltResult<()> {
    let presence = root.presence();
    let nibble = presence.map_or(Presence02::AllPresent, |mask| shared.nibble_for(mask));
    begin_col02(enc, nibble, interior_type02(root), Some(name))?;
    if let (Presence02::Inline, Some(mask)) = (nibble, presence) {
        write_presence_bits(enc.data_mut(), mask);
    }

    let outer = enc.count_context;
    let values = match presence {
        Some(mask) => u32::try_from(mask.iter().filter(|&&bit| bit).count())?,
        None => match outer {
            Count02::Implied(count) => count,
            Count02::Explicit => u32::try_from(root.parent_count())?,
        },
    };
    enc.count_context = body_context02(root, Count02::Implied(values));
    let result = write_interior02(root, name, "", enc, codecs, dicts);
    enc.count_context = outer;
    result?;

    // The corpora this column's leaves index, so the shape race costs each candidate whole.
    // They trail the body, which is what lets a column that shares nothing write the bytes
    // it wrote before this existed: only a shared leaf says the section is there at all.
    if !dicts.is_empty() {
        enc.count_context = Count02::Explicit;
        enc.data_mut()
            .write_varint(u32::try_from(dicts.corpora.len())?)?;
        for corpus in &dicts.corpora {
            codecs.write_nested_corpus02(corpus, enc)?;
        }
        enc.count_context = outer;
    }
    Ok(())
}

/// What a node's body is counted against: nothing, once a lengths stream sits on
/// the path from the root, and its own present count until then.
fn body_context02(interior: &StagedInterior, present: Count02) -> Count02 {
    match interior {
        StagedInterior::Struct(_) => present,
        StagedInterior::List(_) | StagedInterior::Map(_) => Count02::Explicit,
    }
}

fn interior_type02(interior: &StagedInterior) -> DataType02 {
    match interior {
        StagedInterior::Struct(_) => DataType02::Struct,
        StagedInterior::List(_) => DataType02::List,
        StagedInterior::Map(_) => DataType02::Map,
    }
}

fn node_kind02(node: &StagedNode) -> NodeKind02 {
    match node {
        StagedNode::Interior(StagedInterior::Struct(_)) => NodeKind02::Struct,
        StagedNode::Interior(StagedInterior::List(_)) => NodeKind02::List,
        StagedNode::Interior(StagedInterior::Map(_)) => NodeKind02::Map,
        StagedNode::Leaf(leaf) => NodeKind02::Leaf(value_type02(&leaf.values)),
    }
}

/// Write one node below the root, keeping whichever of its wire shapes is smaller.
///
/// A struct one level down races a map exactly as the root does, since the two say
/// the same thing wherever the struct holds leaves of one type.
fn write_node02(
    node: &StagedNode,
    column: &str,
    path: &str,
    field: Option<&str>,
    enc: &mut Encoder,
    codecs: &mut Codecs,
    dicts: &NestedDicts,
) -> MltResult<()> {
    let as_map = match node {
        StagedNode::Interior(interior) => interior.as_map().map(StagedNode::Interior),
        StagedNode::Leaf(_) => None,
    };
    let Some(as_map) = as_map else {
        return write_node_shape02(node, column, path, field, enc, codecs, dicts);
    };
    let mut alt = enc.try_alternatives();
    alt.with(|enc| write_node_shape02(node, column, path, field, enc, codecs, dicts))?;
    alt.with(|enc| write_node_shape02(&as_map, column, path, field, enc, codecs, dicts))?;
    Ok(())
}

/// Write one node below the root: `[type byte][name?][presence stream?][body]`.
///
/// `field` is the name a struct field carries and is absent for a list element or
/// a map value. `path` is where the node sits inside `column`, which is what an
/// explicit encoder pins a stream's encoding by.
fn write_node_shape02(
    node: &StagedNode,
    column: &str,
    path: &str,
    field: Option<&str>,
    enc: &mut Encoder,
    codecs: &mut Codecs,
    dicts: &NestedDicts,
) -> MltResult<()> {
    let presence = match node {
        StagedNode::Interior(interior) => interior.presence(),
        StagedNode::Leaf(leaf) => leaf.presence.as_ref(),
    };
    // A string leaf the plan holds writes its codes into a corpus column instead of a
    // dictionary of its own, which the two shared presence nibbles say on the type byte.
    let link =
        matches!(node, StagedNode::Leaf(leaf) if matches!(leaf.values, StagedValues::Str(_)))
            .then(|| dicts.get(column, path))
            .flatten();
    let node_presence = match (link.is_some(), presence.is_some()) {
        (false, false) => NodePresence::AllPresent,
        (false, true) => NodePresence::Stream,
        (true, false) => NodePresence::SharedAllPresent,
        (true, true) => NodePresence::SharedStream,
    };
    let kind = node_kind02(node);
    enc.data_mut()
        .push(NodeType02::new(node_presence, kind).to_byte());
    if let Some(field) = field {
        enc.data_mut().write_string(field)?;
    }

    // A list or a map ends the implied counts, its own streams included.
    let parent = enc.count_context;
    let node_count = match kind {
        NodeKind02::List | NodeKind02::Map => Count02::Explicit,
        NodeKind02::Struct | NodeKind02::Leaf(_) => parent,
    };
    if let Some(mask) = presence {
        enc.count_context = node_count;
        write_bool_stream02(enc, mask, StreamType::Present)?;
    }
    enc.count_context = match node_count {
        Count02::Explicit => Count02::Explicit,
        Count02::Implied(_) => Count02::Implied(u32::try_from(node.present_count())?),
    };
    let result = match (link, node) {
        (Some(link), _) => write_shared_leaf02(link, column, path, enc, codecs),
        (None, StagedNode::Interior(interior)) => {
            enc.count_context = body_context02(interior, enc.count_context);
            write_interior02(interior, column, path, enc, codecs, dicts)
        }
        (None, StagedNode::Leaf(leaf)) => write_leaf02(leaf, column, path, enc, codecs),
    };
    enc.count_context = parent;
    result
}

/// Write the body of an interior node, whose presence the caller has already written.
fn write_interior02(
    interior: &StagedInterior,
    column: &str,
    path: &str,
    enc: &mut Encoder,
    codecs: &mut Codecs,
    dicts: &NestedDicts,
) -> MltResult<()> {
    match interior {
        StagedInterior::Struct(node) => {
            if node.fields.is_empty() {
                return Err(MltError::EmptyStructNode);
            }
            enc.data_mut()
                .write_varint(u32::try_from(node.fields.len())?)?;
            let fields = enc.count_context;
            for (name, field) in &node.fields {
                enc.count_context = fields;
                let path = format!("{path}.{name}");
                write_node02(field, column, &path, Some(name), enc, codecs, dicts)?;
            }
            enc.count_context = fields;
            Ok(())
        }
        StagedInterior::List(node) => {
            write_lengths02(&node.lengths, column, path, enc, codecs)?;
            write_node02(
                &node.element,
                column,
                &format!("{path}[]"),
                None,
                enc,
                codecs,
                dicts,
            )
        }
        StagedInterior::Map(node) => {
            write_lengths02(&node.lengths, column, path, enc, codecs)?;
            let keys = StagedStrings::from_strings(column, &node.keys);
            codecs.write_str_col02(&keys, StrAt::nested(column, path), enc)?;
            write_node02(
                &node.value,
                column,
                &format!("{path}{{}}"),
                None,
                enc,
                codecs,
                dicts,
            )
        }
    }
}

/// Write a list or map node's lengths, one per value the node marks present.
fn write_lengths02(
    lengths: &[u32],
    column: &str,
    path: &str,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    enc.family_context = Family::Int;
    let ctx = StreamCtx::prop2(StreamType::Length(LengthType::Nested), column, path);
    codecs.write_int_stream(lengths, &ctx, enc)
}

/// Write a shared leaf's body: which corpus column its codes index, then the codes.
fn write_shared_leaf02(
    link: &SharedLeaf,
    column: &str,
    path: &str,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    enc.data_mut().write_varint(link.corpus)?;
    let ctx = StreamCtx::prop2(StreamType::Offset(OffsetType::String), column, path);
    enc.family_context = Family::Str(StrLayout::Dict);
    let result = codecs.write_int_stream(&link.codes, &ctx, enc);
    enc.family_context = Family::Int;
    result
}

/// Write a leaf's data streams, which are the ones a column of its type holds.
fn write_leaf02(
    leaf: &StagedLeaf,
    column: &str,
    path: &str,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    use StagedValues as V;

    let ctx = StreamCtx::prop2(StreamType::Data(DictionaryType::None), column, path);
    enc.family_context = family_of(value_type02(&leaf.values).into());
    match &leaf.values {
        V::Bool(v) => write_bool_bitfield(enc, v),
        V::I8(v) => codecs.write_int_stream(v, &ctx, enc),
        V::U8(v) => codecs.write_int_stream(v, &ctx, enc),
        V::I32(v) => codecs.write_int_stream(v, &ctx, enc),
        V::U32(v) => codecs.write_int_stream(v, &ctx, enc),
        V::I64(v) => codecs.write_int_stream(v, &ctx, enc),
        V::U64(v) => codecs.write_int_stream(v, &ctx, enc),
        V::F32(v) => codecs.write_float_stream(v, &ctx, enc),
        V::F64(v) => codecs.write_float_stream(v, &ctx, enc),
        V::Str(v) => codecs.write_str_col02(
            &StagedStrings::from_strings(column, v),
            StrAt::nested(column, path),
            enc,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoder::GeometryValues;
    use crate::encoder::{EncoderConfig, StagedStruct, WireVersion};
    use crate::geo_types::{Geometry, Point};

    fn points(count: usize) -> GeometryValues {
        let mut geometry = GeometryValues::default();
        for _ in 0..count {
            geometry.push_geom(&Geometry::<i32>::Point(Point::new(0, 0)));
        }
        geometry
    }

    fn str_leaf(presence: Option<Vec<bool>>, values: &[&str]) -> StagedNode {
        let values = values.iter().map(|v| (*v).to_string()).collect();
        StagedNode::Leaf(StagedLeaf::new(presence, StagedValues::Str(values)))
    }

    /// Wrap `inner` in a root struct that holds nothing else, and encode it over `rows` features.
    fn encode_one_field(inner: StagedNode, rows: usize) -> Vec<u8> {
        let root = StagedInterior::Struct(StagedStruct::new(None, vec![("inner", inner)]));
        StagedLayer::with_nested(
            "layer",
            4096,
            StagedId::None,
            points(rows),
            Vec::new(),
            Vec::new(),
            vec![StagedNested::new("n", root)],
        )
        .expect("staged layer")
        .encode_into(
            Encoder::new(EncoderConfig::default().with_wire_version(WireVersion::V02)),
            &mut Codecs::default(),
        )
        .expect("encode")
        .into_layer_bytes()
        .expect("layer bytes")
    }

    /// The node type byte of the field named `field`, which the wire puts right before its name.
    fn field_type_byte(bytes: &[u8], field: &str) -> u8 {
        let mut name = vec![u8::try_from(field.len()).expect("a short name")];
        name.extend_from_slice(field.as_bytes());
        let at = bytes
            .windows(name.len())
            .position(|window| window == name.as_slice())
            .expect("the field name");
        bytes[at - 1]
    }

    #[test]
    fn an_interior_struct_of_sparse_keys_is_written_as_a_map() {
        let rows = 12;
        let fields: Vec<(String, StagedNode)> = (0..rows)
            .map(|row| {
                let mut presence = vec![false; rows];
                presence[row] = true;
                (format!("key{row}"), str_leaf(Some(presence), &["v"]))
            })
            .collect();
        let inner = StagedNode::Interior(StagedInterior::Struct(StagedStruct::new(None, fields)));
        assert_eq!(
            field_type_byte(&encode_one_field(inner, rows), "inner"),
            NodeType02::new(NodePresence::AllPresent, NodeKind02::Map).to_byte()
        );
    }

    #[test]
    fn an_interior_struct_of_dense_keys_stays_a_struct() {
        let rows = 12;
        let inner = StagedNode::Interior(StagedInterior::Struct(StagedStruct::new(
            None,
            vec![
                ("alpha", str_leaf(None, &["a"; 12])),
                ("beta", str_leaf(None, &["b"; 12])),
            ],
        )));
        assert_eq!(
            field_type_byte(&encode_one_field(inner, rows), "inner"),
            NodeType02::new(NodePresence::AllPresent, NodeKind02::Struct).to_byte()
        );
    }
}
