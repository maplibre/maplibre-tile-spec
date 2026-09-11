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
//! ── m-value section, only when the layout byte says so ─
//! [varint m_value_count]            non-zero
//! per m-value column:
//!   [u8 column_type]                as for a counted column, but never an id
//!                                   nor a shared dictionary
//!   [varint name_len] [name]
//!   [presence bitfield]             as for a counted column, over features
//!   [data stream]                   one value per vertex of every present
//!                                   feature, counted in its own header
//! ```

use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;
use bitvec::view::BitView as _;
use usize_cast::IntoUsize as _;

use crate::LazyParsed::Raw;
use crate::MltError::{BufferUnderflow, MissingLayerName, TrailingLayerData};
use crate::codecs::varint::parse_varint;
use crate::decoder::stream::header02;
use crate::decoder::stream::header02::{HAS_EXPLICIT_COUNT, StrLayout, StreamCtx02};
use crate::decoder::{
    Column02, ColumnType02, DataType02, DictLayout, DictionaryType, FloatLogical, GeoLayout, Id,
    Layer01, LayerLayout, LengthType, LogicalEncoding, Presence02, RawFloats, RawFloatsEncoding,
    RawFsstData, RawGeometry, RawId, RawIdValue, RawMValue, RawPlainData, RawPresence, RawScalar,
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

    if layout.m_values && !layout.geometry.allows_m_values() {
        return Err(MltError::MValuesNeedVertexCounts(layout.geometry.into()));
    }

    // ── Shared presence bitfields ─────────────────────────────────────────
    let (input, cols) = parse_shared_presence(input, layout, feature_count)?;

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
        let typ = match cols.column(typ_byte)? {
            Column02::SharedDict(kind) => {
                let shared_dict;
                (input, shared_dict) = parse_shared_dict02(input, kind, &cols, parser)?;
                properties.push(Raw(RP::SharedDict(shared_dict)));
                #[cfg(fuzzing)]
                layer_order.push(crate::decoder::fuzzing::LayerOrdering::Property);
                continue;
            }
            Column02::Values(typ) => typ,
        };
        let name;
        let presence;
        (input, name, presence) = parse_column_header(input, typ, &cols)?;
        let data_count = cols.count(&presence)?;
        #[cfg(fuzzing)]
        layer_order.push(match typ.data {
            DataType02::Id | DataType02::LongId => crate::decoder::fuzzing::LayerOrdering::Id,
            _ => crate::decoder::fuzzing::LayerOrdering::Property,
        });

        let values;
        (input, values) = parse_column_values(
            input,
            typ.data,
            name,
            presence,
            Count::Implied(data_count),
            parser,
        )?;

        let prop = match values {
            ColumnValues::Strings(strings) => RP::Str(strings),
            ColumnValues::Floats(floats) => {
                if typ.data == DataType02::F32 {
                    RP::F32(floats)
                } else {
                    RP::F64(floats)
                }
            }
            ColumnValues::Scalar(scalar) => match typ.data {
                DataType02::Id => {
                    id_column.set_once(Raw(RawId {
                        presence: scalar.presence,
                        value: RawIdValue::Id32(scalar.data),
                    }))?;
                    continue;
                }
                DataType02::LongId => {
                    id_column.set_once(Raw(RawId {
                        presence: scalar.presence,
                        value: RawIdValue::Id64(scalar.data),
                    }))?;
                    continue;
                }
                DataType02::Bool => RP::Bool(scalar),
                DataType02::I8 => RP::I8(scalar),
                DataType02::U8 => RP::U8(scalar),
                DataType02::I32 => RP::I32(scalar),
                DataType02::U32 => RP::U32(scalar),
                DataType02::I64 => RP::I64(scalar),
                DataType02::U64 => RP::U64(scalar),
                DataType02::F32 | DataType02::F64 | DataType02::Str => {
                    unreachable!("read as their own stream sets")
                }
            },
        };
        properties.push(Raw(prop));
    }

    // ── M-value section ───────────────────────────────────────────────────
    let m_values;
    (input, m_values) = if layout.m_values {
        parse_m_values(input, &cols, parser)?
    } else {
        (input, Vec::new())
    };

    if !input.is_empty() {
        return Err(TrailingLayerData(input.len()));
    }
    Ok(Layer01 {
        name: layer_name,
        extent,
        id: id_column,
        geometry: Raw(geometry),
        properties,
        m_values,
        #[cfg(fuzzing)]
        layer_order,
    })
}

