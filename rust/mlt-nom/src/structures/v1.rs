use borrowme::borrowme;
use nom::IResult;
use nom::error::Error;

use crate::structures::complex_enums::PhysicalStreamType;
use crate::structures::enums::{ColumnType, LogicalTechnique, PhysicalTechnique};
use crate::utils;
use crate::utils::{fail, parse_varint_vec};

/// MVT-compatible feature table data
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct FeatureTable<'a> {
    pub meta: FeatureMetaTable<'a>,
    #[borrowme(borrow_with = Vec::as_slice)]
    pub data: &'a [u8],
}

/// MVT-compatible feature table data
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct Stream<'a> {
    pub physical_stream_type: PhysicalStreamType,
    pub logical_technique1: LogicalTechnique,
    pub logical_technique2: LogicalTechnique,
    pub physical_technique: PhysicalTechnique,
    pub num_values: u32,
    pub byte_length: u32,
    #[borrowme(borrow_with = Vec::as_slice)]
    pub data: &'a [u8],
}

impl Stream<'_> {
    fn parse<'a>(
        input: &'a [u8],
        _column: &'_ Column<'_>,
        _meta: &'_ FeatureMetaTable<'_>,
    ) -> IResult<&'a [u8], Stream<'a>> {
        let (input, val) = utils::parse_u8(input)?;
        let physical_stream_type = PhysicalStreamType::from_u8(val).ok_or(fail(input))?;

        let (input, val) = utils::parse_u8(input)?;
        let logical_technique1 = LogicalTechnique::try_from(val >> 5).or(Err(fail(input)))?;
        let logical_technique2 =
            LogicalTechnique::try_from((val >> 2) & 0x7).or(Err(fail(input)))?;
        let physical_technique = PhysicalTechnique::try_from(val & 0x3).or(Err(fail(input)))?;

        let (input, num_values) = utils::parse_varint::<u32>(input)?;
        let (input, byte_length) = utils::parse_varint::<u32>(input)?;

        Ok((
            input,
            Stream {
                physical_stream_type,
                logical_technique1,
                logical_technique2,
                physical_technique,
                num_values,
                byte_length,
                data: input,
            },
        ))
    }

    pub fn decode<'a>(&self, input: &'a [u8]) -> IResult<&'a [u8], Vec<u32>> {
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

impl FeatureTable<'_> {
    #[expect(clippy::unnecessary_wraps)]
    pub fn parse<'a>(
        mut input: &'a [u8],
        tbl_meta: FeatureMetaTable<'a>,
    ) -> Result<FeatureTable<'a>, nom::Err<Error<&'a [u8]>>> {
        for column in &tbl_meta.columns {
            if matches!(
                column.typ,
                ColumnType::Id | ColumnType::OptId | ColumnType::LongId | ColumnType::OptLongId
            ) {
                let _stream_count = if column.typ.has_stream_count() {
                    let pair = utils::parse_u7(input)?;
                    input = pair.0;
                    pair.1
                } else {
                    1
                };
                let bools = if column.typ.is_optional() {
                    let meta;
                    (input, meta) = Stream::parse(input, column, &tbl_meta)?;
                    let bools = meta.decode(input)?;
                    Some(bools)
                } else {
                    None
                };

                let meta;
                (input, meta) = Stream::parse(input, column, &tbl_meta)?;
                let ints;
                (input, ints) = meta.decode(input)?;

                dbg!(bools);
                dbg!(ints);
            }
        }

        Ok(FeatureTable { meta: tbl_meta, data: input })
    }
}

/// `FeatureTable` V1 metadata structure
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct FeatureMetaTable<'a> {
    pub name: &'a str,
    pub extent: u32,
    pub columns: Vec<Column<'a>>,
}

impl FeatureMetaTable<'_> {
    /// Parse `FeatureTable` V1 metadata
    pub fn parse(input: &[u8]) -> IResult<&[u8], FeatureMetaTable<'_>> {
        let (input, name) = utils::parse_string(input)?;
        let (input, extent) = utils::parse_varint::<u32>(input)?;
        let (mut input, column_count) = utils::parse_varint::<usize>(input)?;

        let mut columns = Vec::with_capacity(column_count);
        for _ in 0..column_count {
            let pair = Column::parse(input)?;
            input = pair.0;
            columns.push(pair.1);
        }

        Ok((
            input,
            FeatureMetaTable {
                name,
                extent,
                columns,
            },
        ))
    }
}

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
