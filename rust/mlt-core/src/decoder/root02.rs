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
use crate::decoder::stream::header02::{BlobLayout, StrLayout, StreamCtx02};
use crate::decoder::{
    ColumnType02, DataType02, DictLayout, DictionaryType, FloatLogical, GeoLayout, Id, Layer01,
    LayerLayout, LengthType, LogicalEncoding, Presence02, RawFloats, RawFloatsEncoding,
    RawFsstData, RawGeometry, RawId, RawIdValue, RawPlainData, RawPresence, RawScalar,
    RawSharedDict, RawSharedDictEncoding, RawSharedDictItem, RawStream, RawStrings,
    RawStringsEncoding, SharedDictKind,
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
        // A shared dictionary spends its presence nibble on the corpus encoding, so it is
        // taken here rather than through `ColumnType02::parse`, which reads that nibble as presence.
        if ColumnType02::fields(typ_byte).1 == DataType02::SharedDict as u8 {
            let shared_dict;
            (input, shared_dict) = parse_shared_dict02(
                input,
                typ_byte,
                &shared_presence,
                feature_count,
                layout.shared_presence,
                parser,
            )?;
            properties.push(Raw(RP::SharedDict(shared_dict)));
            #[cfg(fuzzing)]
            layer_order.push(crate::decoder::fuzzing::LayerOrdering::Property);
            continue;
        }

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
        #[cfg(fuzzing)]
        layer_order.push(match typ.data {
            DataType02::Id | DataType02::LongId => crate::decoder::fuzzing::LayerOrdering::Id,
            _ => crate::decoder::fuzzing::LayerOrdering::Property,
        });

        // A string column reads a stream set of its own, the rest one data stream.
        if typ.data == DataType02::Str {
            let strings;
            (input, strings) = parse_strings(input, name, presence, data_count, parser)?;
            properties.push(Raw(RP::Str(strings)));
            continue;
        }

        let ctx = StreamCtx02::Property(typ.data);
        let value;
        (input, value) = header02::parse_stream(input, ctx, data_count, parser)?;

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
            // `ColumnType02::parse` rejects this byte, since the loop takes it first.
            DataType02::SharedDict => return Err(MltError::ParsingColumnType(typ.to_byte())),
            DataType02::Bool => RP::Bool(RawScalar::new(name, presence, value)),
            DataType02::I8 => RP::I8(RawScalar::new(name, presence, value)),
            DataType02::U8 => RP::U8(RawScalar::new(name, presence, value)),
            DataType02::I32 => RP::I32(RawScalar::new(name, presence, value)),
            DataType02::U32 => RP::U32(RawScalar::new(name, presence, value)),
            DataType02::I64 => RP::I64(RawScalar::new(name, presence, value)),
            DataType02::U64 => RP::U64(RawScalar::new(name, presence, value)),
            DataType02::F32 | DataType02::F64 => {
                let floats;
                (input, floats) = parse_floats(input, typ.data, name, presence, value, parser)?;
                if typ.data == DataType02::F32 {
                    RP::F32(floats)
                } else {
                    RP::F64(floats)
                }
            }
            DataType02::Str => unreachable!("string columns are read before this match"),
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

/// Finish a float column, reading the dictionary stream when its data stream turned out to be one of codes.
fn parse_floats<'a>(
    input: &'a [u8],
    typ: DataType02,
    name: &'a str,
    presence: RawPresence<'a>,
    data: RawStream<'a>,
    parser: &mut Parser,
) -> MltRefResult<'a, RawFloats<'a>> {
    let (input, encoding) = match data.meta.encoding.logical {
        LogicalEncoding::Float(FloatLogical::Alp(params)) => {
            (input, RawFloatsEncoding::Alp { params, data })
        }
        LogicalEncoding::Float(FloatLogical::Dict) => {
            // The dictionary's count is explicit in its header, so this fallback is never used.
            let ctx = StreamCtx02::PropertyDictionary(typ);
            let (input, dictionary) =
                header02::parse_stream(input, ctx, data.meta.num_values, parser)?;
            let encoding = RawFloatsEncoding::Dictionary {
                codes: data,
                dictionary,
            };
            (input, encoding)
        }
        LogicalEncoding::Float(FloatLogical::None)
        | LogicalEncoding::Int(_)
        | LogicalEncoding::Bool(_)
        | LogicalEncoding::Vertex(_) => (input, RawFloatsEncoding::Single(data)),
    };
    Ok((
        input,
        RawFloats {
            name,
            presence,
            encoding,
        },
    ))
}

