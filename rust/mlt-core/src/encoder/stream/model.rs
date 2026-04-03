use std::fmt;
use std::io::Write;

use crate::decoder::StreamMeta;
use crate::utils::formatter::fmt_byte_array;

/// Owned variant of [`RawStream`](crate::decoder::RawStream).
#[derive(Debug, PartialEq, Clone)]
pub struct EncodedStream {
    pub meta: StreamMeta,
    pub(crate) data: EncodedStreamData,
}

#[derive(PartialEq, Clone)]
pub enum EncodedStreamData {
    VarInt(Vec<u8>),
    Encoded(Vec<u8>),
}
impl EncodedStreamData {
    pub fn write_to<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        match self {
            Self::VarInt(d) | Self::Encoded(d) => writer.write_all(d),
        }
    }
}
impl fmt::Debug for EncodedStreamData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::VarInt(d) | Self::Encoded(d) => fmt_byte_array(d, f),
        }
    }
}
