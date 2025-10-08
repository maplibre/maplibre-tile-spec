use borrowme::borrowme;
use nom::IResult;
use nom::error::Error;
use num_enum::TryFromPrimitive;

use crate::structures::enums::{ColumnType, LogicalLevelTechnique, PhysicalLevelTechnique, PhysicalStreamType};
use crate::utils;
use crate::utils::fail;

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
pub struct FeatureStream<'a> {
    pub physical_stream_type: PhysicalStreamType,
    pub logical_level_technique1: LogicalLevelTechnique,
    pub logical_level_technique2: LogicalLevelTechnique,
    pub physical_level_technique: PhysicalLevelTechnique,
    #[borrowme(borrow_with = Vec::as_slice)]
    pub data: &'a [u8],
}

impl FeatureStream<'_> {
    fn parse<'a>(
        input: &'a [u8],
        _column: &'_ Column<'_>,
        _meta: &'_ FeatureMetaTable<'_>,
    ) -> IResult<&'a [u8], FeatureStream<'a>> {
        let (input, val) = utils::parse_u8(input)?;
        let physical_stream_type = PhysicalStreamType::from_u8(val).ok_or(fail(input))?;

        let (input, val) = utils::parse_u8(input)?;
        let logical_level_technique1 =
            LogicalLevelTechnique::try_from(val >> 5).or(Err(fail(input)))?;
        let logical_level_technique2 =
            LogicalLevelTechnique::try_from((val >> 2) & 0x7).or(Err(fail(input)))?;
        let physical_level_technique =
            PhysicalLevelTechnique::try_from(val & 0x3).or(Err(fail(input)))?;

        Ok((
            input,
            FeatureStream {
                physical_stream_type,
                logical_level_technique1,
                logical_level_technique2,
                physical_level_technique,
                data: input,
            },
        ))
    }
}

impl FeatureTable<'_> {
    #[expect(clippy::unnecessary_wraps)]
    pub fn parse<'a>(
        mut input: &'a [u8],
        meta: FeatureMetaTable<'a>,
    ) -> Result<FeatureTable<'a>, nom::Err<Error<&'a [u8]>>> {
        for column in &meta.columns {
            let _stream_count = if column.typ.has_stream_count() {
                let pair = utils::parse_u7(input)?;
                input = pair.0;
                pair.1
            } else {
                1
            };
            if column.typ.is_optional() {
                let _opt = FeatureStream::parse(input, column, &meta)?;
            }
            let _main = FeatureStream::parse(input, column, &meta)?;

            // #[expect(clippy::match_same_arms)]
            // match column.typ {
            //     ColumnType::Id => {}
            //     ColumnType::OptId => {}
            //     ColumnType::LongId => {}
            //     ColumnType::OptLongId => {}
            //     ColumnType::Geometry => {}
            //     ColumnType::Bool => {}
            //     ColumnType::OptBool => {}
            //     ColumnType::I8 => {}
            //     ColumnType::OptI8 => {}
            //     ColumnType::U8 => {}
            //     ColumnType::OptU8 => {}
            //     ColumnType::I32 => {}
            //     ColumnType::OptI32 => {}
            //     ColumnType::U32 => {}
            //     ColumnType::OptU32 => {}
            //     ColumnType::I64 => {}
            //     ColumnType::OptI64 => {}
            //     ColumnType::U64 => {}
            //     ColumnType::OptU64 => {}
            //     ColumnType::F32 => {}
            //     ColumnType::OptF32 => {}
            //     ColumnType::F64 => {}
            //     ColumnType::OptF64 => {}
            //     ColumnType::Str => {}
            //     ColumnType::OptStr => {}
            //     ColumnType::Struct => {}
            // }
        }

        Ok(FeatureTable { meta, data: input })
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
        let (input, extent) = utils::parse_varint_u32(input)?;
        let (mut input, column_count) = utils::parse_varint_usize(input)?;

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