/// What a column's leading data stream takes as its value count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Count {
    /// The count the envelope implies: `feature_count`, or the presence popcount.
    Implied(u32),
    /// No implied count, so the stream has to carry one, as an m-value column's does.
    Explicit,
}

impl Count {
    /// The count to read the leading stream at `input` against.
    ///
    /// A stream with no implied count is unreadable without its own, so the
    /// encoding byte is checked for one before it is parsed.
    fn resolve(self, input: &[u8], name: &str) -> MltResult<u32> {
        match self {
            Self::Implied(count) => Ok(count),
            Self::Explicit => {
                let (_, enc_byte) = parse_u8(input)?;
                if enc_byte & HAS_EXPLICIT_COUNT == 0 {
                    return Err(MltError::MValueImplicitCount {
                        name: name.to_string(),
                        byte: enc_byte,
                    });
                }
                Ok(0)
            }
        }
    }
}

/// The data streams of a v2 column of values, whichever set its data type names.
enum ColumnValues<'a> {
    Scalar(RawScalar<'a>),
    Floats(RawFloats<'a>),
    Strings(RawStrings<'a>),
}

/// Parse a column's name and presence, the two fields every column of values starts with.
fn parse_column_header<'a>(
    input: &'a [u8],
    typ: ColumnType02,
    cols: &LayerCols<'a>,
) -> MltResult<(&'a [u8], &'a str, RawPresence<'a>)> {
    let (input, name) = if typ.data.has_name() {
        parse_string(input)?
    } else {
        (input, "")
    };
    let (input, presence) = cols.presence(typ, input)?;
    Ok((input, name, presence))
}

/// Parse the data streams of a column of values, of which a counted column and an
/// m-value column hold exactly the same set.
///
/// They differ only in `count`: a counted column's leading stream takes its count
/// from the envelope, an m-value column's has to write one.
fn parse_column_values<'a>(
    input: &'a [u8],
    data: DataType02,
    name: &'a str,
    presence: RawPresence<'a>,
    count: Count,
    parser: &mut Parser,
) -> MltRefResult<'a, ColumnValues<'a>> {
    if data == DataType02::Str {
        let (input, strings) = parse_strings(input, name, presence, count, parser)?;
        return Ok((input, ColumnValues::Strings(strings)));
    }
    let leading = count.resolve(input, name)?;
    let ctx = StreamCtx02::Property(data);
    let (input, value) = header02::parse_stream(input, ctx, leading, parser)?;
    Ok(match data {
        DataType02::F32 | DataType02::F64 => {
            let (input, floats) = parse_floats(input, data, name, presence, value, parser)?;
            (input, ColumnValues::Floats(floats))
        }
        DataType02::Id
        | DataType02::LongId
        | DataType02::Bool
        | DataType02::I8
        | DataType02::U8
        | DataType02::I32
        | DataType02::U32
        | DataType02::I64
        | DataType02::U64
        | DataType02::Str => (
            input,
            ColumnValues::Scalar(RawScalar::new(name, presence, value)),
        ),
    })
}

