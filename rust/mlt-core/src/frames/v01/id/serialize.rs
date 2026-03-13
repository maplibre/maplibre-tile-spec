use std::io::Write;

use crate::MltError;
use crate::utils::BinarySerializer as _;
use crate::v01::{ColumnType, EncodedId, EncodedIdValue, StagedId};

impl StagedId {
    /// Return the inner [`EncodedId`] if this is in the `Encoded` state,
    /// or an error if it still needs to be encoded first.
    pub fn as_encoded(&self) -> Result<&EncodedId, MltError> {
        match self {
            Self::Encoded(r) => Ok(r),
            Self::Decoded(_) => Err(MltError::NeedsEncodingBeforeWriting),
        }
    }
}

impl EncodedId {
    pub fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        match (&self.presence, &self.value) {
            (None, EncodedIdValue::Id32(_)) => ColumnType::Id.write_to(writer)?,
            (None, EncodedIdValue::Id64(_)) => ColumnType::LongId.write_to(writer)?,
            (Some(_), EncodedIdValue::Id32(_)) => ColumnType::OptId.write_to(writer)?,
            (Some(_), EncodedIdValue::Id64(_)) => ColumnType::OptLongId.write_to(writer)?,
        }
        Ok(())
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        writer.write_optional_stream(self.presence.as_ref())?;
        match &self.value {
            EncodedIdValue::Id32(s) | EncodedIdValue::Id64(s) => {
                writer.write_stream(s)?;
            }
        }
        Ok(())
    }
}
