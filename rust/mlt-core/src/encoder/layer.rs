use integer_encoding::VarIntWriter as _;

use crate::MltResult;
use crate::encoder::Encoder;
use crate::encoder::model::EncodedLayer;
use crate::utils::{BinarySerializer as _, checked_sum2};

impl EncodedLayer {
    /// Serialize an unknown layer record directly to [`enc.data`](Encoder::data).
    ///
    /// Writes the complete `[varint(size)][tag][value]` record — the bytes are
    /// already in wire format so no `hdr`/`meta` split is needed.
    pub fn write_to(&self, enc: &mut Encoder) -> MltResult<()> {
        match self {
            Self::Unknown(unknown) => {
                let buffer_len = u32::try_from(unknown.value.len())?;
                let size = checked_sum2(buffer_len, 1)?;
                enc.write_varint(size)?;
                enc.write_u8(unknown.tag)?;
                enc.data.extend_from_slice(&unknown.value);
                Ok(())
            }
        }
    }
}
