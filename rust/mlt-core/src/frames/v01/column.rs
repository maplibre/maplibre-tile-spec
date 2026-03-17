use std::io;
use std::io::Write;

use crate::MltError::ParsingColumnType;
use crate::utils::{BinarySerializer as _, parse_string, parse_u8};
use crate::v01::{Column, ColumnType};
use crate::{MltRefResult, Parser};

impl Column<'_> {
    /// Parse a single column definition
    pub fn from_bytes<'a>(input: &'a [u8], _parser: &mut Parser) -> MltRefResult<'a, Column<'a>> {
        let (mut input, typ) = ColumnType::from_bytes(input)?;
        let name = if typ.has_name() {
            let pair = parse_string(input)?;
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

impl ColumnType {
    /// Parse a column type from u8
    pub fn from_bytes(input: &[u8]) -> MltRefResult<'_, Self> {
        let (input, value) = parse_u8(input)?;
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
