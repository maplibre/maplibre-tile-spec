//! Column model and parsing specific to tag `0x01` (v1) layers.
//!
//! v2 replaces both types below with `model02::ColumnType02` and
//! `model02::GeoLayout`; the resulting [`super::Layer01`] is shared.

use std::io;
use std::io::Write;

use num_enum::TryFromPrimitive;

use crate::MltError::ParsingColumnType;
use crate::utils::{BinarySerializer as _, parse_string, parse_u8};
use crate::{MltRefResult, Parser};

/// Bit 0 of the column type byte: the column has a presence stream.
const OPTIONAL_FLAG: u8 = 0b0000_0001;

/// Column definition
#[derive(Debug, PartialEq)]
pub struct Column<'a> {
    pub(crate) typ: ColumnType,
    pub(crate) name: Option<&'a str>,
    pub(crate) children: Vec<Self>,
}

/// Column data type, as stored in the tile.
#[derive(Debug, Clone, Copy, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum ColumnType {
    Id = 0b0000_0000,
    OptId = 0b0000_0001,
    LongId = 0b0000_0010,
    OptLongId = 0b0000_0011,
    Geometry = 0b0000_0100,
    Bool = 0b0000_1010,
    OptBool = 0b0000_1011,
    I8 = 0b0000_1100,
    OptI8 = 0b0000_1101,
    U8 = 0b0000_1110,
    OptU8 = 0b0000_1111,
    I32 = 0b0001_0000,
    OptI32 = 0b0001_0001,
    U32 = 0b0001_0010,
    OptU32 = 0b0001_0011,
    I64 = 0b0001_0100,
    OptI64 = 0b0001_0101,
    U64 = 0b0001_0110,
    OptU64 = 0b0001_0111,
    F32 = 0b0001_1000,
    OptF32 = 0b0001_1001,
    F64 = 0b0001_1010,
    OptF64 = 0b0001_1011,
    Str = 0b0001_1100,
    OptStr = 0b0001_1101,
    SharedDict = 0b0001_1110,
}

impl Column<'_> {
    /// Parse a single column definition
    pub(crate) fn from_bytes<'a>(
        input: &'a [u8],
        _parser: &mut Parser,
    ) -> MltRefResult<'a, Column<'a>> {
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
    pub(crate) fn from_bytes(input: &[u8]) -> MltRefResult<'_, Self> {
        let (input, value) = parse_u8(input)?;
        let value = Self::try_from(value).or(Err(ParsingColumnType(value)))?;
        Ok((input, value))
    }

    pub(crate) fn write_to<W: Write>(self, writer: &mut W) -> io::Result<()> {
        writer.write_u8(self as u8)?;
        Ok(())
    }

    /// Returns true if the column definition includes a name field in the serialized format.
    /// Note: ID and Geometry columns use implicit naming and do not include a name field.
    #[must_use]
    pub(crate) fn has_name(self) -> bool {
        !matches!(
            self,
            Self::Id | Self::OptId | Self::LongId | Self::OptLongId | Self::Geometry
        )
    }

    /// Check if the column type has a presence stream
    #[must_use]
    pub(crate) fn is_optional(self) -> bool {
        (self as u8) & OPTIONAL_FLAG != 0
    }
}
