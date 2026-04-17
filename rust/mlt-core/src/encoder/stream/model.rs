use std::fmt;
use std::io::Write;

use crate::decoder::StreamMeta;
use crate::utils::formatter::fmt_byte_array;

/// Owned variant of [`RawStream`](crate::decoder::RawStream).
#[derive(PartialEq, Clone)]
pub struct EncodedStream {
    pub meta: StreamMeta,
    pub(crate) data: Vec<u8>,
}

impl EncodedStream {
    pub fn write_to<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&self.data)
    }
}

impl fmt::Debug for EncodedStream {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt_byte_array(&self.data, f)
    }
}
