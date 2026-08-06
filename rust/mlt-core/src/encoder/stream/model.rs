#[cfg(any(test, feature = "__private"))]
use std::io::Write;

#[cfg(any(test, feature = "__private"))]
use crate::decoder::StreamMeta;

/// Owned variant of [`RawStream`](crate::decoder::RawStream).
#[cfg(any(test, feature = "__private"))]
#[derive(Debug, Clone, PartialEq)]
pub struct EncodedStream {
    pub meta: StreamMeta,
    pub(crate) data: Vec<u8>,
}

#[cfg(any(test, feature = "__private"))]
impl EncodedStream {
    pub fn write_to<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&self.data)
    }
}
