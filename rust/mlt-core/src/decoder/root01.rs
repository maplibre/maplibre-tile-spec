//! Parser for tag `0x01` (v1) layer bodies.

use usize_cast::IntoUsize as _;

use crate::LazyParsed::Raw;
use crate::MltError::{
    BufferUnderflow, GeometryWithoutStreams, InvalidSharedDictStreamCount, MissingGeometry,
    MissingLayerName, MultipleGeometryColumns, MultipleIdColumns, SharedDictRequiresStreams,
    TrailingLayerData, UnexpectedStructChildCount, UnsupportedStringStreamCount,
};
use crate::codecs::varint::parse_varint;
use crate::decoder::stream::header01;
use crate::decoder::{
    Column, ColumnType, DictLayout, DictionaryType, Geometry, Id, Layer01, RawFloats, RawFsstData,
    RawGeometry,
    RawId, RawIdValue, RawPlainData, RawPresence, RawProperty, RawScalar, RawSharedDict,
    RawSharedDictEncoding, RawSharedDictItem, RawStrings, RawStringsEncoding, StreamType,
    ValueKind,
};
use crate::errors::AsMltError as _;
use crate::tile::Extent;
use crate::utils::{SetOptionOnce as _, parse_string};
use crate::{Lazy, MltRefResult, MltResult, Parser};