/// Parse a shared-dictionary column: its corpus, then the children that index into it.
///
/// The type byte's high nibble names the corpus encoding rather than presence, since the group
/// holds no values of its own. Each child carries its own presence nibble and offsets stream.
#[allow(
    clippy::too_many_arguments,
    reason = "each argument is a distinct wire input"
)]
fn parse_shared_dict02<'a>(
    input: &'a [u8],
    typ_byte: u8,
    shared: &[&'a BitSlice<u8, Lsb0>],
    feature_count: u32,
    shared_count: u8,
    parser: &mut Parser,
) -> MltRefResult<'a, RawSharedDict<'a>> {
    let kind = SharedDictKind::parse(ColumnType02::fields(typ_byte).0)
        .ok_or(MltError::ParsingColumnType(typ_byte))?;
    let (input, name) = parse_string(input)?;
    let (input, child_count) = parse_varint::<u32>(input)?;
    parser.reserve(child_count)?;

    let stream =
        |input: &'a [u8], ctx, parser: &mut Parser| header02::parse_stream(input, ctx, 0, parser);
    // The corpus streams mirror a lone string column's dictionary tail, so the blob that ends
    // them is what names front coding here too.
    let (input, encoding, dict) = match kind {
        SharedDictKind::Plain => {
            let (input, lengths) = stream(input, StreamCtx02::StrDictLengths, parser)?;
            let dict = blob_dict_layout(input)?;
            let (input, data) =
                stream(input, StreamCtx02::StrBlob(DictionaryType::Shared), parser)?;
            let plain = RawPlainData::new(lengths, data)?;
            (input, RawSharedDictEncoding::plain(plain), dict)
        }
        SharedDictKind::Fsst => {
            let (input, lengths) = stream(input, StreamCtx02::StrDictLengths, parser)?;
            let (input, symbol_lengths) = stream(input, StreamCtx02::StrSymbolLengths, parser)?;
            let (input, symbols) =
                stream(input, StreamCtx02::StrBlob(DictionaryType::Fsst), parser)?;
            let dict = blob_dict_layout(input)?;
            let (input, corpus) =
                stream(input, StreamCtx02::StrBlob(DictionaryType::Shared), parser)?;
            let fsst = RawFsstData::new(symbol_lengths, symbols, lengths, corpus)?;
            (input, RawSharedDictEncoding::fsst_plain(fsst), dict)
        }
    };

    let mut input = input;
    let mut children = Vec::with_capacity(child_count.into_usize());
    for _ in 0..child_count {
        let child_byte;
        (input, child_byte) = parse_u8(input)?;
        let child_typ = ColumnType02::parse(child_byte, shared_count)?;
        if child_typ.data != DataType02::Str {
            return Err(MltError::ParsingColumnType(child_byte));
        }
        let child_name;
        (input, child_name) = parse_string(input)?;
        let presence;
        (input, presence) = parse_presence(child_typ, shared, input, feature_count)?;
        let count = match &presence {
            RawPresence::Bitfield(bits) => u32::try_from(bits.count_ones())?,
            RawPresence::AllPresent | RawPresence::Stream(_) => feature_count,
        };
        let data;
        (input, data) =
            header02::parse_stream(input, StreamCtx02::StrData(StrLayout::Dict), count, parser)?;
        children.push(RawSharedDictItem {
            name: child_name,
            presence,
            data,
        });
    }

    Ok((input, RawSharedDict::new(name, encoding, dict, children)))
}

/// Read how a dictionary blob lays its entries out, from the encoding byte its stream begins with.
///
/// The blob is the last stream either dictionary layout writes, so its own byte is what names
/// front coding; a stream count would not fit v2's positional string columns.
fn blob_dict_layout(input: &[u8]) -> MltResult<DictLayout> {
    let (_, enc_byte) = parse_u8(input)?;
    Ok(match header02::peek_blob_layout(enc_byte) {
        Some(BlobLayout::FrontCoded) => DictLayout::FrontCoded,
        // An unknown code is left to `parse_stream`, which reports it against the blob's family.
        Some(BlobLayout::Plain) | None => DictLayout::Plain,
    })
}

