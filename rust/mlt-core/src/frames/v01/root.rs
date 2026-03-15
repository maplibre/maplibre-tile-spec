use crate::analyse::{Analyze, StatType};
use crate::utils::{AsUsize as _, SetOptionOnce as _, parse_string, parse_varint};
use crate::v01::{
    Column, ColumnType, DictionaryType, Geometry, GeometryValues, Id, Layer01, Property,
    RawFsstData, RawIdValue, RawPlainData, RawPresence, RawProperty, RawScalar, RawSharedDict,
    RawSharedDictEncoding, RawSharedDictItem, RawStream, RawStrings, RawStringsEncoding,
    StreamType,
};
use crate::{Decoder, MltError, MltRefResult};

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

    fn for_each_stream(&self, cb: &mut dyn FnMut(crate::v01::StreamMeta)) {
        if let Some(ref id) = self.id {
            id.for_each_stream(cb);
        }
        self.geometry.for_each_stream(cb);
        self.properties.for_each_stream(cb);
    }
}

impl Layer01<'_> {
    /// Parse `v01::Layer` metadata
    pub fn parse(input: &[u8]) -> Result<Layer01<'_>, MltError> {
        let (input, layer_name) = parse_string(input)?;
        let (input, extent) = parse_varint::<u32>(input)?;
        let (input, column_count) = parse_varint::<u32>(input)?;

        // Each column requires at least 1 byte (column type)
        if input.len() < column_count.as_usize() {
            return Err(MltError::BufferUnderflow(column_count, input.len()));
        }

        // !!!!!!!
        // WARNING: make sure to never use `let (input, ...)` after this point: input var is reused
        let (mut input, (col_info, prop_count)) = parse_columns_meta(input, column_count)?;
        #[cfg(fuzzing)]
        let layer_order = col_info
            .iter()
            .map(|column| column.typ)
            .map(LayerOrdering::from)
            .collect();

        let mut properties = Vec::with_capacity(prop_count.as_usize());
        let mut id_column: Option<Id> = None;
        let mut geometry: Option<Geometry> = None;

        for column in col_info {
            let opt;
            let value;
            let name = column.name.unwrap_or("");

            match column.typ {
                ColumnType::Id | ColumnType::OptId => {
                    (input, opt) = parse_optional(column.typ, input)?;
                    (input, value) = RawStream::parse(input)?;
                    id_column.set_once(Id::new_raw(opt, RawIdValue::Id32(value)))?;
                }
                ColumnType::LongId | ColumnType::OptLongId => {
                    (input, opt) = parse_optional(column.typ, input)?;
                    (input, value) = RawStream::parse(input)?;
                    id_column.set_once(Id::new_raw(opt, RawIdValue::Id64(value)))?;
                }
                ColumnType::Geometry => {
                    input = parse_geometry_column(input, &mut geometry)?;
                }
                ColumnType::Bool | ColumnType::OptBool => {
                    (input, opt) = parse_optional(column.typ, input)?;
                    (input, value) = RawStream::parse_bool(input)?;
                    properties.push(Property::from(RawProperty::Bool(RawScalar {
                        name,
                        presence: RawPresence(opt),
                        data: value,
                    })));
                }
                ColumnType::I8 | ColumnType::OptI8 => {
                    (input, opt) = parse_optional(column.typ, input)?;
                    (input, value) = RawStream::parse(input)?;
                    properties.push(Property::from(RawProperty::I8(RawScalar {
                        name,
                        presence: RawPresence(opt),
                        data: value,
                    })));
                }
                ColumnType::U8 | ColumnType::OptU8 => {
                    (input, opt) = parse_optional(column.typ, input)?;
                    (input, value) = RawStream::parse(input)?;
                    properties.push(Property::from(RawProperty::U8(RawScalar {
                        name,
                        presence: RawPresence(opt),
                        data: value,
                    })));
                }
                ColumnType::I32 | ColumnType::OptI32 => {
                    (input, opt) = parse_optional(column.typ, input)?;
                    (input, value) = RawStream::parse(input)?;
                    properties.push(Property::from(RawProperty::I32(RawScalar {
                        name,
                        presence: RawPresence(opt),
                        data: value,
                    })));
                }
                ColumnType::U32 | ColumnType::OptU32 => {
                    (input, opt) = parse_optional(column.typ, input)?;
                    (input, value) = RawStream::parse(input)?;
                    properties.push(Property::from(RawProperty::U32(RawScalar {
                        name,
                        presence: RawPresence(opt),
                        data: value,
                    })));
                }
                ColumnType::I64 | ColumnType::OptI64 => {
                    (input, opt) = parse_optional(column.typ, input)?;
                    (input, value) = RawStream::parse(input)?;
                    properties.push(Property::from(RawProperty::I64(RawScalar {
                        name,
                        presence: RawPresence(opt),
                        data: value,
                    })));
                }
                ColumnType::U64 | ColumnType::OptU64 => {
                    (input, opt) = parse_optional(column.typ, input)?;
                    (input, value) = RawStream::parse(input)?;
                    properties.push(Property::from(RawProperty::U64(RawScalar {
                        name,
                        presence: RawPresence(opt),
                        data: value,
                    })));
                }
                ColumnType::F32 | ColumnType::OptF32 => {
                    (input, opt) = parse_optional(column.typ, input)?;
                    (input, value) = RawStream::parse(input)?;
                    properties.push(Property::from(RawProperty::F32(RawScalar {
                        name,
                        presence: RawPresence(opt),
                        data: value,
                    })));
                }
                ColumnType::F64 | ColumnType::OptF64 => {
                    (input, opt) = parse_optional(column.typ, input)?;
                    (input, value) = RawStream::parse(input)?;
                    properties.push(Property::from(RawProperty::F64(RawScalar {
                        name,
                        presence: RawPresence(opt),
                        data: value,
                    })));
                }
                ColumnType::Str | ColumnType::OptStr => {
                    let prop;
                    (input, prop) = parse_str_column(input, name, column.typ)?;
                    properties.push(Property::from(prop));
                }
                ColumnType::SharedDict => {
                    let prop;
                    (input, prop) = parse_shared_dict_column(input, &column)?;
                    properties.push(Property::from(prop));
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
    pub fn decode_id(&mut self, dec: &mut Decoder) -> Result<(), MltError> {
        if let Some(id) = self.id.take() {
            self.id = Some(Id::Parsed(id.decode(dec)?));
        }
        Ok(())
    }

    /// Decode only the geometry column, leaving other columns in their encoded form.
    ///
    /// Use this instead of [`Self::decode_all`] when other columns will be accessed lazily.
    pub fn decode_geometry(&mut self, dec: &mut Decoder) -> Result<(), MltError> {
        // Swap out the geometry with a temporary default (Decoded(GeometryValues::default()))
        // so we can take ownership of the raw value without unsafe code.
        let geom = std::mem::replace(
            &mut self.geometry,
            Geometry::Parsed(GeometryValues::default()),
        );
        self.geometry = Geometry::Parsed(geom.decode(dec)?);
        Ok(())
    }

    /// Decode only the property columns, leaving other columns in their encoded form.
    ///
    /// Use this instead of [`Self::decode_all`] when other columns will be accessed lazily.
    pub fn decode_properties(&mut self, dec: &mut Decoder) -> Result<(), MltError> {
        let old_props = std::mem::take(&mut self.properties);
        for prop in old_props {
            self.properties.push(Property::Parsed(prop.decode(dec)?));
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
) -> MltRefResult<'a, Vec<RawSharedDictItem<'a>>> {
    let mut children = Vec::with_capacity(column.children.len());
    for child in &column.children {
        let (inp, sc) = parse_varint::<u32>(input)?;
        let (inp, child_optional) = parse_optional(child.typ, inp)?;
        let optional_stream_count = u32::from(child_optional.is_some());
        if let Some(data_count) = sc.checked_sub(optional_stream_count)
            && data_count != 1
        {
            return Err(MltError::UnexpectedStructChildCount(data_count));
        }
        let (inp, child_data) = RawStream::parse(inp)?;
        children.push(RawSharedDictItem {
            name: child.name.unwrap_or(""),
            presence: RawPresence(child_optional),
            data: child_data,
        });
        input = inp;
    }
    Ok((input, children))
}

fn parse_optional(typ: ColumnType, input: &[u8]) -> MltRefResult<'_, Option<RawStream<'_>>> {
    if typ.is_optional() {
        let (input, optional) = RawStream::parse_bool(input)?;
        Ok((input, Some(optional)))
    } else {
        Ok((input, None))
    }
}

fn parse_geometry_column<'a>(
    input: &'a [u8],
    geometry: &mut Option<Geometry<'a>>,
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
    let (input, value) = RawStream::parse(input)?;
    // geometry items
    let (input, value_vec) = RawStream::parse_multiple(input, stream_count_capa - 1)?;
    geometry.set_once(Geometry::new_raw(value, value_vec))?;
    Ok(input)
}

fn parse_str_column<'a>(
    mut input: &'a [u8],
    name: &'a str,
    typ: ColumnType,
) -> MltRefResult<'a, RawProperty<'a>> {
    let mut stream_count = {
        let stream_count_u32;
        (input, stream_count_u32) = parse_varint::<u32>(input)?;
        stream_count_u32.as_usize()
    };
    let presence;
    (input, presence) = parse_optional(typ, input)?;
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
        (input, stream) = RawStream::parse(input)?;
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
) -> MltRefResult<'a, RawProperty<'a>> {
    // Read header streams until we hit the dictionary DATA(Single|Shared) stream.
    let stream_count;
    (input, stream_count) = parse_varint::<u32>(input)?;
    let mut dict_streams = [None, None, None, None, None];
    let mut streams_taken = 0_usize;
    while streams_taken < stream_count.as_usize() {
        let stream;
        (input, stream) = RawStream::parse(input)?;
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
    (input, children) = parse_struct_children(input, column)?;
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

fn parse_columns_meta(
    mut input: &'_ [u8],
    column_count: u32,
) -> MltRefResult<'_, (Vec<Column<'_>>, u32)> {
    use crate::v01::ColumnType::{Geometry, Id, LongId, OptId, OptLongId, SharedDict};

    let mut col_info = Vec::with_capacity(column_count.as_usize());
    let mut geometries = 0;
    let mut ids = 0;
    for _ in 0..column_count {
        let mut typ;
        (input, typ) = Column::parse(input)?;
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
                    (input, child) = Column::parse(input)?;
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

#[cfg(fuzzing)]
/// To make sure we serialize out in the same order as the original file, we need to store the order in which we parsed the columns
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum LayerOrdering {
    Id,
    Geometry,
    Property,
}

#[cfg(fuzzing)]
impl From<ColumnType> for LayerOrdering {
    fn from(typ: ColumnType) -> Self {
        use ColumnType::*;
        match typ {
            OptId | Id | LongId | OptLongId => Self::Id,
            Bool | OptBool | I8 | OptI8 | U8 | OptU8 | I32 | OptI32 | U32 | OptU32 | I64
            | OptI64 | U64 | OptU64 | F32 | OptF32 | F64 | OptF64 | Str | OptStr | SharedDict => {
                Self::Property
            }
            Geometry => Self::Geometry,
        }
    }
}
