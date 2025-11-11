use std::fmt::{Debug, Formatter};

use borrowme::borrowme;

use crate::MltError;
use crate::decodable::{FromRaw, impl_decodable};
use crate::v01::Stream;

/// ID column representation, either raw or decoded, or none if there are no IDs
#[borrowme]
#[derive(Debug, Default, PartialEq)]
pub enum Id<'a> {
    #[default]
    None,
    Raw(RawId<'a>),
    Decoded(DecodedId),
}

/// Unparsed ID data as read directly from the tile
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct RawId<'a> {
    optional: Option<Stream<'a>>,
    value: RawIdValue<'a>,
}

/// A sequence of encoded (raw) ID values, either 32-bit or 64-bit unsigned integers
#[borrowme]
#[derive(Debug, PartialEq)]
pub enum RawIdValue<'a> {
    Id32(Stream<'a>),
    Id64(Stream<'a>),
}

/// Decoded ID values as a vector of optional 64-bit unsigned integers
#[derive(Clone, Default, PartialEq)]
pub struct DecodedId(Option<Vec<Option<u64>>>);

impl_decodable!(Id<'a>, RawId<'a>, DecodedId);

impl Debug for DecodedId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            None => write!(f, "DecodedId(None)"),
            Some(ids) => {
                let preview: Vec<String> = ids
                    .iter()
                    .take(8)
                    .map(|opt_id| {
                        opt_id
                            .map(|id| id.to_string())
                            .unwrap_or_else(|| "None".to_string())
                    })
                    .collect();
                write!(
                    f,
                    "DecodedId([{}{}; {}])",
                    preview.join(","),
                    if ids.len() > 8 { ", ..." } else { "" },
                    ids.len()
                )
            }
        }
    }
}

impl<'a> From<RawId<'a>> for Id<'a> {
    fn from(value: RawId<'a>) -> Self {
        Self::Raw(value)
    }
}

impl<'a> Id<'a> {
    #[must_use]
    pub fn raw(optional: Option<Stream<'a>>, value: RawIdValue<'a>) -> Self {
        Self::Raw(RawId { optional, value })
    }

    #[inline]
    pub fn decode(self) -> Result<DecodedId, MltError> {
        Ok(match self {
            Self::Raw(v) => DecodedId::from_raw(v)?,
            Self::Decoded(v) => v,
            Self::None => DecodedId(None),
        })
    }
}

impl<'a> FromRaw<'a> for DecodedId {
    type Input = RawId<'a>;

    fn from_raw(RawId { optional: _, value }: RawId<'_>) -> Result<Self, MltError> {
        // Note: The optional/present stream is ignored for ID columns (following C++ implementation)
        // The ID stream contains all IDs directly

        match value {
            RawIdValue::Id32(stream) => {
                // Decode 32-bit IDs as u32, then convert to u64
                let ids: Vec<u32> = stream.decode_bits_u32()?.decode_u32()?;
                let ids_u64: Vec<Option<u64>> =
                    ids.into_iter().map(|id| Some(u64::from(id))).collect();
                Ok(DecodedId(Some(ids_u64)))
            }
            RawIdValue::Id64(stream) => {
                // Decode 64-bit IDs directly as u64
                let ids: Vec<u64> = stream.decode_u64()?;
                let ids_u64: Vec<Option<u64>> = ids.into_iter().map(Some).collect();
                Ok(DecodedId(Some(ids_u64)))
            }
        }
    }
}
