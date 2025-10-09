use borrowme::borrowme;
use nom::bytes::complete::take;
use nom::error::Error;
use nom::{IResult, Parser};

use crate::structures::complex_enums::{ColumnStreams, PhysicalStreamType};
use crate::structures::enums::{ColumnType, LogicalTechnique, PhysicalTechnique};
use crate::utils;
use crate::utils::{fail, parse_u7, parse_varint_vec};

/// MVT-compatible feature table data
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct Stream<'a> {
    pub physical_stream_type: PhysicalStreamType,
    pub logical_technique1: LogicalTechnique,
    pub logical_technique2: LogicalTechnique,
    pub physical_technique: PhysicalTechnique,
    pub num_values: u32,
    #[borrowme(borrow_with = Vec::as_slice)]
    pub data: &'a [u8],
}

impl Stream<'_> {
    fn parse(input: &[u8]) -> IResult<&[u8], Stream<'_>> {
        let (input, val) = utils::parse_u8(input)?;
        let physical_stream_type = PhysicalStreamType::from_u8(val).ok_or(fail(input))?;

        let (input, val) = utils::parse_u8(input)?;
        let logical_technique1 = LogicalTechnique::try_from(val >> 5).or(Err(fail(input)))?;
        let logical_technique2 =
            LogicalTechnique::try_from((val >> 2) & 0x7).or(Err(fail(input)))?;
        let physical_technique = PhysicalTechnique::try_from(val & 0x3).or(Err(fail(input)))?;

        let (input, num_values) = utils::parse_varint::<u32>(input)?;
        let (input, byte_length) = utils::parse_varint::<u32>(input)?;
        let (input, data) = take(byte_length).parse(input)?;

        Ok((
            input,
            Stream {
                physical_stream_type,
                logical_technique1,
                logical_technique2,
                physical_technique,
                num_values,
                data,
            },
        ))
    }

    pub fn _decode<'a>(&self, input: &'a [u8]) -> IResult<&'a [u8], Vec<u32>> {
        match self.physical_stream_type {
            PhysicalStreamType::Present => {
                todo!()
            }
            PhysicalStreamType::Data(_v) => parse_varint_vec(input, self.num_values as usize),
            PhysicalStreamType::Offset(_v) => {
                todo!()
            }
            PhysicalStreamType::Length(_v) => {
                todo!()
            }
        }
    }
}

/// MVT-compatible feature table data
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct FeatureTable<'a> {
    pub name: &'a str,
    pub extent: u32,
    pub columns: Vec<ColumnStreams<'a>>,
}