impl<'a> Layer01<'a, Lazy> {
    /// Parse `v01::Layer` metadata, reserving decoded memory against the parser's budget.
    pub(crate) fn from_bytes(input: &'a [u8], parser: &mut Parser) -> MltResult<Self> {
        let (input, layer_name) = parse_string(input)?;
        if layer_name.is_empty() {
            return Err(MissingLayerName);
        }
        let (input, extent) = parse_varint::<u32>(input)?;
        let extent = Extent::new(extent)?;
        let (input, column_count) = parse_varint::<u32>(input)?;

        // Each column requires at least 1 byte (column type)
        if input.len() < column_count.into_usize() {
            return Err(BufferUnderflow(column_count, input.len()));
        }

        // !!!!!!!
        // WARNING: make sure to never use `let (input, ...)` after this point: input var is reused
        let (mut input, (col_info, prop_count)) = parse_columns_meta(input, column_count, parser)?;
        #[cfg(fuzzing)]
        let layer_order = col_info
            .iter()
            .map(|column| column.typ)
            .map(crate::decoder::fuzzing::LayerOrdering::from)
            .collect();

        let mut properties = Vec::with_capacity(prop_count.into_usize());
        let mut id_column: Option<Id> = None;
        let mut geometry: Option<Geometry> = None;

        for column in col_info {
            use crate::decoder::RawProperty as RP;

            let presence;
            let value;
            let name = column.name.unwrap_or("");

            match column.typ {
                ColumnType::Id | ColumnType::OptId => {
                    (input, presence) = parse_optional(column.typ, input, parser)?;
                    (input, value) = header01::parse_stream(input, ValueKind::Int, parser)?;
                    id_column.set_once(Raw(RawId {
                        presence,
                        value: RawIdValue::Id32(value),
                    }))?;
                }
                ColumnType::LongId | ColumnType::OptLongId => {
                    (input, presence) = parse_optional(column.typ, input, parser)?;
                    (input, value) = header01::parse_stream(input, ValueKind::Int, parser)?;
                    id_column.set_once(Raw(RawId {
                        presence,
                        value: RawIdValue::Id64(value),
                    }))?;
                }
                ColumnType::Geometry => {
                    input = parse_geometry_column(input, &mut geometry, parser)?;
                }
                ColumnType::Bool | ColumnType::OptBool => {
                    (input, presence) = parse_optional(column.typ, input, parser)?;
                    (input, value) = header01::parse_bool_stream(input, parser)?;
                    properties.push(Raw(RP::Bool(RawScalar::new(name, presence, value))));
                }
                ColumnType::I8 | ColumnType::OptI8 => {
                    (input, presence) = parse_optional(column.typ, input, parser)?;
                    (input, value) = header01::parse_stream(input, ValueKind::Int, parser)?;
                    properties.push(Raw(RP::I8(RawScalar::new(name, presence, value))));
                }
                ColumnType::U8 | ColumnType::OptU8 => {
                    (input, presence) = parse_optional(column.typ, input, parser)?;
                    (input, value) = header01::parse_stream(input, ValueKind::Int, parser)?;
                    properties.push(Raw(RP::U8(RawScalar::new(name, presence, value))));
                }
                ColumnType::I32 | ColumnType::OptI32 => {
                    (input, presence) = parse_optional(column.typ, input, parser)?;
                    (input, value) = header01::parse_stream(input, ValueKind::Int, parser)?;
                    properties.push(Raw(RP::I32(RawScalar::new(name, presence, value))));
                }
                ColumnType::U32 | ColumnType::OptU32 => {
                    (input, presence) = parse_optional(column.typ, input, parser)?;
                    (input, value) = header01::parse_stream(input, ValueKind::Int, parser)?;
                    properties.push(Raw(RP::U32(RawScalar::new(name, presence, value))));
                }
                ColumnType::I64 | ColumnType::OptI64 => {
                    (input, presence) = parse_optional(column.typ, input, parser)?;
                    (input, value) = header01::parse_stream(input, ValueKind::Int, parser)?;
                    properties.push(Raw(RP::I64(RawScalar::new(name, presence, value))));
                }
                ColumnType::U64 | ColumnType::OptU64 => {
                    (input, presence) = parse_optional(column.typ, input, parser)?;
                    (input, value) = header01::parse_stream(input, ValueKind::Int, parser)?;
                    properties.push(Raw(RP::U64(RawScalar::new(name, presence, value))));
                }
                ColumnType::F32 | ColumnType::OptF32 => {
                    (input, presence) = parse_optional(column.typ, input, parser)?;
                    (input, value) = header01::parse_stream(input, ValueKind::Float, parser)?;
                    properties.push(Raw(RP::F32(RawFloats::single(name, presence, value))));
                }
                ColumnType::F64 | ColumnType::OptF64 => {
                    (input, presence) = parse_optional(column.typ, input, parser)?;
                    (input, value) = header01::parse_stream(input, ValueKind::Float, parser)?;
                    properties.push(Raw(RP::F64(RawFloats::single(name, presence, value))));
                }
                ColumnType::Str | ColumnType::OptStr => {
                    let prop;
                    (input, prop) = parse_str_column(input, name, column.typ, parser)?;
                    properties.push(Raw(prop));
                }
                ColumnType::SharedDict => {
                    let prop;
                    (input, prop) = parse_shared_dict_column(input, &column, parser)?;
                    properties.push(Raw(prop));
                }
            }
        }
        if input.is_empty() {
            Ok(Layer01 {
                name: layer_name,
                extent,
                id: id_column,
                geometry: geometry.ok_or(MissingGeometry)?,
                properties,
                #[cfg(fuzzing)]
                layer_order,
            })
        } else {
            Err(TrailingLayerData(input.len()))
        }
    }
}

fn parse_shared_dict_children<'a>(
    mut input: &'a [u8],
    column: &Column<'a>,
    parser: &mut Parser,
) -> MltRefResult<'a, Vec<RawSharedDictItem<'a>>> {
    let mut children = Vec::with_capacity(column.children.len());
    for child in &column.children {
        let (inp, sc) = parse_varint::<u32>(input)?;
        let (inp, presence) = parse_optional(child.typ, inp, parser)?;
        let optional_stream_count = u32::from(presence.is_optional());
        if let Some(data_count) = sc.checked_sub(optional_stream_count)
            && data_count != 1
        {
            return Err(UnexpectedStructChildCount(data_count));
        }
        let (inp, data) = header01::parse_stream(inp, ValueKind::Int, parser)?;
        children.push(RawSharedDictItem {
            name: child.name.unwrap_or(""),
            presence,
            data,
        });
        input = inp;
    }
    Ok((input, children))
}

fn parse_optional<'a>(
    typ: ColumnType,
    input: &'a [u8],
    parser: &mut Parser,
) -> MltRefResult<'a, RawPresence<'a>> {
    if typ.is_optional() {
        let (input, optional) = header01::parse_bool_stream(input, parser)?;
        Ok((input, RawPresence::Stream(optional)))
    } else {
        Ok((input, RawPresence::AllPresent))
    }
}

