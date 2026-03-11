use std::io;
use std::io::Write;

use borrowme::borrowme;

use crate::analyse::{Analyze, StatType};
use crate::utils::{AsUsize as _, SetOptionOnce as _, parse_string, parse_varint};
use crate::v01::{
    Column, ColumnType, DictionaryType, EncodedIdValue, EncodedPresence, EncodedProperty,
    EncodedSharedDict, EncodedSharedDictChild, EncodedStrings, Geometry, Id, NameRef, Property,
    Stream, StreamType,
};
use crate::{Decodable as _, MltError, MltRefResult, utils};

/// Representation of a feature table layer encoded as MLT tag `0x01`
#[cfg(not(fuzzing))]
#[borrowme]
#[derive(Debug, PartialEq)]
#[cfg_attr(
    all(not(test), not(fuzzing), feature = "arbitrary"),
    owned_attr(derive(arbitrary::Arbitrary))
)]
pub struct Layer01<'a> {
    pub name: &'a str,
    pub extent: u32,
    pub id: Id<'a>,
    pub geometry: Geometry<'a>,
    pub properties: Vec<Property<'a>>,
}

/// FIXME: fuzzing is only adding layer_order but this borrowme does not codegen correctly in this case
#[cfg(fuzzing)]
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct Layer01<'a> {
    pub name: &'a str,
    pub extent: u32,
    pub id: Id<'a>,
    pub geometry: Geometry<'a>,
    pub properties: Vec<Property<'a>>,
    pub layer_order: Vec<LayerOrdering>,
}

impl Analyze for Layer01<'_> {
    fn collect_statistic(&self, stat: StatType) -> usize {
        match stat {
            StatType::DecodedMetaSize => self.name.len() + size_of::<u32>(),
            StatType::DecodedDataSize => {
                self.id.collect_statistic(stat)
                    + self.geometry.collect_statistic(stat)
                    + self.properties.collect_statistic(stat)
            }
            StatType::FeatureCount => self.geometry.collect_statistic(stat),
        }
    }

    fn for_each_stream(&self, cb: &mut dyn FnMut(&Stream<'_>)) {
        self.id.for_each_stream(cb);
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
        let mut id_stream: Option<Id> = None;
        let mut geometry: Option<Geometry> = None;

        for column in col_info {
            let opt;
            let value;
            let name = column.name.unwrap_or("");

            match column.typ {
                ColumnType::Id | ColumnType::OptId => {
                    (input, opt) = parse_optional(column.typ, input)?;
                    (input, value) = Stream::parse(input)?;
                    id_stream.set_once(Id::new_encoded(opt, EncodedIdValue::Id32(value)))?;
                }
                ColumnType::LongId | ColumnType::OptLongId => {
                    (input, opt) = parse_optional(column.typ, input)?;
                    (input, value) = Stream::parse(input)?;
                    id_stream.set_once(Id::new_encoded(opt, EncodedIdValue::Id64(value)))?;
                }
                ColumnType::Geometry => {
                    input = parse_geometry_column(input, &mut geometry)?;
                }
                ColumnType::Bool | ColumnType::OptBool => {
                    (input, opt) = parse_optional(column.typ, input)?;
                    (input, value) = Stream::parse_bool(input)?;
                    properties.push(Property::from(EncodedProperty::Bool(
                        NameRef(name),
                        EncodedPresence(opt),
                        value,
                    )));
                }
                ColumnType::I8 | ColumnType::OptI8 => {
                    (input, opt) = parse_optional(column.typ, input)?;
                    (input, value) = Stream::parse(input)?;
                    properties.push(Property::from(EncodedProperty::I8(
                        NameRef(name),
                        EncodedPresence(opt),
                        value,
                    )));
                }
                ColumnType::U8 | ColumnType::OptU8 => {
                    (input, opt) = parse_optional(column.typ, input)?;
                    (input, value) = Stream::parse(input)?;
                    properties.push(Property::from(EncodedProperty::U8(
                        NameRef(name),
                        EncodedPresence(opt),
                        value,
                    )));
                }
                ColumnType::I32 | ColumnType::OptI32 => {
                    (input, opt) = parse_optional(column.typ, input)?;
                    (input, value) = Stream::parse(input)?;
                    properties.push(Property::from(EncodedProperty::I32(
                        NameRef(name),
                        EncodedPresence(opt),
                        value,
                    )));
                }
                ColumnType::U32 | ColumnType::OptU32 => {
                    (input, opt) = parse_optional(column.typ, input)?;
                    (input, value) = Stream::parse(input)?;
                    properties.push(Property::from(EncodedProperty::U32(
                        NameRef(name),
                        EncodedPresence(opt),
                        value,
                    )));
                }
                ColumnType::I64 | ColumnType::OptI64 => {
                    (input, opt) = parse_optional(column.typ, input)?;
                    (input, value) = Stream::parse(input)?;
                    properties.push(Property::from(EncodedProperty::I64(
                        NameRef(name),
                        EncodedPresence(opt),
                        value,
                    )));
                }
                ColumnType::U64 | ColumnType::OptU64 => {
                    (input, opt) = parse_optional(column.typ, input)?;
                    (input, value) = Stream::parse(input)?;
                    properties.push(Property::from(EncodedProperty::U64(
                        NameRef(name),
                        EncodedPresence(opt),
                        value,
                    )));
                }
                ColumnType::F32 | ColumnType::OptF32 => {
                    (input, opt) = parse_optional(column.typ, input)?;
                    (input, value) = Stream::parse(input)?;
                    properties.push(Property::from(EncodedProperty::F32(
                        NameRef(name),
                        EncodedPresence(opt),
                        value,
                    )));
                }
                ColumnType::F64 | ColumnType::OptF64 => {
                    (input, opt) = parse_optional(column.typ, input)?;
                    (input, value) = Stream::parse(input)?;
                    properties.push(Property::from(EncodedProperty::F64(
                        NameRef(name),
                        EncodedPresence(opt),
                        value,
                    )));
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
                id: id_stream.unwrap_or(Id::Encoded(None)),
                geometry: geometry.ok_or(MltError::MissingGeometry)?,
                properties,
                #[cfg(fuzzing)]
                layer_order,
            })
        } else {
            Err(MltError::TrailingLayerData(input.len()))
        }
    }

    /// Decode only the ID column, leaving properties in their encoded form.
    ///
    /// Use this instead of [`Self::decode_all`] when properties will be accessed lazily
    pub fn decode_id(&mut self) -> Result<(), MltError> {
        self.id.materialize()?;
        Ok(())
    }

    /// Decode only the geometry column, leaving properties in their encoded form.
    ///
    /// Use this instead of [`Self::decode_all`] when properties will be accessed lazily
    pub fn decode_geometry(&mut self) -> Result<(), MltError> {
        self.geometry.materialize()?;
        Ok(())
    }

    /// Decode only the properties columns, leaving the ID and geometry columns in their encoded form.
    ///
    /// Use this instead of [`Self::decode_all`] when the ID or geometry will be accessed lazily
    pub fn decode_properties(&mut self) -> Result<(), MltError> {
        let old_props = std::mem::take(&mut self.properties);
        for prop in old_props {
            self.properties.push(Property::Decoded(prop.decode()?));
        }
        Ok(())
    }

    pub fn decode_all(&mut self) -> Result<(), MltError> {
        self.decode_id()?;
        self.decode_geometry()?;
        self.decode_properties()?;
        Ok(())
    }
}

