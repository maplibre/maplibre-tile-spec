//! Parser for tag `0x02` (v2) layer bodies.
//!
//! Produces the same in-memory representation as the v1 parser - a lazy
//! [`Layer01`] over `Raw*` column containers - by synthesizing per-stream
//! metadata (stream role, value count) from the envelope context instead of
//! reading it from the wire. All downstream decoding is shared with v1.
//!
//! A v2 layer body is laid out as:
//!
//! ```text
//! [varint name_len] [name bytes]
//! [varint extent]
//! [varint feature_count]
//! [u8 layer_layout]                 reserved | shared presence count | geometry layout, see LayerLayout
//! [shared presence bitfields]       ceil(feature_count/8) raw bytes each,
//!                                   one per shared presence count
//! ── geometry section ─────────────────────────────────
//! [types stream]                    count = feature_count
//! [length streams per layout]       explicit counts
//! [vertex stream]                   explicit count
//! ── counted columns ──────────────────────────────────
//! [varint column_count]             ids + scalars only (geometry excluded)
//! per column:
//!   [u8 column_type]                presence nibble | data type nibble,
//!                                   see ColumnType02
//!   [varint name_len] [name]        only when data type has_name()
//!   [presence bitfield]             ceil(feature_count/8) raw bytes, only when
//!                                   the presence nibble is Inline; a Shared
//!                                   nibble reads one of the layer's instead
//!   [data stream]                   count = feature_count or presence popcount
//! ```

use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;
use bitvec::view::BitView as _;
use usize_cast::IntoUsize as _;

use crate::LazyParsed::Raw;
use crate::MltError::{BufferUnderflow, MissingLayerName, TrailingLayerData};
use crate::codecs::varint::parse_varint;
use crate::decoder::stream::header02;
use crate::decoder::stream::header02::StreamCtx02;
use crate::decoder::{
    ColumnType02, DataType02, GeoLayout, Id, Layer01, LayerLayout, LengthType, Presence02,
    RawFloats, RawGeometry, RawId, RawIdValue, RawPresence, RawScalar,
};
use crate::tile::Extent;
use crate::utils::{SetOptionOnce as _, parse_string, parse_u8, take};
use crate::{Lazy, MltError, MltRefResult, MltResult, Parser};

/// Parse a v2 layer body (the bytes after the `tag = 2` byte).
pub(crate) fn parse_layer02<'a>(
    input: &'a [u8],
    parser: &mut Parser,
) -> MltResult<Layer01<'a, Lazy>> {
    let (input, layer_name) = parse_string(input)?;
    if layer_name.is_empty() {
        return Err(MissingLayerName);
    }
    let (input, extent) = parse_varint::<u32>(input)?;
    let extent = Extent::new(extent)?;
    let (input, feature_count) = parse_varint::<u32>(input)?;
    let (input, layout_byte) = parse_u8(input)?;
    let layout = LayerLayout::parse(layout_byte)?;

    // ── Shared presence bitfields ─────────────────────────────────────────
    let (input, shared_presence) = parse_shared_presence(input, layout, feature_count)?;

    // ── Geometry section ──────────────────────────────────────────────────
    let (input, geometry) = parse_geometry(input, layout.geometry, feature_count, parser)?;

    // ── Counted columns ───────────────────────────────────────────────────
    let (mut input, column_count) = parse_varint::<u32>(input)?;
    // Each column requires at least 1 byte (column type).
    if input.len() < column_count.into_usize() {
        return Err(BufferUnderflow(column_count, input.len()));
    }

    let mut id_column: Option<Id> = None;
    let mut properties = Vec::with_capacity(column_count.into_usize());
    #[cfg(fuzzing)]
    let mut layer_order = vec![crate::decoder::fuzzing::LayerOrdering::Geometry];

    for _ in 0..column_count {
        use crate::decoder::RawProperty as RP;

        let typ_byte;
        (input, typ_byte) = parse_u8(input)?;
        let typ = ColumnType02::parse(typ_byte, layout.shared_presence)?;
        let name = if typ.data.has_name() {
            let named;
            (input, named) = parse_string(input)?;
            named
        } else {
            ""
        };
        let presence;
        (input, presence) = parse_presence(typ, &shared_presence, input, feature_count)?;
        // The count context for this column's data stream: all features, or
        // only the present ones when a presence bitfield precedes the data.
        let data_count = match &presence {
            RawPresence::Bitfield(bits) => u32::try_from(bits.count_ones())?,
            RawPresence::AllPresent | RawPresence::Stream(_) => feature_count,
        };
        let ctx = StreamCtx02::Property(typ.data);
        let value;
        (input, value) = header02::parse_stream(input, ctx, data_count, parser)?;

        #[cfg(fuzzing)]
        layer_order.push(match typ.data {
            DataType02::Id | DataType02::LongId => crate::decoder::fuzzing::LayerOrdering::Id,
            _ => crate::decoder::fuzzing::LayerOrdering::Property,
        });

        let prop = match typ.data {
            DataType02::Id => {
                id_column.set_once(Raw(RawId {
                    presence,
                    value: RawIdValue::Id32(value),
                }))?;
                continue;
            }
            DataType02::LongId => {
                id_column.set_once(Raw(RawId {
                    presence,
                    value: RawIdValue::Id64(value),
                }))?;
                continue;
            }
            DataType02::Bool => RP::Bool(RawScalar::new(name, presence, value)),
            DataType02::I8 => RP::I8(RawScalar::new(name, presence, value)),
            DataType02::U8 => RP::U8(RawScalar::new(name, presence, value)),
            DataType02::I32 => RP::I32(RawScalar::new(name, presence, value)),
            DataType02::U32 => RP::U32(RawScalar::new(name, presence, value)),
            DataType02::I64 => RP::I64(RawScalar::new(name, presence, value)),
            DataType02::U64 => RP::U64(RawScalar::new(name, presence, value)),
            DataType02::F32 => RP::F32(RawFloats::single(name, presence, value)),
            DataType02::F64 => RP::F64(RawFloats::single(name, presence, value)),
        };
        properties.push(Raw(prop));
    }

    if !input.is_empty() {
        return Err(TrailingLayerData(input.len()));
    }
    Ok(Layer01 {
        name: layer_name,
        extent,
        id: id_column,
        geometry: Raw(geometry),
        properties,
        #[cfg(fuzzing)]
        layer_order,
    })
}

