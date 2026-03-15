use crate::analyse::{Analyze, StatType};
use crate::utils::{AsUsize as _, SetOptionOnce as _, parse_string, parse_varint};
use crate::v01::{
    Column, ColumnType, DictionaryType, Geometry, GeometryValues, Id, IdValues, Layer01, Property,
    RawFsstData, RawIdValue, RawPlainData, RawPresence, RawProperty, RawScalar, RawSharedDict,
    RawSharedDictEncoding, RawSharedDictItem, RawStream, RawStrings, RawStringsEncoding,
    StreamMeta, StreamType,
};
use crate::{Decoder, MltError, MltRefResult, Parser};

impl Analyze for Layer01<'_> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        match stat {
            StatType::DecodedMetaSize => self.name.len() + size_of::<u32>(),
            StatType::DecodedDataSize => {
                self.id.as_ref().map_or(0, |id| id.collect_statistic(stat))
                    + self.geometry.collect_statistic(stat)
                    + self.properties.collect_statistic(stat)
            }
            StatType::FeatureCount => self.geometry.collect_statistic(stat),
        }
    }

    fn for_each_stream(&self, cb: &mut dyn FnMut(StreamMeta)) {
        if let Some(ref id) = self.id {
            id.for_each_stream(cb);
        }
        self.geometry.for_each_stream(cb);
        self.properties.for_each_stream(cb);
    }
}

impl Layer01<'_> {
    /// Parse `v01::Layer` metadata, reserving decoded memory against the parser's budget.
    pub fn from_bytes<'a>(
        input: &'a [u8],
        parser: &mut Parser,
    ) -> Result<Layer01<'a>, MltError> {
        let (input, layer_name) = parse_string(input)?;
        let (input, extent) = parse_varint::<u32>(input)?;
        let (input, column_count) = parse_varint::<u32>(input)?;

        // Each column requires at least 1 byte (column type)
        if input.len() < column_count.as_usize() {
            return Err(MltError::BufferUnderflow(column_count, input.len()));
        }

        // !!!!!!!
        // WARNING: make sure to never use `let (input, ...)` after this point: input var is reused
        let (mut input, (col_info, prop_count)) = parse_columns_meta(input, column_count, parser)?;
        #[cfg(fuzzing)]
        let layer_order = col_info
            .iter()
            .map(|column| column.typ)
            .map(crate::frames::v01::fuzzing::LayerOrdering::from)
            .collect();

        let mut properties = Vec::with_capacity(prop_count.as_usize());
        let mut id_column: Option<Id> = None;
        let mut geometry: Option<Geometry> = None;

        for column in col_info {
            use crate::v01::RawProperty as RP;

            let opt;
            let value;
            let name = column.name.unwrap_or("");

            match column.typ {
                ColumnType::Id | ColumnType::OptId => {
                    (input, opt) = parse_optional(column.typ, input, parser)?;
                    (input, value) = RawStream::from_bytes(input, parser)?;
                    id_column.set_once(Id::new_raw(RawPresence(opt), RawIdValue::Id32(value)))?;
                }
                ColumnType::LongId | ColumnType::OptLongId => {
                    (input, opt) = parse_optional(column.typ, input, parser)?;
                    (input, value) = RawStream::from_bytes(input, parser)?;
                    id_column.set_once(Id::new_raw(RawPresence(opt), RawIdValue::Id64(value)))?;
                }
                ColumnType::Geometry => {
                    input = parse_geometry_column(input, &mut geometry, parser)?;
                }
                ColumnType::Bool | ColumnType::OptBool => {
                    (input, opt) = parse_optional(column.typ, input, parser)?;
                    (input, value) = RawStream::parse_bool(input, parser)?;
                    properties.push(Property::Raw(RP::Bool(scalar(name, opt, value))));
                }
                ColumnType::I8 | ColumnType::OptI8 => {
                    (input, opt) = parse_optional(column.typ, input, parser)?;
                    (input, value) = RawStream::from_bytes(input, parser)?;
                    properties.push(Property::Raw(RP::I8(scalar(name, opt, value))));
                }
                ColumnType::U8 | ColumnType::OptU8 => {
                    (input, opt) = parse_optional(column.typ, input, parser)?;
                    (input, value) = RawStream::from_bytes(input, parser)?;
                    properties.push(Property::Raw(RP::U8(scalar(name, opt, value))));
                }
                ColumnType::I32 | ColumnType::OptI32 => {
                    (input, opt) = parse_optional(column.typ, input, parser)?;
                    (input, value) = RawStream::from_bytes(input, parser)?;
                    properties.push(Property::Raw(RP::I32(scalar(name, opt, value))));
                }
                ColumnType::U32 | ColumnType::OptU32 => {
                    (input, opt) = parse_optional(column.typ, input, parser)?;
                    (input, value) = RawStream::from_bytes(input, parser)?;
                    properties.push(Property::Raw(RP::U32(scalar(name, opt, value))));
                }
                ColumnType::I64 | ColumnType::OptI64 => {
                    (input, opt) = parse_optional(column.typ, input, parser)?;
                    (input, value) = RawStream::from_bytes(input, parser)?;
                    properties.push(Property::Raw(RP::I64(scalar(name, opt, value))));
                }
                ColumnType::U64 | ColumnType::OptU64 => {
                    (input, opt) = parse_optional(column.typ, input, parser)?;
                    (input, value) = RawStream::from_bytes(input, parser)?;
                    properties.push(Property::Raw(RP::U64(scalar(name, opt, value))));
                }
                ColumnType::F32 | ColumnType::OptF32 => {
                    (input, opt) = parse_optional(column.typ, input, parser)?;
                    (input, value) = RawStream::from_bytes(input, parser)?;
                    properties.push(Property::Raw(RP::F32(scalar(name, opt, value))));
                }
                ColumnType::F64 | ColumnType::OptF64 => {
                    (input, opt) = parse_optional(column.typ, input, parser)?;
                    (input, value) = RawStream::from_bytes(input, parser)?;
                    properties.push(Property::Raw(RP::F64(scalar(name, opt, value))));
                }
                ColumnType::Str | ColumnType::OptStr => {
                    let prop;
                    (input, prop) = parse_str_column(input, name, column.typ, parser)?;
                    properties.push(Property::Raw(prop));
                }
                ColumnType::SharedDict => {
                    let prop;
                    (input, prop) = parse_shared_dict_column(input, &column, parser)?;
                    properties.push(Property::Raw(prop));
                }
            }
        }
        if input.is_empty() {
            Ok(Layer01 {
                name: layer_name,
                extent,
                id: id_column,
                geometry: geometry.ok_or(MltError::MissingGeometry)?,
                properties,
                #[cfg(fuzzing)]
                layer_order,
            })
        } else {
            Err(MltError::TrailingLayerData(input.len()))
        }
    }

    /// Decode only the ID column, leaving other columns in their encoded form.
    ///
    /// Use this instead of [`Self::decode_all`] when other columns will be accessed lazily.
    pub fn decode_id(&mut self, dec: &mut Decoder) -> Result<Option<&mut IdValues>, MltError> {
        Ok(if let Some(id) = &mut self.id {
            Some(id.decode(dec)?)
        } else {
            None
        })
    }

    /// Decode only the geometry column, leaving other columns in their encoded form.
    ///
    /// Use this instead of [`Self::decode_all`] when other columns will be accessed lazily.
    pub fn decode_geometry(&mut self, dec: &mut Decoder) -> Result<&mut GeometryValues, MltError> {
        self.geometry.decode(dec)
    }

    /// Decode only the property columns, leaving other columns in their encoded form.
    ///
    /// Use this instead of [`Self::decode_all`] when other columns will be accessed lazily.
    pub fn decode_properties(&mut self, dec: &mut Decoder) -> Result<(), MltError> {
        for prop in &mut self.properties {
            prop.decode(dec)?;
        }
        Ok(())
    }

    pub fn decode_all(&mut self, dec: &mut Decoder) -> Result<(), MltError> {
        self.decode_id(dec)?;
        self.decode_geometry(dec)?;
        self.decode_properties(dec)?;
        Ok(())
    }
}

