use std::io;
use std::io::Write;

use borrowme::borrowme;
use integer_encoding::VarIntWriter;

use crate::decodable::{FromRaw, impl_decodable};
use crate::utils::SetOptionOnce;
use crate::v01::{
    Column, ColumnType, DecodedGeometry, DecodedId, DecodedProperty, Geometry, Id, OwnedId,
    Property, RawIdValue, RawPropValue, Stream,
};
use crate::{MltError, MltRefResult, utils};

/// feature table capable of storing references or own data, similar to `Cow`
/// Note that in many cases full decoding is not needed - individual columns can be decoded
/// inside a raw feature table.
#[borrowme]
#[derive(Debug, PartialEq)]
pub enum Layer01<'a> {
    Raw(RawLayer01<'a>),
    Decoded(DecodedLayer01),
}

#[borrowme]
#[derive(Debug, PartialEq)]
pub struct RawLayer01<'a> {
    pub name: &'a str,
    pub extent: u32,
    pub id: Id<'a>,
    pub geometry: Geometry<'a>,
    pub properties: Vec<Property<'a>>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DecodedLayer01 {
    pub name: String,
    pub extent: u32,
    pub id: DecodedId,
    pub geometry: DecodedGeometry,
    pub properties: Vec<DecodedProperty>,
}

impl_decodable!(Layer01<'a>, RawLayer01<'a>, DecodedLayer01);

impl<'a> From<RawLayer01<'a>> for Layer01<'a> {
    fn from(value: RawLayer01<'a>) -> Self {
        Self::Raw(value)
    }
}

impl<'a> Layer01<'a> {
    #[inline]
    pub fn decode(self) -> Result<DecodedLayer01, MltError> {
        Ok(match self {
            Self::Raw(v) => DecodedLayer01::from_raw(v)?,
            Self::Decoded(v) => v,
        })
    }
}

impl<'a> FromRaw<'a> for DecodedLayer01 {
    type Input = RawLayer01<'a>;

    fn from_raw(raw: RawLayer01<'a>) -> Result<Self, MltError> {
        // For now, just convert the raw data to owned data
        // In the future, this could do more sophisticated decoding
        Ok(DecodedLayer01 {
            name: raw.name.to_string(),
            extent: raw.extent,
            id: DecodedId::default(), // TODO: implement proper ID decoding
            geometry: raw.geometry.decode()?,
            properties: raw
                .properties
                .into_iter()
                .map(|_| DecodedProperty())
                .collect(), // TODO: implement proper property decoding
        })
    }
}

fn parse_optional(typ: ColumnType, input: &[u8]) -> MltRefResult<'_, Option<Stream<'_>>> {
    if typ.is_optional() {
        let (input, optional) = Stream::parse_bool(input)?;
        Ok((input, Some(optional)))
    } else {
        Ok((input, None))
    }
}

