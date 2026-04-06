use std::io;
use std::io::Write;

use crate::MltError::ParsingColumnType;
use crate::decoder::{Column, ColumnType};
use crate::encoder::IdWidth;
use crate::utils::{BinarySerializer as _, parse_string, parse_u8};
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

    pub fn write_one_of<W: Write>(
        is_opt: bool,
        opt: Self,
        non_opt: Self,
        writer: &mut W,
    ) -> io::Result<()> {
        let col_type = if is_opt { opt } else { non_opt };
        col_type.write_to(writer)
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
            Self::Id | Self::OptId | Self::LongId | Self::OptLongId | Self::Geometry
        )
    }

    /// Check if the column type has a presence stream
    #[must_use]
    pub fn is_optional(self) -> bool {
        (self as u8) & 1 != 0
    }
}

impl From<IdWidth> for ColumnType {
    fn from(value: IdWidth) -> Self {
        match value {
            IdWidth::Id32 => Self::Id,
            IdWidth::OptId32 => Self::OptId,
            IdWidth::Id64 => Self::LongId,
            IdWidth::OptId64 => Self::OptLongId,
        }
    }
}
