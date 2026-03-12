use std::io::Write;

use crate::MltError;
use crate::utils::BinarySerializer as _;
use crate::v01::{ColumnType, OwnedEncodedId, OwnedEncodedIdValue, OwnedId};

impl OwnedId {
    #[doc(hidden)]
    pub fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        match self {
            Self::Encoded(r) => r.write_columns_meta_to(writer),
            Self::Decoded(_) => Err(MltError::NeedsEncodingBeforeWriting),
        }
    }

    #[doc(hidden)]
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        match self {
            Self::Encoded(r) => r.write_to(writer),
            Self::Decoded(_) => Err(MltError::NeedsEncodingBeforeWriting),
        }
    }
}

impl OwnedEncodedId {
    pub(crate) fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        match (&self.presence, &self.value) {
            (None, OwnedEncodedIdValue::Id32(_)) => ColumnType::Id.write_to(writer)?,
            (None, OwnedEncodedIdValue::Id64(_)) => ColumnType::LongId.write_to(writer)?,
            (Some(_), OwnedEncodedIdValue::Id32(_)) => ColumnType::OptId.write_to(writer)?,
            (Some(_), OwnedEncodedIdValue::Id64(_)) => ColumnType::OptLongId.write_to(writer)?,
        }
        Ok(())
    }

    pub(crate) fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        writer.write_optional_stream(self.presence.as_ref())?;
        match &self.value {
            OwnedEncodedIdValue::Id32(s) | OwnedEncodedIdValue::Id64(s) => {
                writer.write_stream(s)?;
            }
        }
        Ok(())
    }
}