/// Parse the m-value section: the vertex-scoped columns that end a layer body.
///
/// Each reads the streams a counted column of its data type reads, over the
/// layer's vertex sequence rather than its features. Only the geometry says how
/// long that is, which the decoder may not have read yet, so the count stays with
/// the column until [`ParsedMValue::spans`](crate::decoder::ParsedMValue::spans)
/// checks the two against each other.
fn parse_m_values<'a>(
    input: &'a [u8],
    cols: &LayerCols<'a>,
    parser: &mut Parser,
) -> MltRefResult<'a, Vec<crate::decoder::MValueColumn<'a, Lazy>>> {
    let (mut input, count) = parse_varint::<u32>(input)?;
    if count == 0 {
        return Err(MltError::EmptyMValueSection);
    }
    // Each column requires at least 1 byte (column type).
    if input.len() < count.into_usize() {
        return Err(BufferUnderflow(count, input.len()));
    }
    parser.reserve(count)?;

    let mut m_values: Vec<crate::decoder::MValueColumn<'a, Lazy>> =
        Vec::with_capacity(count.into_usize());
    let mut names: Vec<&str> = Vec::with_capacity(count.into_usize());
    for _ in 0..count {
        let typ_byte;
        (input, typ_byte) = parse_u8(input)?;
        let typ = ColumnType02::parse_m_value(typ_byte, cols.shared_count()?)?;
        let name;
        let presence;
        (input, name, presence) = parse_column_header(input, typ, cols)?;
        if names.contains(&name) {
            return Err(MltError::DuplicateMValueName(name.to_string()));
        }
        names.push(name);

        let values;
        (input, values) =
            parse_column_values(input, typ.data, name, presence, Count::Explicit, parser)?;
        let column = match (values, typ.data) {
            (ColumnValues::Strings(strings), _) => RawMValue::Str(strings),
            (ColumnValues::Floats(floats), DataType02::F32) => RawMValue::F32(floats),
            (ColumnValues::Floats(floats), _) => RawMValue::F64(floats),
            (ColumnValues::Scalar(scalar), data) => match data {
                DataType02::Bool => RawMValue::Bool(scalar),
                DataType02::I8 => RawMValue::I8(scalar),
                DataType02::U8 => RawMValue::U8(scalar),
                DataType02::I32 => RawMValue::I32(scalar),
                DataType02::U32 => RawMValue::U32(scalar),
                DataType02::I64 => RawMValue::I64(scalar),
                DataType02::U64 => RawMValue::U64(scalar),
                DataType02::Id
                | DataType02::LongId
                | DataType02::F32
                | DataType02::F64
                | DataType02::Str => {
                    unreachable!("rejected by parse_m_value, or read as their own stream sets")
                }
            },
        };
        m_values.push(Raw(column));
    }
    Ok((input, m_values))
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
/// Each child carries its own presence nibble and offsets stream.
fn parse_shared_dict02<'a>(
    input: &'a [u8],
    kind: SharedDictKind,
    cols: &LayerCols<'a>,
    parser: &mut Parser,
) -> MltRefResult<'a, RawSharedDict<'a>> {
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
            let dict = blob_layout(input)?;
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
            let dict = blob_layout(input)?;
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
        let child_typ = cols.column_type(child_byte)?;
        if child_typ.data != DataType02::Str {
            return Err(MltError::ParsingColumnType(child_byte));
        }
        let child_name;
        (input, child_name) = parse_string(input)?;
        let presence;
        (input, presence) = cols.presence(child_typ, input)?;
        let count = cols.count(&presence)?;
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
/// The blob is the last stream either dictionary layout writes, so its own byte is what names front coding.
fn blob_layout(input: &[u8]) -> MltResult<DictLayout> {
    let (_, enc_byte) = parse_u8(input)?;
    // An unknown code is left to `parse_stream`, which reports it against the blob's family.
    Ok(DictLayout::from_bits(enc_byte).unwrap_or(DictLayout::Plain))
}

/// Parse a string column, whose leading stream's extension bits name the layout the rest follow.
///
/// Every stream but that leading one carries an explicit count, or, for the byte
/// blobs, none at all, so the context only ever matters for the leading stream.
fn parse_strings<'a>(
    input: &'a [u8],
    name: &'a str,
    presence: RawPresence<'a>,
    count: Count,
    parser: &mut Parser,
) -> MltRefResult<'a, RawStrings<'a>> {
    // The layout is in the leading stream's encoding byte, which its own context is needed to read.
    let (_, enc_byte) = parse_u8(input)?;
    let layout = StrLayout::from_bits(enc_byte);
    let leading_count = count.resolve(input, name)?;
    let (input, leading) =
        header02::parse_stream(input, StreamCtx02::StrData(layout), leading_count, parser)?;
    let count = leading.meta.num_values;
    let stream = |input: &'a [u8], ctx, parser: &mut Parser| {
        header02::parse_stream(input, ctx, count, parser)
    };

    let (input, encoding) = match layout {
        StrLayout::Plain => {
            let (input, data) = stream(input, StreamCtx02::StrBlob(DictionaryType::None), parser)?;
            let plain = RawPlainData::new(leading, data)?;
            (input, RawStringsEncoding::plain(plain))
        }
        StrLayout::Dict => {
            let (input, lengths) = stream(input, StreamCtx02::StrDictLengths, parser)?;
            let dict = blob_layout(input)?;
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
            let dict = blob_layout(input)?;
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

/// What every column of a layer reads its type byte, presence bitfield and value counts against.
struct LayerCols<'a> {
    /// The layer's shared presence bitfields, which columns point into by index.
    shared: Vec<&'a BitSlice<u8, Lsb0>>,
    feature_count: u32,
}

impl<'a> LayerCols<'a> {
    /// How many shared bitfields the layer declared, capped at [`LayerLayout::MAX_SHARED_PRESENCE`].
    fn shared_count(&self) -> MltResult<u8> {
        Ok(u8::try_from(self.shared.len())?)
    }

    /// Read a column type byte as the shape its data type nibble names.
    fn column(&self, byte: u8) -> MltResult<Column02> {
        Column02::parse(byte, self.shared_count()?)
    }

    /// Read a column type byte that has to name a column of values.
    fn column_type(&self, byte: u8) -> MltResult<ColumnType02> {
        ColumnType02::parse(byte, self.shared_count()?)
    }

    /// Resolve a column's presence nibble into the bits that describe its nulls,
    /// consuming the column's own bitfield only when it has one.
    fn presence(&self, typ: ColumnType02, input: &'a [u8]) -> MltRefResult<'a, RawPresence<'a>> {
        match typ.presence {
            Presence02::AllPresent => Ok((input, RawPresence::AllPresent)),
            Presence02::Inline => {
                let (input, bits) = parse_bitfield(input, self.feature_count)?;
                Ok((input, RawPresence::Bitfield(bits)))
            }
            // `ColumnType02::parse` rejected any index past the declared count.
            Presence02::Shared(index) => self
                .shared
                .get(usize::from(index))
                .map(|&bits| (input, RawPresence::Bitfield(bits)))
                .ok_or_else(|| MltError::ParsingColumnType(typ.to_byte())),
        }
    }

    /// The count context for a column's data stream: all features, or only the
    /// present ones when a presence bitfield precedes the data.
    fn count(&self, presence: &RawPresence<'_>) -> MltResult<u32> {
        Ok(match presence {
            RawPresence::Bitfield(bits) => u32::try_from(bits.count_ones())?,
            RawPresence::AllPresent | RawPresence::Stream(_) => self.feature_count,
        })
    }
}

/// Parse the layer's shared presence bitfields: `shared_presence` back-to-back
/// bitfields of `ceil(feature_count/8)` raw packed bytes each.
///
/// The layout byte caps the count at [`LayerLayout::MAX_SHARED_PRESENCE`], so this
/// allocates nothing worth charging to the parser's budget.
fn parse_shared_presence(
    input: &[u8],
    layout: LayerLayout,
    feature_count: u32,
) -> MltRefResult<'_, LayerCols<'_>> {
    let mut input = input;
    let mut shared = Vec::with_capacity(usize::from(layout.shared_presence));
    for _ in 0..layout.shared_presence {
        let bits;
        (input, bits) = parse_bitfield(input, feature_count)?;
        shared.push(bits);
    }
    Ok((
        input,
        LayerCols {
            shared,
            feature_count,
        },
    ))
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