impl FeatureTable<'_> {
    /// Parse `FeatureTable` V1 metadata
    pub fn parse(mut input: &[u8]) -> Result<FeatureTable<'_>, nom::Err<Error<&[u8]>>> {
        let name;
        let extent;
        let column_count;

        (input, name) = utils::parse_string(input)?;
        (input, extent) = utils::parse_varint::<u32>(input)?;
        (input, column_count) = utils::parse_varint::<usize>(input)?;

        let mut col_info = Vec::with_capacity(column_count);
        for _ in 0..column_count {
            let typ;
            (input, typ) = Column::parse(input)?;
            col_info.push(typ);
        }

        let mut columns = Vec::with_capacity(col_info.len());
        for info in col_info {
            let opt;
            let val;
            let nam = info.name.unwrap_or("");
            match info.typ {
                ColumnType::Id => {
                    (input, val) = Stream::parse(input)?;
                    columns.push(ColumnStreams::Id(val));
                }
                ColumnType::OptId => {
                    (input, (opt, val)) = parse_pair(input)?;
                    columns.push(ColumnStreams::OptId(opt, val));
                }
                ColumnType::LongId => {
                    (input, val) = Stream::parse(input)?;
                    columns.push(ColumnStreams::LongId(val));
                }
                ColumnType::OptLongId => {
                    (input, (opt, val)) = parse_pair(input)?;
                    columns.push(ColumnStreams::OptLongId(opt, val));
                }
                ColumnType::Geometry => {
                    let stream_count;
                    // let geom_metadata;
                    (input, stream_count) = parse_u7(input)?;
                    // (input, geom_metadata) = Stream::parse(input)?;
                    let mut vec = Vec::with_capacity(stream_count as usize);
                    for _ in 0..stream_count {
                        let stream;
                        (input, stream) = Stream::parse(input)?;
                        vec.push(stream);
                    }
                    columns.push(ColumnStreams::Geometry(vec));
                }
                ColumnType::Bool => {
                    (input, val) = Stream::parse(input)?;
                    columns.push(ColumnStreams::Bool(nam, val));
                }
                ColumnType::OptBool => {
                    (input, (opt, val)) = parse_pair(input)?;
                    columns.push(ColumnStreams::OptBool(nam, opt, val));
                }
                ColumnType::I8 => {
                    (input, val) = Stream::parse(input)?;
                    columns.push(ColumnStreams::I8(nam, val));
                }
                ColumnType::OptI8 => {
                    (input, (opt, val)) = parse_pair(input)?;
                    columns.push(ColumnStreams::OptI8(nam, opt, val));
                }
                ColumnType::U8 => {
                    (input, val) = Stream::parse(input)?;
                    columns.push(ColumnStreams::U8(nam, val));
                }
                ColumnType::OptU8 => {
                    (input, (opt, val)) = parse_pair(input)?;
                    columns.push(ColumnStreams::OptU8(nam, opt, val));
                }
                ColumnType::I32 => {
                    (input, val) = Stream::parse(input)?;
                    columns.push(ColumnStreams::I32(nam, val));
                }
                ColumnType::OptI32 => {
                    (input, (opt, val)) = parse_pair(input)?;
                    columns.push(ColumnStreams::OptI32(nam, opt, val));
                }
                ColumnType::U32 => {
                    (input, val) = Stream::parse(input)?;
                    columns.push(ColumnStreams::U32(nam, val));
                }
                ColumnType::OptU32 => {
                    (input, (opt, val)) = parse_pair(input)?;
                    columns.push(ColumnStreams::OptU32(nam, opt, val));
                }
                ColumnType::I64 => {
                    (input, val) = Stream::parse(input)?;
                    columns.push(ColumnStreams::I64(nam, val));
                }
                ColumnType::OptI64 => {
                    (input, (opt, val)) = parse_pair(input)?;
                    columns.push(ColumnStreams::OptI64(nam, opt, val));
                }
                ColumnType::U64 => {
                    (input, val) = Stream::parse(input)?;
                    columns.push(ColumnStreams::U64(nam, val));
                }
                ColumnType::OptU64 => {
                    (input, (opt, val)) = parse_pair(input)?;
                    columns.push(ColumnStreams::OptU64(nam, opt, val));
                }
                ColumnType::F32 => {
                    (input, val) = Stream::parse(input)?;
                    columns.push(ColumnStreams::F32(nam, val));
                }
                ColumnType::OptF32 => {
                    (input, (opt, val)) = parse_pair(input)?;
                    columns.push(ColumnStreams::OptF32(nam, opt, val));
                }
                ColumnType::F64 => {
                    (input, val) = Stream::parse(input)?;
                    columns.push(ColumnStreams::F64(nam, val));
                }
                ColumnType::OptF64 => {
                    (input, (opt, val)) = parse_pair(input)?;
                    columns.push(ColumnStreams::OptF64(nam, opt, val));
                }
                ColumnType::Str => {
                    (input, val) = Stream::parse(input)?;
                    columns.push(ColumnStreams::Str(nam, val));
                }
                ColumnType::OptStr => {
                    (input, (opt, val)) = parse_pair(input)?;
                    columns.push(ColumnStreams::OptStr(nam, opt, val));
                }
                ColumnType::Struct => {}
            }
        }
        if input.is_empty() {
            Ok(FeatureTable {
                name,
                extent,
                columns,
            })
        } else {
            Err(fail(input))
        }
    }
}

pub fn parse_pair(input: &[u8]) -> IResult<&[u8], (Stream<'_>, Stream<'_>)> {
    let (input, opt) = Stream::parse(input)?;
    let (input, val) = Stream::parse(input)?;
    Ok((input, (opt, val)))
}

// impl FeatureMetaTable<'_> {
//     /// Parse `FeatureTable` V1 metadata
//     pub fn parse(input: &[u8]) -> IResult<&[u8], FeatureMetaTable<'_>> {
//         let (input, name) = utils::parse_string(input)?;
//         let (input, extent) = utils::parse_varint::<u32>(input)?;
//         let (mut input, column_count) = utils::parse_varint::<usize>(input)?;
//
//         let mut columns = Vec::with_capacity(column_count);
//         for _ in 0..column_count {
//             let pair = Column::parse(input)?;
//             input = pair.0;
//             columns.push(pair.1);
//         }
//
//         Ok((
//             input,
//             FeatureMetaTable {
//                 name,
//                 extent,
//                 columns,
//             },
//         ))
//     }
// }

/// Column definition
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct Column<'a> {
    pub typ: ColumnType,
    pub name: Option<&'a str>,
}

impl Column<'_> {
    /// Parse a single column definition
    fn parse(input: &[u8]) -> IResult<&[u8], Column<'_>> {
        let (mut input, typ) = ColumnType::parse(input)?;
        let name = if typ.has_name() {
            let pair = utils::parse_string(input)?;
            input = pair.0;
            Some(pair.1)
        } else {
            None
        };
        Ok((input, Column { typ, name }))
    }
}
