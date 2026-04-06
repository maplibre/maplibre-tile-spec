use std::fmt;

use super::RawStreamData;
use crate::utils::formatter::fmt_byte_array;

impl RawStreamData<'_> {
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            RawStreamData::VarInt(d) | RawStreamData::Encoded(d) => d,
        }
    }
}
impl fmt::Debug for RawStreamData<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RawStreamData::VarInt(d) | RawStreamData::Encoded(d) => fmt_byte_array(d, f),
        }
    }
}