fn parse_geometry_column<'a>(
    input: &'a [u8],
    geometry: &mut Option<Geometry<'a>>,
    parser: &mut Parser,
) -> MltResult<&'a [u8]> {
    let (input, stream_count) = parse_varint::<u32>(input)?;
    if stream_count == 0 {
        return Err(GeometryWithoutStreams);
    }
    // Each stream requires at least 1 byte (physical stream type)
    let stream_count_capa = stream_count.into_usize();
    if input.len() < stream_count_capa {
        return Err(BufferUnderflow(stream_count, input.len()));
    }
    // metadata
    let (input, meta) = header01::parse_stream(input, ValueKind::Int, parser)?;
    // geometry items
    let (input, items) =
        header01::parse_multiple_streams(input, stream_count_capa - 1, ValueKind::Int, parser)?;
    geometry.set_once(Raw(RawGeometry { meta, items }))?;
    Ok(input)
}

fn parse_str_column<'a>(
    mut input: &'a [u8],
    name: &'a str,
    typ: ColumnType,
    parser: &mut Parser,
) -> MltRefResult<'a, RawProperty<'a>> {
    let mut stream_count = {
        let stream_count_u32;
        (input, stream_count_u32) = parse_varint::<u32>(input)?;
        stream_count_u32.into_usize()
    };
    let presence;
    (input, presence) = parse_optional(typ, input, parser)?;
    if presence.is_optional() {
        if stream_count == 0 {
            return Err(UnsupportedStringStreamCount(stream_count));
        }
        stream_count -= 1;
    }
    let mut str_streams = [None, None, None, None, None];
    if stream_count > str_streams.len() {
        return Err(UnsupportedStringStreamCount(stream_count));
    }
    for slot in str_streams.iter_mut().take(stream_count) {
        let stream;
        (input, stream) = header01::parse_stream(input, ValueKind::Int, parser)?;
        *slot = Some(stream);
    }
    let encoding = match str_streams {
        [Some(s1), Some(s2), None, None, None] => {
            RawStringsEncoding::plain(RawPlainData::new(s1, s2)?)
        }
        [Some(s1), Some(s2), Some(s3), None, None] => {
            RawStringsEncoding::dictionary(RawPlainData::new(s1, s3)?, s2, DictLayout::Plain)?
        }
        [Some(s1), Some(s2), Some(s3), Some(s4), None] => {
            RawStringsEncoding::fsst_plain(RawFsstData::new(s1, s2, s3, s4)?)
        }
        [Some(s1), Some(s2), Some(s3), Some(s4), Some(s5)] => {
            RawStringsEncoding::fsst_dictionary(RawFsstData::new(s1, s2, s3, s4)?, s5, DictLayout::Plain)?
        }
        _ => Err(UnsupportedStringStreamCount(stream_count))?,
    };
    Ok((
        input,
        RawProperty::Str(RawStrings {
            name,
            presence,
            encoding,
        }),
    ))
}

