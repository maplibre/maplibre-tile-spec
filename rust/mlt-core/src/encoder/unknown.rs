use integer_encoding::VarIntWriter as _;

use crate::MltResult;
use crate::decoder::Unknown;
use crate::encoder::Encoder;
use crate::encoder::model::EncodedUnknown;
use crate::utils::{BinarySerializer as _, checked_sum2};

impl EncodedUnknown {
    /// Serialize an unknown layer record directly to [`enc.data`](Encoder::data).
    ///
    /// Writes the complete `[varint(size)][tag][value]` record — the bytes are
    /// already in wire format so no `hdr`/`meta` split is needed.
    pub fn write_to(&self, mut enc: Encoder) -> MltResult<Encoder> {
        let buffer_len = u32::try_from(self.value.len())?;
        let size = checked_sum2(buffer_len, 1)?;
        enc.write_varint(size)?;
        enc.write_u8(self.tag)?;
        enc.data.extend_from_slice(&self.value);
        Ok(enc)
    }
}

impl<'a> From<Unknown<'a>> for EncodedUnknown {
    fn from(u: Unknown<'a>) -> Self {
        Self {
            tag: u.tag,
            value: u.value.to_vec(),
        }
    }
}

#[cfg(all(not(test), feature = "arbitrary"))]
impl arbitrary::Arbitrary<'_> for EncodedUnknown {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let mut tag: u8 = u.arbitrary()?;
        // Tag 1 is the known Tag01 format; producing it as Unknown would break round-trip-ability
        if tag == 1 {
            tag = 0;
        }
        Ok(Self {
            tag,
            value: u.arbitrary()?,
        })
    }
}
