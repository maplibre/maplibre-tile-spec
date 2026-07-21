//! Parser for tag `0x02` (v2) layer bodies.
//!
//! Produces the same in-memory representation as the v1 parser — a lazy
//! [`Layer01`] over `Raw*` column containers — by synthesizing per-stream
//! metadata (stream role, value count) from the envelope context instead of
//! reading it from the wire. All downstream decoding is shared with v1.
//!
//! A v2 layer body is laid out as:
//!
//! ```text
//! [varint name_len] [name bytes]
//! [varint extent]
//! [varint feature_count]
//! ── geometry section ─────────────────────────────────
//! [u8 geometry_layout]              see GeoLayout
//! [types stream]                    count = feature_count
//! [length streams per layout]       explicit counts
//! [vertex stream]                   explicit count
//! ── counted columns ──────────────────────────────────
//! [varint column_count]             ids + scalars only (geometry excluded)
//! per column:
//!   [u8 column_type]                see ColumnType02
//!   [varint name_len] [name]        only when column_type.has_name()
//!   [presence bitfield]             ceil(feature_count/8) raw bytes,
//!                                   only when column_type.is_optional()
//!   [data stream]                   count = feature_count or presence popcount
//! ```

use bitvec::order::Lsb0;
use bitvec::view::BitView as _;
use usize_cast::IntoUsize as _;

use crate::LazyParsed::Raw;
use crate::MltError::{BufferUnderflow, MissingLayerName, TrailingLayerData};
use crate::codecs::varint::parse_varint;
use crate::decoder::stream::header02;
use crate::decoder::{
    ColumnType02, DictionaryType, Extent, GeoLayout, Id, Layer01, LengthType, RawGeometry, RawId,
    RawIdValue, RawPresence, RawScalar, StreamType,
};
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

    // ── Geometry section ──────────────────────────────────────────────────
    let (input, geometry) = parse_geometry(input, feature_count, parser)?;

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
        let typ =
            ColumnType02::try_from(typ_byte).map_err(|_| MltError::ParsingColumnType(typ_byte))?;
        let name = if typ.has_name() {
            let named;
            (input, named) = parse_string(input)?;
            named
        } else {
            ""
        };
        let presence;
        (input, presence) = parse_presence(typ, input, feature_count)?;
        // The count context for this column's data stream: all features, or
        // only the present ones when a presence bitfield precedes the data.
        let data_count = match &presence {
            RawPresence::Bitfield(bits) => u32::try_from(bits.count_ones())?,
            _ => feature_count,
        };
        let data = StreamType::Data(DictionaryType::None);
        let value;
        (input, value) = header02::parse_stream(input, data, data_count, parser)?;

        #[cfg(fuzzing)]
        layer_order.push(match typ {
            ColumnType02::Id
            | ColumnType02::OptId
            | ColumnType02::LongId
            | ColumnType02::OptLongId => crate::decoder::fuzzing::LayerOrdering::Id,
            _ => crate::decoder::fuzzing::LayerOrdering::Property,
        });

        let prop = match typ {
            ColumnType02::Id | ColumnType02::OptId => {
                id_column.set_once(Raw(RawId {
                    presence,
                    value: RawIdValue::Id32(value),
                }))?;
                continue;
            }
            ColumnType02::LongId | ColumnType02::OptLongId => {
                id_column.set_once(Raw(RawId {
                    presence,
                    value: RawIdValue::Id64(value),
                }))?;
                continue;
            }
            ColumnType02::Bool | ColumnType02::OptBool => {
                RP::Bool(RawScalar::new(name, presence, value))
            }
            ColumnType02::I8 | ColumnType02::OptI8 => RP::I8(RawScalar::new(name, presence, value)),
            ColumnType02::U8 | ColumnType02::OptU8 => RP::U8(RawScalar::new(name, presence, value)),
            ColumnType02::I32 | ColumnType02::OptI32 => {
                RP::I32(RawScalar::new(name, presence, value))
            }
            ColumnType02::U32 | ColumnType02::OptU32 => {
                RP::U32(RawScalar::new(name, presence, value))
            }
            ColumnType02::I64 | ColumnType02::OptI64 => {
                RP::I64(RawScalar::new(name, presence, value))
            }
            ColumnType02::U64 | ColumnType02::OptU64 => {
                RP::U64(RawScalar::new(name, presence, value))
            }
            ColumnType02::F32 | ColumnType02::OptF32 => {
                RP::F32(RawScalar::new(name, presence, value))
            }
            ColumnType02::F64 | ColumnType02::OptF64 => {
                RP::F64(RawScalar::new(name, presence, value))
            }
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

/// Parse the presence bitfield for an optional column: `ceil(feature_count/8)`
/// raw packed bytes, borrowed zero-copy from the tile.
fn parse_presence(
    typ: ColumnType02,
    input: &[u8],
    feature_count: u32,
) -> MltRefResult<'_, RawPresence<'_>> {
    if !typ.is_optional() {
        return Ok((input, RawPresence::AllPresent));
    }
    let byte_count = feature_count.div_ceil(8);
    let (input, bytes) = take(input, byte_count)?;
    let bits = &bytes.view_bits::<Lsb0>()[..feature_count.into_usize()];
    Ok((input, RawPresence::Bitfield(bits)))
}

/// Parse the geometry section: layout byte + streams in the layout's fixed order.
///
/// Stream roles are assigned by position, mirroring the `stream_type` bytes the
/// v1 encoder would have written, so [`RawGeometry`] decoding is shared.
fn parse_geometry<'a>(
    input: &'a [u8],
    feature_count: u32,
    parser: &mut Parser,
) -> MltRefResult<'a, RawGeometry<'a>> {
    let (input, layout_byte) = parse_u8(input)?;
    let layout =
        GeoLayout::try_from(layout_byte).map_err(|_| MltError::ParsingGeoLayout(layout_byte))?;
    if layout.is_dict() {
        return Err(MltError::NotImplemented("v2 dict geometry layouts"));
    }
    if layout.is_tess() {
        return Err(MltError::NotImplemented("v2 tessellated geometry layouts"));
    }

    // Types stream: implicit count = feature_count.
    let types_role = StreamType::Length(LengthType::VarBinary);
    let (mut input, types) = header02::parse_stream(input, types_role, feature_count, parser)?;

    let mut items = Vec::with_capacity(4);
    let lengths = [
        (layout.has_geo_lengths(), LengthType::Geometries),
        (layout.has_part_lengths(), LengthType::Parts),
        (layout.has_ring_lengths(), LengthType::Rings),
    ];
    for (present, length_type) in lengths {
        if present {
            let role = StreamType::Length(length_type);
            let stream;
            (input, stream) = header02::parse_stream(input, role, feature_count, parser)?;
            items.push(stream);
        }
    }

    // Vertex stream (explicit count in practice; context falls back to feature_count).
    let vertex_role = StreamType::Data(DictionaryType::Vertex);
    let (input, vertices) = header02::parse_stream(input, vertex_role, feature_count, parser)?;
    items.push(vertices);

    Ok((input, RawGeometry { meta: types, items }))
}
