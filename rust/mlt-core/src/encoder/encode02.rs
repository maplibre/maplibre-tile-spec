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

use crate::MltResult;
use crate::decoder::stream::header02::Family;
use crate::decoder::{
    BoolLogical, ColumnType02, DataType02, DictionaryType, LayerLayout, LogicalEncoding,
    PhysicalEncoding, Presence02, StreamMeta, StreamType,
};
use crate::encoder::geometry::encode02::encode_geometry02;
use crate::encoder::model::{StagedLayer, StreamCtx};
use crate::encoder::{
    Codecs, Encoder, StagedId, StagedOptScalar, StagedProperty, StagedSharedDictItem,
    write_stream_payload,
};

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
    fn plan(id: &StagedId, properties: &[StagedProperty]) -> Self {
        // (column count, index of the first column with this mask) per mask.
        let mut groups: HashMap<Vec<bool>, (usize, usize)> = HashMap::new();
        for (column, mask) in column_masks(id, properties).enumerate() {
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
/// the ID column first, then the properties, a shared dictionary contributing one
/// mask per child at its parent column's position.
///
/// Owned, because a string column derives its mask from its lengths rather than storing one.
/// Columns that cannot be null contribute nothing - there is no mask to share.
fn column_masks<'a>(
    id: &'a StagedId,
    properties: &'a [StagedProperty],
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
    id.into_iter().chain(props)
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
    } = layer;

    let feature_count = u32::try_from(geometry.feature_count())?;
    enc.count_context = feature_count;

    // ── Layer layout byte + shared presence bitfields ─────────────────────
    let shared = SharedPresence::plan(&id, &properties);
    let geometry = encode_geometry02(geometry)?;
    // The geometry layout is only settled once its vertex streams are written, so
    // the byte is reserved here and patched below.
    let layout_pos = enc.data().len();
    enc.data_mut().push(0);
    shared.write_to(&mut enc);

    // ── Geometry section (not part of column_count) ───────────────────────
    let geo_layout = geometry.write_to(&mut enc, codecs)?;
    enc.data_mut()[layout_pos] = LayerLayout::new(geo_layout, shared.count()).to_byte();

    // ── Counted columns ───────────────────────────────────────────────────
    let column_count = usize::from(!matches!(id, StagedId::None)) + properties.len();
    enc.data_mut().write_varint(u32::try_from(column_count)?)?;

    write_id02(&id, &shared, &mut enc, codecs)?;
    for prop in &properties {
        write_prop02(prop, &shared, &mut enc, codecs)?;
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

    enc.data_mut()
        .push(ColumnType02::new(presence, typ).to_byte());
    debug_assert_eq!(typ.has_name(), name.is_some());
    if let Some(name) = name {
        enc.write_data_name(name)?;
    }
    Ok(())
}

/// Write an optional column's header - type byte, name, and its own presence
/// bitfield unless the layer already stores that mask as a shared one - then run
/// `write_data` with [`Encoder::count_context`] set to the presence popcount, the
/// implicit count of the optional column's data stream.
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
    enc.count_context = popcount;
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
    let mut packed = Vec::with_capacity(values.len().div_ceil(8));
    write_presence_bits(&mut packed, values);
    let meta = StreamMeta::new2(
        StreamType::Data(DictionaryType::None),
        LogicalEncoding::Bool(BoolLogical::None),
        PhysicalEncoding::None,
        values.len(),
    )?;
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
        // A shared dictionary has no data stream of its own; each of its streams sets its own family.
        | DataType02::SharedDict => Family::Int,
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
            codecs.write_str_col02(v, enc)
        }
        D::OptStr(v) => {
            let presence: Vec<bool> = v.presence_bools().collect();
            write_opt_col02(enc, shared, DT::Str, Some(&v.name), &presence, |enc| {
                codecs.write_str_col02(v, enc)
            })
        }
        D::SharedDict(v) => codecs.write_shared_dict02(v, shared, enc),
    }
}
