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

use crate::decoder::{
    ColumnType02, DataType02, DictionaryType, LayerLayout, LogicalEncoding, PhysicalEncoding,
    Presence02, StreamMeta, StreamType,
};
use crate::encoder::geometry::encode02::encode_geometry02;
use crate::encoder::model::{StagedLayer, StreamCtx};
use crate::encoder::{
    Codecs, Encoder, StagedId, StagedOptScalar, StagedProperty, write_stream_payload,
};
use crate::utils::BinarySerializer as _;
use crate::{MltError, MltResult};

/// The presence masks the layer stores once for several columns to read.
///
/// Planned before any bytes are written, because the layout byte declares how
/// many masks there are and the block right after it holds them - both come
/// before the geometry section.
///
/// A mask only earns a slot when more than one column has it; a mask used once is
/// no cheaper shared than inline. The layout byte fits at most
/// [`LayerLayout::MAX_SHARED_PRESENCE`] of them, so when more groups qualify the
/// ones shared by the most columns win, since every extra sharer saves exactly
/// one copy of the same `ceil(feature_count/8)` bytes.
#[derive(Debug)]
struct SharedPresence<'a> {
    /// The masks in wire index order.
    masks: Vec<&'a [bool]>,
    /// Wire index of each mask, for resolving a column's nibble.
    index: HashMap<&'a [bool], u8>,
}

impl<'a> SharedPresence<'a> {
    /// Group the layer's optional columns by mask and keep the shared ones.
    fn plan(id: &'a StagedId, properties: &'a [StagedProperty]) -> Self {
        // (column count, index of the first column with this mask) per mask.
        let mut groups: HashMap<&'a [bool], (usize, usize)> = HashMap::new();
        for (column, mask) in column_masks(id, properties).enumerate() {
            let group = groups.entry(mask).or_insert((0, column));
            group.0 += 1;
        }

        let mut shared: Vec<(&'a [bool], usize, usize)> = groups
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

        let masks: Vec<&'a [bool]> = shared.into_iter().map(|(mask, _, _)| mask).collect();
        let index = masks
            .iter()
            .enumerate()
            .map(|(i, &mask)| {
                (
                    mask,
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
    fn nibble_for(&self, mask: &[bool]) -> Presence02 {
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

/// The presence mask of every optional column, in the order the columns are
/// written: the ID column first, then the properties.
///
/// Columns that cannot be null contribute nothing - there is no mask to share.
fn column_masks<'a>(
    id: &'a StagedId,
    properties: &'a [StagedProperty],
) -> impl Iterator<Item = &'a [bool]> {
    /// `presence` of an optional staged column, as a slice.
    fn mask<T: Copy + PartialEq>(v: &StagedOptScalar<T>) -> &[bool] {
        &v.presence
    }

    let id = match id {
        StagedId::OptU32(v) => Some(mask(v)),
        StagedId::OptU64(v) => Some(mask(v)),
        StagedId::None | StagedId::U32(_) | StagedId::U64(_) => None,
    };
    let props = properties.iter().filter_map(|prop| {
        use StagedProperty as D;
        match prop {
            D::OptBool(v) => Some(mask(v)),
            D::OptI8(v) => Some(mask(v)),
            D::OptU8(v) => Some(mask(v)),
            D::OptI32(v) => Some(mask(v)),
            D::OptU32(v) => Some(mask(v)),
            D::OptI64(v) => Some(mask(v)),
            D::OptU64(v) => Some(mask(v)),
            D::OptF32(v) => Some(mask(v)),
            D::OptF64(v) => Some(mask(v)),
            // Strings and shared dictionaries are not encodable as v2 yet.
            _ => None,
        }
    });
    id.into_iter().chain(props)
}

/// Append `bits` as `ceil(len/8)` LSB-first packed bytes - the layout v2 uses for
/// both presence bitfields and bool column data.
fn write_presence_bits(data: &mut Vec<u8>, bits: &[bool]) {
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
    enc.data_mut()
        .push(LayerLayout::new(geometry.layout, shared.count()).to_byte());
    shared.write_to(&mut enc);

    // ── Geometry section (not part of column_count) ───────────────────────
    geometry.write_to(&mut enc, codecs)?;

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
/// implicit count of the optional column's data stream.
fn write_opt_col02<F>(
    enc: &mut Encoder,
    shared: &SharedPresence<'_>,
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
        LogicalEncoding::None,
        PhysicalEncoding::None,
        values.len(),
    )?;
    write_stream_payload(enc, meta, false, &packed)
}

fn write_id02(
    id: &StagedId,
    shared: &SharedPresence<'_>,
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
    shared: &SharedPresence<'_>,
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
                codecs.write_float_stream(&$v.values, StreamType::Data(DictionaryType::None), enc)
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
            codecs.write_float_stream(&v.values, StreamType::Data(DictionaryType::None), enc)
        }
        D::OptF32(v) => opt_float!(F32, v),
        D::F64(v) => {
            begin_col02(enc, AllPresent, DT::F64, Some(&v.name))?;
            codecs.write_float_stream(&v.values, StreamType::Data(DictionaryType::None), enc)
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
        D::Str(_) | D::OptStr(_) => Err(MltError::NotImplemented("v2 string columns")),
        D::SharedDict(_) => Err(MltError::NotImplemented("v2 shared dictionary columns")),
    }
}