fn parse_struct_children<'a>(
    mut input: &'a [u8],
    column: &Column<'a>,
    parser: &mut Parser,
) -> MltRefResult<'a, Vec<RawSharedDictItem<'a>>> {
    let mut children = Vec::with_capacity(column.children.len());
    for child in &column.children {
        let (inp, sc) = parse_varint::<u32>(input)?;
        let (inp, child_optional) = parse_optional(child.typ, inp, parser)?;
        let optional_stream_count = u32::from(child_optional.is_some());
        if let Some(data_count) = sc.checked_sub(optional_stream_count)
            && data_count != 1
        {
            return Err(MltError::UnexpectedStructChildCount(data_count));
        }
        let (inp, child_data) = RawStream::from_bytes(inp, parser)?;
        children.push(RawSharedDictItem {
            name: child.name.unwrap_or(""),
            presence: RawPresence(child_optional),
            data: child_data,
        });
        input = inp;
    }
    Ok((input, children))
}

fn parse_optional<'a>(
    typ: ColumnType,
    input: &'a [u8],
    parser: &mut Parser,
) -> MltRefResult<'a, Option<RawStream<'a>>> {
    if typ.is_optional() {
        let (input, optional) = RawStream::parse_bool(input, parser)?;
        Ok((input, Some(optional)))
    } else {
        Ok((input, None))
    }
}

