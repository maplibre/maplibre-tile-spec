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

/// The `VarInt` and `Encoded` variants are semantically identical at runtime
/// (both wrap a `Vec<u8>` of wire bytes); the distinction exists only to allow
/// test assertions to verify that the correct physical encoding was chosen.
/// In production code, all streams are written directly to the encoder buffer
/// and only `Encoded` is constructed; `VarInt` is constructed in tests.
#[derive(PartialEq, Clone)]
pub enum EncodedStreamData {
    #[allow(dead_code)]
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
