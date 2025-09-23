use borrowme::borrowme;
use nom::Err::Error as NomError;
use nom::IResult;
use nom::error::{Error, ErrorKind};
use num_enum::TryFromPrimitive;

use crate::utils;

/// MVT-compatible feature table data
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct FeatureTable<'a> {
    pub meta: FeatureMetaTable<'a>,
    #[borrowme(borrow_with = Vec::as_slice)]
    pub data: &'a [u8],
}

impl FeatureTable<'_> {
    #[expect(clippy::unnecessary_wraps)]
    pub fn parse<'a>(
        input: &'a [u8],
        meta: FeatureMetaTable<'a>,
    ) -> Result<FeatureTable<'a>, nom::Err<Error<&'a [u8]>>> {
        for column in &meta.columns {
            #[expect(clippy::match_same_arms)]
            match column.typ {
                ColumnType::Id => {
                    // TODO: parse id
                }
                ColumnType::Geometry => {
                    // TODO
                }
                ColumnType::StringProperty => {
                    // TODO
                }
                ColumnType::FloatProperty => {
                    // TODO
                }
                ColumnType::DoubleProperty => {
                    // TODO
                }
                ColumnType::IntProperty => {
                    // TODO
                }
                ColumnType::UintProperty => {
                    // TODO
                }
                ColumnType::SintProperty => {
                    // TODO
                }
                ColumnType::BoolProperty => {
                    // TODO
                }
            }
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
        let name = if typ != ColumnType::Id && typ != ColumnType::Geometry {
            let pair = utils::parse_string(input)?;
            input = pair.0;
            Some(pair.1)
        } else {
            None
        };
        Ok((input, Column { typ, name }))
    }
}

/// Column type enumeration
#[derive(Debug, PartialEq, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
pub enum ColumnType {
    Id = 1,
    // TODO: decide if we need additional geometry types like
    //   PointGeometry/LineGeometry/PolygonGeometry -- if all of them are the same
    //   PreTessellated geometry - if we include additional tessellation data
    Geometry = 2,
    StringProperty = 3,
    FloatProperty = 4,
    DoubleProperty = 5,
    IntProperty = 6,
    UintProperty = 7,
    SintProperty = 8,
    BoolProperty = 9,
}

impl ColumnType {
    /// Parse a column type from u8
    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, value) = utils::parse_u8(input)?;
        let value = Self::try_from(value);
        let value = value.or(Err(NomError(Error::new(input, ErrorKind::Fail))))?;
        Ok((input, value))
    }
}