fn parse_geometry_column<'a>(
    input: &'a [u8],
    geometry: &mut Option<Geometry<'a>>,
    parser: &mut Parser,
) -> Result<&'a [u8], MltError> {
    let (input, stream_count) = parse_varint::<u32>(input)?;
    if stream_count == 0 {
        return Err(MltError::GeometryWithoutStreams);
    }
    // Each stream requires at least 1 byte (physical stream type)
    let stream_count_capa = stream_count.as_usize();
    if input.len() < stream_count_capa {
        return Err(MltError::BufferUnderflow(stream_count, input.len()));
    }
    // metadata
    let (input, value) = RawStream::from_bytes(input, parser)?;
    // geometry items
    let (input, value_vec) = RawStream::parse_multiple(input, stream_count_capa - 1, parser)?;
    geometry.set_once(Geometry::new_raw(value, value_vec))?;
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
        stream_count_u32.as_usize()
    };
    let presence;
    (input, presence) = parse_optional(typ, input, parser)?;
    if presence.is_some() {
        if stream_count == 0 {
            return Err(MltError::UnsupportedStringStreamCount(stream_count));
        }
        stream_count -= 1;
    }
    let mut str_streams = [None, None, None, None, None];
    if stream_count > str_streams.len() {
        return Err(MltError::UnsupportedStringStreamCount(stream_count));
    }
    for slot in str_streams.iter_mut().take(stream_count) {
        let stream;
        (input, stream) = RawStream::from_bytes(input, parser)?;
        *slot = Some(stream);
    }
    let encoding = match str_streams {
        [Some(s1), Some(s2), None, None, None] => {
            RawStringsEncoding::plain(RawPlainData::new(s1, s2)?)
        }
        [Some(s1), Some(s2), Some(s3), None, None] => {
            RawStringsEncoding::dictionary(RawPlainData::new(s1, s3)?, s2)?
        }
        [Some(s1), Some(s2), Some(s3), Some(s4), None] => {
            RawStringsEncoding::fsst_plain(RawFsstData::new(s1, s2, s3, s4)?)
        }
        [Some(s1), Some(s2), Some(s3), Some(s4), Some(s5)] => {
            RawStringsEncoding::fsst_dictionary(RawFsstData::new(s1, s2, s3, s4)?, s5)?
        }
        _ => Err(MltError::UnsupportedStringStreamCount(stream_count))?,
    };
    Ok((
        input,
        RawProperty::Str(RawStrings {
            name,
            presence: RawPresence(presence),
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
    while streams_taken < stream_count.as_usize() {
        let stream;
        (input, stream) = RawStream::from_bytes(input, parser)?;
        let is_last = matches!(
            stream.meta.stream_type,
            StreamType::Data(DictionaryType::Single | DictionaryType::Shared)
        );
        dict_streams[streams_taken] = Some(stream);
        streams_taken += 1;
        if is_last {
            break;
        } else if streams_taken >= dict_streams.len() {
            return Err(MltError::UnsupportedStringStreamCount(streams_taken + 1));
        }
    }
    let children;
    (input, children) = parse_struct_children(input, column, parser)?;
    let name = column.name.unwrap_or("");
    let encoding = match dict_streams {
        [Some(s1), Some(s2), None, None, None] => {
            RawSharedDictEncoding::plain(RawPlainData::new(s1, s2)?)
        }
        [Some(s1), Some(s2), Some(s3), Some(s4), None] => {
            RawSharedDictEncoding::fsst_plain(RawFsstData::new(s1, s2, s3, s4)?)
        }
        _ => Err(MltError::SharedDictRequiresStreams(streams_taken))?,
    };
    Ok((
        input,
        RawProperty::SharedDict(RawSharedDict {
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
    use crate::v01::ColumnType::{Geometry, Id, LongId, OptId, OptLongId, SharedDict};

    let mut col_info = Vec::with_capacity(column_count.as_usize());
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
                let child_col_capacity = child_column_count.as_usize();
                if input.len() < child_col_capacity {
                    return Err(MltError::BufferUnderflow(child_column_count, input.len()));
                }
                let mut children = Vec::with_capacity(child_col_capacity);
                for _ in 0..child_column_count {
                    let child;
                    (input, child) = Column::from_bytes(input, parser)?;
                    children.push(child);
                }
                typ.children = children;
            }
            _ => {}
        }
        col_info.push(typ);
    }
    if geometries > 1 {
        return Err(MltError::MultipleGeometryColumns);
    }
    if ids > 1 {
        return Err(MltError::MultipleIdColumns);
    }

    Ok((input, (col_info, column_count - geometries - ids)))
}

fn scalar<'a>(name: &'a str, opt: Option<RawStream<'a>>, value: RawStream<'a>) -> RawScalar<'a> {
    RawScalar {
        name,
        presence: RawPresence(opt),
        data: value,
    }
}
