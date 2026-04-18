use std::io::Write;

use crate::decoder::StreamMeta;

/// Owned variant of [`RawStream`](crate::decoder::RawStream).
#[derive(Debug, Clone, PartialEq)]
pub struct EncodedStream {
    pub meta: StreamMeta,
    pub(crate) data: Vec<u8>,
}

impl EncodedStream {
    pub fn write_to<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&self.data)
    }
}
