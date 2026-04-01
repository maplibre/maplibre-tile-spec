use std::io::Write;

use super::model::{EncodedId, EncodedIdValue};
use crate::MltResult;
use crate::utils::BinarySerializer as _;
use crate::v01::ColumnType;

impl EncodedId {
    pub fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> MltResult<()> {
        match (&self.presence, &self.value) {
            (None, EncodedIdValue::Id32(_)) => ColumnType::Id.write_to(writer)?,
            (None, EncodedIdValue::Id64(_)) => ColumnType::LongId.write_to(writer)?,
            (Some(_), EncodedIdValue::Id32(_)) => ColumnType::OptId.write_to(writer)?,
            (Some(_), EncodedIdValue::Id64(_)) => ColumnType::OptLongId.write_to(writer)?,
        }
        Ok(())
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> MltResult<()> {
        writer.write_optional_stream(self.presence.as_ref())?;
        match &self.value {
            EncodedIdValue::Id32(s) | EncodedIdValue::Id64(s) => {
                writer.write_stream(s)?;
            }
        }
        Ok(())
    }
}