fn parse_struct_children<'a>(
    mut input: &'a [u8],
    column: &Column<'a>,
) -> MltRefResult<'a, Vec<EncodedSharedDictChild<'a>>> {
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
        let (inp, child_data) = Stream::parse(inp)?;
        children.push(EncodedSharedDictChild {
            name: NameRef(child.name.unwrap_or("")),
            presence: EncodedPresence(child_optional),
            data: child_data,
        });
        input = inp;
    }
    Ok((input, children))
}

fn parse_optional(typ: ColumnType, input: &[u8]) -> MltRefResult<'_, Option<Stream<'_>>> {
    if typ.is_optional() {
        let (input, optional) = Stream::parse_bool(input)?;
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
    let (input, value) = Stream::parse(input)?;
    // geometry items
    let (input, value_vec) = Stream::parse_multiple(input, stream_count_capa - 1)?;
    geometry.set_once(Geometry::new_encoded(value, value_vec))?;
    Ok(input)
}

fn parse_str_column<'a>(
    mut input: &'a [u8],
    name: &'a str,
    typ: ColumnType,
) -> MltRefResult<'a, EncodedProperty<'a>> {
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
        (input, stream) = Stream::parse(input)?;
        *slot = Some(stream);
    }
    let encoding = match str_streams {
        [Some(s1), Some(s2), None, None, None] => EncodedStrings::plain(s1, s2)?,
        [Some(s1), Some(s2), Some(s3), None, None] => EncodedStrings::dictionary(s1, s2, s3)?,
        [Some(s1), Some(s2), Some(s3), Some(s4), None] => {
            EncodedStrings::fsst_plain(s1, s2, s3, s4)?
        }
        [Some(s1), Some(s2), Some(s3), Some(s4), Some(s5)] => {
            EncodedStrings::fsst_dictionary(s1, s2, s3, s4, s5)?
        }
        _ => Err(MltError::UnsupportedStringStreamCount(stream_count))?,
    };
    Ok((
        input,
        EncodedProperty::Str(NameRef(name), EncodedPresence(presence), encoding),
    ))
}

