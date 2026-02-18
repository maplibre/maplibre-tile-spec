use std::io;
use std::io::Write;

use borrowme::borrowme;
use num_enum::TryFromPrimitive;

use crate::MltError::ParsingColumnType;
use crate::utils::BinarySerializer as _;
use crate::{MltRefResult, utils};

/// Column definition
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct Column<'a> {
    pub typ: ColumnType,
    pub name: Option<&'a str>,
    pub children: Vec<Column<'a>>,
}

impl Column<'_> {
    /// Parse a single column definition
    pub fn parse(input: &[u8]) -> MltRefResult<'_, Column<'_>> {
        let (mut input, typ) = ColumnType::parse(input)?;
        let name = if typ.has_name() {
            let pair = utils::parse_string(input)?;
            input = pair.0;
            Some(pair.1)
        } else {
            None
        };

        Ok((
            input,
            Column {
                typ,
                name,
                children: Vec::new(),
            },
        ))
    }
}

/// Column data type, as stored in the tile
#[derive(Debug, Clone, Copy, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum ColumnType {
    Id = 0,
    OptId = 1,
    LongId = 2,
    OptLongId = 3,
    Geometry = 4,
    Bool = 10,
    OptBool = 11,
    I8 = 12,
    OptI8 = 13,
    U8 = 14,
    OptU8 = 15,
    I32 = 16,
    OptI32 = 17,
    U32 = 18,
    OptU32 = 19,
    I64 = 20,
    OptI64 = 21,
    U64 = 22,
    OptU64 = 23,
    F32 = 24,
    OptF32 = 25,
    F64 = 26,
    OptF64 = 27,
    Str = 28,
    OptStr = 29,
    Struct = 30,
}

impl ColumnType {
    /// Parse a column type from u8
    pub fn parse(input: &[u8]) -> MltRefResult<'_, Self> {
        let (input, value) = utils::parse_u8(input)?;
        let value = Self::try_from(value).or(Err(ParsingColumnType(value)))?;
        Ok((input, value))
    }
    pub fn write_to<W: Write>(self, writer: &mut W) -> io::Result<()> {
        writer.write_u8(self as u8)?;
        Ok(())
    }

    /// Returns true if the column definition includes a name field in the serialized format.
    /// Note: ID and Geometry columns use implicit naming and do not include a name field.
    #[must_use]
    pub fn has_name(self) -> bool {
        !matches!(
            self,
            ColumnType::Id
                | ColumnType::OptId
                | ColumnType::LongId
                | ColumnType::OptLongId
                | ColumnType::Geometry
        )
    }

    /// Check if the column type has a presence stream
    #[must_use]
    pub fn is_optional(self) -> bool {
        (self as u8) & 1 != 0
    }
}
