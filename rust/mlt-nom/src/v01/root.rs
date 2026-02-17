use std::io;
use std::io::Write;

use borrowme::borrowme;
use integer_encoding::VarIntWriter as _;

use crate::utils::SetOptionOnce as _;
use crate::v01::column::ColumnType;
use crate::v01::{Column, Geometry, Id, OwnedId, Property, RawIdValue, RawPropValue, Stream};
use crate::{Decodable as _, MltError, MltRefResult, utils};

/// Representation of a feature table layer encoded as MLT tag `0x01`
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct Layer01<'a> {
    pub name: &'a str,
    pub extent: u32,
    pub id: Id<'a>,
    pub geometry: Geometry<'a>,
    pub properties: Vec<Property<'a>>,
}

impl Layer01<'_> {
    /// Parse `v01::Layer` metadata
    pub fn parse(input: &[u8]) -> Result<Layer01<'_>, MltError> {
        let (input, layer_name) = utils::parse_string(input)?;
        let (input, extent) = utils::parse_varint::<u32>(input)?;
        let (input, column_count) = utils::parse_varint::<usize>(input)?;

        // Each column requires at least 1 byte (column type)
        if input.len() < column_count {
            return Err(MltError::BufferUnderflow {
                needed: column_count,
                remaining: input.len(),
            });
        }

        // !!!!!!!
        // WARNING: make sure to never use `let (input, ...)` after this point: input var is reused
        let (mut input, (col_info, prop_count)) = parse_columns_meta(input, column_count)?;

        let mut properties = Vec::with_capacity(prop_count);
        let mut id_stream: Option<Id> = None;
        let mut geometry: Option<Geometry> = None;

        for column in col_info {
            let optional;
            let value;
            let mut stream_count;
            let name = column.name.unwrap_or("");

            match column.typ {
                ColumnType::Id | ColumnType::OptId => {
                    (input, optional) = parse_optional(column.typ, input)?;
                    (input, value) = Stream::parse(input)?;
                    id_stream.set_once(Id::raw(optional, RawIdValue::Id32(value)))?;
                }
                ColumnType::LongId | ColumnType::OptLongId => {
                    (input, optional) = parse_optional(column.typ, input)?;
                    (input, value) = Stream::parse(input)?;
                    id_stream.set_once(Id::raw(optional, RawIdValue::Id64(value)))?;
                }
                ColumnType::Geometry => {
                    let value_vec;
                    (input, stream_count) = utils::parse_varint::<usize>(input)?;
                    // Each stream requires at least 1 byte (physical stream type)
                    if input.len() < stream_count {
                        return Err(MltError::BufferUnderflow {
                            needed: stream_count,
                            remaining: input.len(),
                        });
                    }
                    if stream_count == 0 {
                        return Err(MltError::MinLength {
                            ctx: "geometry type, but without streams",
                            min: 1,
                            got: 0,
                        });
                    }

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
                    // Each stream requires at least 1 byte (physical stream type)
                    if input.len() < stream_count {
                        return Err(MltError::BufferUnderflow {
                            needed: stream_count,
                            remaining: input.len(),
                        });
                    }
                    if stream_count > 0 {
                        (input, optional) = parse_optional(column.typ, input)?;
                    } else {
                        optional = None;
                    }
                    stream_count -= usize::from(optional.is_some());
                    let value_vec;
                    (input, value_vec) = Stream::parse_multiple(input, stream_count)?;
                    properties.push(Property::raw(name, optional, RawPropValue::Str(value_vec)));
                }
                ColumnType::Struct => {
                    return Err(MltError::NotImplemented("Struct column type"));
                }
            }
        }
        if input.is_empty() {
            Ok(Layer01 {
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

    pub fn decode_all(&mut self) -> Result<(), MltError> {
        self.id.materialize()?;
        self.geometry.materialize()?;
        for prop in &mut self.properties {
            prop.materialize()?;
        }
        Ok(())
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

fn parse_columns_meta(
    mut input: &'_ [u8],
    column_count: usize,
) -> MltRefResult<'_, (Vec<Column<'_>>, usize)> {
    #[allow(clippy::enum_glob_use)]
    use crate::v01::column::ColumnType::*;

    let mut col_info = Vec::with_capacity(column_count);
    let mut geometries = 0;
    let mut ids = 0;
    for _ in 0..column_count {
        let mut typ;
        (input, typ) = Column::parse(input)?;
        match typ.typ {
            Geometry => geometries += 1,
            Id | OptId | LongId | OptLongId => ids += 1,
            Struct => {
                // Yes, we need to parse children right here, otherwise this messes up the next column
                let child_column_count;
                (input, child_column_count) = utils::parse_varint::<usize>(input)?;

                // Each collumn requires at least 1 byte (ColumnType without name)
                if input.len() < child_column_count {
                    return Err(MltError::BufferUnderflow {
                        needed: child_column_count,
                        remaining: input.len(),
                    });
                }
                let mut children = Vec::with_capacity(child_column_count);
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
        writer.write_varint(self.name.len() as u64)?;
        writer.write_all(self.name.as_bytes())?;
        writer.write_varint(u64::from(self.extent))?;
        let has_id = !matches!(self.id, OwnedId::None);
        let column_count = self.properties.len() + usize::from(has_id) + 1;
        writer.write_varint(column_count as u64)?;
        if has_id {
            // self.id.write_to(writer)?;
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                MltError::NotImplemented("ID write").to_string(),
            ));
        }

        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            MltError::NotImplemented("full Layer write").to_string(),
        ))
    }
}