/// Parse a string column, whose leading stream's extension bits name the layout the rest follow.
///
/// Every stream but that leading one carries an explicit count, or, for the byte
/// blobs, none at all, so `count` is only ever the leading stream's context.
fn parse_strings<'a>(
    input: &'a [u8],
    name: &'a str,
    presence: RawPresence<'a>,
    count: u32,
    parser: &mut Parser,
) -> MltRefResult<'a, RawStrings<'a>> {
    // The layout is in the leading stream's encoding byte, which its own context is needed to read.
    let (_, enc_byte) = parse_u8(input)?;
    let layout = StrLayout::from_bits(enc_byte);
    let stream = |input: &'a [u8], ctx, parser: &mut Parser| {
        header02::parse_stream(input, ctx, count, parser)
    };
    let (input, leading) = stream(input, StreamCtx02::StrData(layout), parser)?;

    let (input, encoding) = match layout {
        StrLayout::Plain => {
            let (input, data) = stream(input, StreamCtx02::StrBlob(DictionaryType::None), parser)?;
            let plain = RawPlainData::new(leading, data)?;
            (input, RawStringsEncoding::plain(plain))
        }
        StrLayout::Dict => {
            let (input, lengths) = stream(input, StreamCtx02::StrDictLengths, parser)?;
            let dict = blob_dict_layout(input)?;
            let (input, data) =
                stream(input, StreamCtx02::StrBlob(DictionaryType::Single), parser)?;
            let plain = RawPlainData::new(lengths, data)?;
            (input, RawStringsEncoding::dictionary(plain, leading, dict)?)
        }
        StrLayout::Fsst => {
            let (input, symbol_lengths) = stream(input, StreamCtx02::StrSymbolLengths, parser)?;
            let (input, symbols) =
                stream(input, StreamCtx02::StrBlob(DictionaryType::Fsst), parser)?;
            let (input, corpus) =
                stream(input, StreamCtx02::StrBlob(DictionaryType::Single), parser)?;
            let fsst = RawFsstData::new(symbol_lengths, symbols, leading, corpus)?;
            (input, RawStringsEncoding::fsst_plain(fsst))
        }
        StrLayout::FsstDict => {
            let (input, lengths) = stream(input, StreamCtx02::StrDictLengths, parser)?;
            let (input, symbol_lengths) = stream(input, StreamCtx02::StrSymbolLengths, parser)?;
            let (input, symbols) =
                stream(input, StreamCtx02::StrBlob(DictionaryType::Fsst), parser)?;
            let dict = blob_dict_layout(input)?;
            let (input, corpus) =
                stream(input, StreamCtx02::StrBlob(DictionaryType::Single), parser)?;
            let fsst = RawFsstData::new(symbol_lengths, symbols, lengths, corpus)?;
            (
                input,
                RawStringsEncoding::fsst_dictionary(fsst, leading, dict)?,
            )
        }
    };
    Ok((
        input,
        RawStrings {
            name,
            presence,
            encoding,
        },
    ))
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
    // Types stream: implicit count = feature_count.
    let (mut input, types) =
        header02::parse_stream(input, StreamCtx02::GeomTypes, feature_count, parser)?;

    let mut items = Vec::with_capacity(6);
    // Each stream's role comes from its position, so they only differ in context.
    let mut stream = |input: &'a [u8], ctx, items: &mut Vec<_>| -> MltResult<&'a [u8]> {
        let (rest, parsed) = header02::parse_stream(input, ctx, feature_count, parser)?;
        items.push(parsed);
        Ok(rest)
    };

    let lengths = [
        (layout.has_geo_lengths(), LengthType::Geometries),
        (layout.has_part_lengths(), LengthType::Parts),
        (layout.has_ring_lengths(), LengthType::Rings),
    ];
    for (present, length_type) in lengths {
        if present {
            input = stream(input, StreamCtx02::GeomOffsets(length_type), &mut items)?;
        }
    }

    if layout.is_tess() {
        let triangles = StreamCtx02::GeomOffsets(LengthType::Triangles);
        input = stream(input, triangles, &mut items)?;
        input = stream(input, StreamCtx02::GeomIndices, &mut items)?;
    }

    // Vertex stream, holding the whole vertex sequence or a dictionary of the distinct
    // ones (explicit count in practice; context falls back to feature_count).
    input = stream(input, StreamCtx02::GeomVertices, &mut items)?;
    if layout.is_dict() {
        input = stream(input, StreamCtx02::GeomVertexOffsets, &mut items)?;
    }

    Ok((input, RawGeometry { meta: types, items }))
}