fn parse_shared_dict_column<'a>(
    mut input: &'a [u8],
    column: &Column<'a>,
    parser: &mut Parser,
) -> MltRefResult<'a, RawProperty<'a>> {
    // Read header streams until we hit the dictionary DATA(Single|Shared) stream.
    let stream_count;
    (input, stream_count) = parse_varint::<u32>(input)?;
    let mut dict_streams = [None, None, None, None, None];
    let mut streams_taken = 0_usize;
    while streams_taken < stream_count.into_usize() {
        let stream;
        (input, stream) = header01::parse_stream(input, ValueKind::Int, parser)?;
        let is_last = matches!(
            stream.meta.stream_type,
            StreamType::Data(DictionaryType::Single | DictionaryType::Shared)
        );
        dict_streams[streams_taken] = Some(stream);
        streams_taken += 1;
        if is_last {
            break;
        } else if streams_taken >= dict_streams.len() {
            return Err(UnsupportedStringStreamCount(streams_taken + 1));
        }
    }
    let children;
    (input, children) = parse_shared_dict_children(input, column, parser)?;

    // Validate stream_count: must equal dict_streams + children + optional_children.
    let children_n = u32::try_from(children.len()).or_overflow()?;
    let optional_n = children
        .iter()
        .filter(|c| c.presence.is_optional())
        .count()
        .try_into()
        .or_overflow()?;
    let dict_n = u32::try_from(streams_taken).or_overflow()?;
    let expected = crate::utils::checked_sum3(dict_n, children_n, optional_n)?;
    // Java's encoder had a bug (fixed) that overcounted by 1: dict + 2*N + 1.
    // Accept that value too so that files produced by older Java encoders still parse.
    let java_legacy = expected.checked_add(1).or_overflow()?;
    if stream_count != expected && stream_count != java_legacy {
        return Err(InvalidSharedDictStreamCount {
            actual: stream_count,
            expected,
        });
    }

    let name = column.name.unwrap_or("");
    let encoding = match dict_streams {
        [Some(s1), Some(s2), None, None, None] => {
            RawSharedDictEncoding::plain(RawPlainData::new(s1, s2)?)
        }
        [Some(s1), Some(s2), Some(s3), Some(s4), None] => {
            RawSharedDictEncoding::fsst_plain(RawFsstData::new(s1, s2, s3, s4)?)
        }
        _ => Err(SharedDictRequiresStreams(streams_taken))?,
    };
    Ok((
        input,
        RawProperty::SharedDict(RawSharedDict {
            dict: DictLayout::Plain,
            name,
            encoding,
            children,
        }),
    ))
}

fn parse_columns_meta<'a>(
    mut input: &'a [u8],
    column_count: u32,
    parser: &mut Parser,
) -> MltRefResult<'a, (Vec<Column<'a>>, u32)> {
    use crate::decoder::ColumnType::{
        Bool, F32, F64, Geometry, I8, I32, I64, Id, LongId, OptBool, OptF32, OptF64, OptI8, OptI32,
        OptI64, OptId, OptLongId, OptStr, OptU8, OptU32, OptU64, SharedDict, Str, U8, U32, U64,
    };

    let mut col_info = Vec::with_capacity(column_count.into_usize());
    let mut geometries = 0;
    let mut ids = 0;
    for _ in 0..column_count {
        let mut typ;
        (input, typ) = Column::from_bytes(input, parser)?;
        match typ.typ {
            Geometry => geometries += 1,
            Id | OptId | LongId | OptLongId => ids += 1,
            SharedDict => {
                // Yes, we need to parse children right here; otherwise this messes up the next column
                let child_column_count;
                (input, child_column_count) = parse_varint::<u32>(input)?;

                // Each column requires at least 1 byte (ColumnType without a name)
                let child_col_capacity = child_column_count.into_usize();
                if input.len() < child_col_capacity {
                    return Err(BufferUnderflow(child_column_count, input.len()));
                }
                let mut children = Vec::with_capacity(child_col_capacity);
                for _ in 0..child_column_count {
                    let child;
                    (input, child) = Column::from_bytes(input, parser)?;
                    children.push(child);
                }
                typ.children = children;
            }
            Bool | OptBool | I8 | OptI8 | U8 | OptU8 | I32 | OptI32 | U32 | OptU32 | I64
            | OptI64 | U64 | OptU64 | F32 | OptF32 | F64 | OptF64 | Str | OptStr => {}
        }
        col_info.push(typ);
    }
    if geometries > 1 {
        return Err(MultipleGeometryColumns);
    }
    if ids > 1 {
        return Err(MultipleIdColumns);
    }

    Ok((input, (col_info, column_count - geometries - ids)))
}

#[cfg(test)]
mod tests {
    use crate::{MltError, Parser};

    #[test]
    fn parse_layers_rejects_empty_layer_name() {
        let bytes = [
            5, // layer size: tag byte + 4-byte body
            1, // tag 0x01
            0, // empty layer name
            0x80, 0x20, // extent 4096
            0,    // column count
        ];

        assert!(matches!(
            Parser::default().parse_layers(&bytes),
            Err(MltError::MissingLayerName)
        ));
    }
}