fn parse_shared_dict_column<'a>(
    mut input: &'a [u8],
    column: &Column<'a>,
) -> MltRefResult<'a, EncodedProperty<'a>> {
    // Read header streams until we hit the dictionary DATA(Single|Shared) stream.
    let stream_count;
    (input, stream_count) = parse_varint::<u32>(input)?;
    let mut dict_streams = [None, None, None, None, None];
    let mut streams_taken = 0_usize;
    while streams_taken < stream_count.as_usize() {
        let stream;
        (input, stream) = Stream::parse(input)?;
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
    let prefix = NameRef(column.name.unwrap_or(""));
    let shared_dict = match dict_streams {
        [Some(s1), Some(s2), None, None, None] => EncodedSharedDict::plain(s1, s2)?,
        [Some(s1), Some(s2), Some(s3), Some(s4), None] => {
            EncodedSharedDict::fsst_plain(s1, s2, s3, s4)?
        }
        _ => Err(MltError::SharedDictRequiresStreams(streams_taken))?,
    };
    Ok((
        input,
        EncodedProperty::SharedDict(prefix, shared_dict, children),
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

impl OwnedLayer01 {
    /// Write Layer's binary representation to a Write stream without allocating a Vec
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        use integer_encoding::VarIntWriter as _;
        use utils::BinarySerializer as _;

        writer.write_string(&self.name)?;
        writer.write_varint(self.extent)?;

        // write size
        let has_id = self.id.is_present();
        let id_columns_count = u32::from(has_id);
        let geometry_column_count = 1;
        let property_column_count = u32::try_from(self.properties.len()).map_err(MltError::from)?;
        let column_count = property_column_count + id_columns_count + geometry_column_count;
        writer.write_varint(column_count)?;

        let map_error_to_io = |e: MltError| match e {
            MltError::Io(e) => e,
            e => io::Error::other(e),
        };
        self.write_columns_meta_to(writer)
            .map_err(map_error_to_io)?;
        self.write_columns_to(writer).map_err(map_error_to_io)?;
        Ok(())
    }

    #[cfg(not(fuzzing))]
    fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        self.id.write_columns_meta_to(writer)?;
        self.geometry.write_columns_meta_to(writer)?;
        for prop in &self.properties {
            prop.write_columns_meta_to(writer)?;
        }
        Ok(())
    }
    #[cfg(fuzzing)]
    fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        let props = &mut self.properties.iter();
        for ord in &self.layer_order {
            match ord {
                LayerOrdering::Id => self.id.write_columns_meta_to(writer)?,
                LayerOrdering::Geometry => self.geometry.write_columns_meta_to(writer)?,
                LayerOrdering::Property => {
                    let prop = props.next().expect(
                        "the number of layer order elements must match the number of properties",
                    );
                    prop.write_columns_meta_to(writer)?;
                }
            }
        }
        Ok(())
    }
    #[cfg(not(fuzzing))]
    fn write_columns_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        self.id.write_to(writer)?;
        self.geometry.write_to(writer)?;
        for prop in &self.properties {
            prop.write_to(writer)?;
        }
        Ok(())
    }
    #[cfg(fuzzing)]
    fn write_columns_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        let props = &mut self.properties.iter();
        for ord in &self.layer_order {
            match ord {
                LayerOrdering::Id => self.id.write_to(writer)?,
                LayerOrdering::Geometry => self.geometry.write_to(writer)?,
                LayerOrdering::Property => {
                    let prop = props.next().expect(
                        "the number of layer order elements must match the number of properties",
                    );
                    prop.write_to(writer)?;
                }
            }
        }
        Ok(())
    }
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

#[cfg(all(fuzzing, feature = "arbitrary"))]
impl arbitrary::Arbitrary<'_> for OwnedLayer01 {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        use crate::v01::{OwnedGeometry, OwnedId, OwnedProperty};

        let name: String = u.arbitrary()?;
        let extent: u32 = u.arbitrary()?;
        let id: OwnedId = u.arbitrary()?;
        let geometry: OwnedGeometry = u.arbitrary()?;
        let properties: Vec<OwnedProperty> = u.arbitrary()?;

        // Build a valid layer_order: 1 Geometry, N Property (one per property),
        // and optionally 1 ID when the layer carries an ID column, then shuffle.
        let mut layer_order: Vec<LayerOrdering> = Vec::new();
        if id.is_present() {
            layer_order.push(LayerOrdering::Id);
        }
        layer_order.push(LayerOrdering::Geometry);
        for _ in &properties {
            layer_order.push(LayerOrdering::Property);
        }

        // Fisher-Yates shuffle using arbitrary indices
        let n = layer_order.len();
        for i in (1..n).rev() {
            let j: usize = u.int_in_range(0..=i)?;
            layer_order.swap(i, j);
        }

        Ok(OwnedLayer01 {
            name,
            extent,
            id,
            geometry,
            properties,
            layer_order,
        })
    }
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