impl RawLayer01<'_> {
    /// Parse `v01::Layer` metadata
    pub fn parse(input: &[u8]) -> Result<RawLayer01<'_>, MltError> {
        let (input, layer_name) = utils::parse_string(input)?;
        let (input, extent) = utils::parse_varint::<u32>(input)?;
        let (input, column_count) = utils::parse_varint::<usize>(input)?;

        // !!!!!!!
        // WARNING: make sure to never use `let (input, ...)` after this point, as input var is reused
        let (mut input, (col_info, prop_count)) = parse_columns_meta(input, column_count)?;

        let mut properties = Vec::with_capacity(prop_count);
        let mut id_stream: Option<Id> = None;
        let mut geometry: Option<Geometry> = None;

        for column in col_info {
            let optional;
            let value;
            let stream_count;
            let name = column.name.unwrap_or("");

            match column.typ {
                ColumnType::Id | ColumnType::OptId => {
                    (input, optional) = parse_optional(column.typ, input)?;
                    (input, value) = Stream::parse(input)?;
                    id_stream.set_once(Id::raw(optional, RawIdValue::Id(value)))?;
                }
                ColumnType::LongId | ColumnType::OptLongId => {
                    (input, optional) = parse_optional(column.typ, input)?;
                    (input, value) = Stream::parse(input)?;
                    id_stream.set_once(Id::raw(optional, RawIdValue::LongId(value)))?;
                }
                ColumnType::Geometry => {
                    let value_vec;
                    (input, stream_count) = utils::parse_varint::<usize>(input)?;
                    (input, value) = Stream::parse(input)?;
                    (input, value_vec) = Stream::parse_multiple(input, stream_count - 1)?;
                    geometry.set_once(Geometry::raw(value, value_vec))?;
                }
                ColumnType::Bool | ColumnType::OptBool => {
                    (input, optional) = parse_optional(column.typ, input)?;
                    (input, value) = Stream::parse_bool(input)?;
                    properties.push(Property::raw(name, optional, RawPropValue::Bool(value)));
                }
                ColumnType::I8 | ColumnType::OptI8 => {
                    (input, optional) = parse_optional(column.typ, input)?;
                    (input, value) = Stream::parse(input)?;
                    properties.push(Property::raw(name, optional, RawPropValue::I8(value)));
                }
                ColumnType::U8 | ColumnType::OptU8 => {
                    (input, optional) = parse_optional(column.typ, input)?;
                    (input, value) = Stream::parse(input)?;
                    properties.push(Property::raw(name, optional, RawPropValue::U8(value)));
                }
                ColumnType::I32 | ColumnType::OptI32 => {
                    (input, optional) = parse_optional(column.typ, input)?;
                    (input, value) = Stream::parse(input)?;
                    properties.push(Property::raw(name, optional, RawPropValue::I32(value)));
                }
                ColumnType::U32 | ColumnType::OptU32 => {
                    (input, optional) = parse_optional(column.typ, input)?;
                    (input, value) = Stream::parse(input)?;
                    properties.push(Property::raw(name, optional, RawPropValue::U32(value)));
                }
                ColumnType::I64 | ColumnType::OptI64 => {
                    (input, optional) = parse_optional(column.typ, input)?;
                    (input, value) = Stream::parse(input)?;
                    properties.push(Property::raw(name, optional, RawPropValue::I64(value)));
                }
                ColumnType::U64 | ColumnType::OptU64 => {
                    (input, optional) = parse_optional(column.typ, input)?;
                    (input, value) = Stream::parse(input)?;
                    properties.push(Property::raw(name, optional, RawPropValue::U64(value)));
                }
                ColumnType::F32 | ColumnType::OptF32 => {
                    (input, optional) = parse_optional(column.typ, input)?;
                    (input, value) = Stream::parse(input)?;
                    properties.push(Property::raw(name, optional, RawPropValue::F32(value)));
                }
                ColumnType::F64 | ColumnType::OptF64 => {
                    (input, optional) = parse_optional(column.typ, input)?;
                    (input, value) = Stream::parse(input)?;
                    properties.push(Property::raw(name, optional, RawPropValue::F64(value)));
                }
                ColumnType::Str | ColumnType::OptStr => {
                    (input, stream_count) = utils::parse_varint::<usize>(input)?;
                    (input, optional) = parse_optional(column.typ, input)?;
                    // if optional has a value, one stream has already been consumed
                    let stream_count = stream_count - usize::from(optional.is_some());
                    let value_vec;
                    (input, value_vec) = Stream::parse_multiple(input, stream_count)?;
                    properties.push(Property::raw(name, optional, RawPropValue::Str(value_vec)));
                }
                ColumnType::Struct => {
                    todo!("Struct column type not implemented yet");
                }
            }
        }
        if input.is_empty() {
            Ok(RawLayer01 {
                name: layer_name,
                extent,
                id: id_stream.unwrap_or_default(),
                geometry: geometry.ok_or(MltError::MissingGeometry)?,
                properties,
            })
        } else {
            Err(MltError::TrailingLayerData(input.len()))
        }
    }
}

fn parse_columns_meta(
    mut input: &'_ [u8],
    column_count: usize,
) -> MltRefResult<'_, (Vec<Column<'_>>, usize)> {
    #[allow(clippy::enum_glob_use)]
    use ColumnType::*;

    let mut col_info = Vec::with_capacity(column_count);
    let mut geometries = 0;
    let mut ids = 0;
    for _ in 0..column_count {
        let typ;
        (input, typ) = Column::parse(input)?;
        match typ.typ {
            Geometry => geometries += 1,
            Id | OptId | LongId | OptLongId => ids += 1,
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

impl OwnedRawLayer01 {
    /// Write Layer's binary representation to a Write stream without allocating a Vec
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_varint(self.name.len() as u64)?;
        writer.write_all(self.name.as_bytes())?;
        writer.write_varint(u64::from(self.extent))?;
        let has_id = !matches!(self.id, OwnedId::None);
        let column_count = self.properties.len() + usize::from(has_id) + 1;
        writer.write_varint(column_count as u64)?;
        if has_id {
            // self.id.write_to(writer)?;
            todo!()
        }

        todo!()
    }
}