/// Parse the layer's shared presence bitfields: `shared_presence` back-to-back
/// bitfields of `ceil(feature_count/8)` raw packed bytes each.
///
/// Columns point into the returned slice by index; the layout byte caps the count
/// at [`LayerLayout::MAX_SHARED_PRESENCE`], so this allocates nothing worth
/// charging to the parser's budget.
fn parse_shared_presence(
    input: &[u8],
    layout: LayerLayout,
    feature_count: u32,
) -> MltRefResult<'_, Vec<&BitSlice<u8, Lsb0>>> {
    let mut input = input;
    let mut bitfields = Vec::with_capacity(usize::from(layout.shared_presence));
    for _ in 0..layout.shared_presence {
        let bits;
        (input, bits) = parse_bitfield(input, feature_count)?;
        bitfields.push(bits);
    }
    Ok((input, bitfields))
}

/// Parse one presence bitfield: `ceil(feature_count/8)` raw packed bytes,
/// borrowed zero-copy from the tile.
fn parse_bitfield(input: &[u8], feature_count: u32) -> MltRefResult<'_, &BitSlice<u8, Lsb0>> {
    let (input, bytes) = take(input, feature_count.div_ceil(8))?;
    Ok((
        input,
        &bytes.view_bits::<Lsb0>()[..feature_count.into_usize()],
    ))
}

/// Resolve a column's presence nibble into the bits that describe its nulls,
/// consuming the column's own bitfield only when it has one.
fn parse_presence<'a>(
    typ: ColumnType02,
    shared: &[&'a BitSlice<u8, Lsb0>],
    input: &'a [u8],
    feature_count: u32,
) -> MltRefResult<'a, RawPresence<'a>> {
    match typ.presence {
        Presence02::AllPresent => Ok((input, RawPresence::AllPresent)),
        Presence02::Inline => {
            let (input, bits) = parse_bitfield(input, feature_count)?;
            Ok((input, RawPresence::Bitfield(bits)))
        }
        // `ColumnType02::parse` rejected any index past the declared count.
        Presence02::Shared(index) => shared
            .get(usize::from(index))
            .map(|&bits| (input, RawPresence::Bitfield(bits)))
            .ok_or_else(|| MltError::ParsingColumnType(typ.to_byte())),
    }
}

/// Parse the geometry section: the streams the layer layout declares, in its fixed order.
///
/// Stream roles are assigned by position, mirroring the `stream_type` bytes the
/// v1 encoder would have written, so [`RawGeometry`] decoding is shared.
fn parse_geometry<'a>(
    input: &'a [u8],
    layout: GeoLayout,
    feature_count: u32,
    parser: &mut Parser,
) -> MltRefResult<'a, RawGeometry<'a>> {
    if layout.is_dict() {
        return Err(MltError::NotImplemented("v2 dict geometry layouts"));
    }
    if layout.is_tess() {
        return Err(MltError::NotImplemented("v2 tessellated geometry layouts"));
    }

    // Types stream: implicit count = feature_count.
    let (mut input, types) =
        header02::parse_stream(input, StreamCtx02::GeomTypes, feature_count, parser)?;

    let mut items = Vec::with_capacity(4);
    let lengths = [
        (layout.has_geo_lengths(), LengthType::Geometries),
        (layout.has_part_lengths(), LengthType::Parts),
        (layout.has_ring_lengths(), LengthType::Rings),
    ];
    for (present, length_type) in lengths {
        if present {
            let ctx = StreamCtx02::GeomOffsets(length_type);
            let stream;
            (input, stream) = header02::parse_stream(input, ctx, feature_count, parser)?;
            items.push(stream);
        }
    }

    // Vertex stream (explicit count in practice; context falls back to feature_count).
    let (input, vertices) =
        header02::parse_stream(input, StreamCtx02::GeomVertices, feature_count, parser)?;
    items.push(vertices);

    Ok((input, RawGeometry { meta: types, items }))
}
