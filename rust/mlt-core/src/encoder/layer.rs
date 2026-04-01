use std::borrow::Cow;
use std::io;
use std::io::Write;

use integer_encoding::VarIntWriter as _;

use crate::MltError;
use crate::encoder::model::EncodedLayer;
use crate::utils::{BinarySerializer as _, checked_sum2};

impl EncodedLayer {
    /// Write layer's binary representation to a Write stream
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let (tag, buffer) = match self {
            Self::Tag01(layer) => {
                let mut buffer = Vec::new();
                layer.write_to(&mut buffer)?;
                (1_u8, Cow::Owned(buffer))
            }
            Self::Unknown(unknown) => (unknown.tag, Cow::Borrowed(&unknown.value)),
        };

        let buffer_len = u32::try_from(buffer.len()).map_err(MltError::from)?;
        let size = checked_sum2(buffer_len, 1)?;
        writer.write_varint(size)?;
        writer.write_u8(tag)?;
        writer.write_all(&buffer)
    }
}
