use std::io;

use integer_encoding::VarIntWriter;

use crate::MltError;
use crate::v01::OwnedStream;

pub trait BinarySerializer: io::Write + VarIntWriter {
    fn write_u8(&mut self, value: u8) -> io::Result<()> {
        self.write_all(&[value])
    }
    fn write_string(&mut self, value: &str) -> io::Result<()> {
        let size =
            u64::try_from(value.len()).map_err(|_| io::Error::other(MltError::IntegerOverflow))?;
        self.write_varint(size)?;
        self.write_all(value.as_bytes())
    }
    /// Reverses [`parse_optional`](mlt_nom::v01::root::parse_optional)
    fn write_optional(&mut self, opt: &OwnedStream) -> io::Result<()> {
        Err(io::Error::other(
            MltError::NotImplemented("write_optional").to_string(),
        ))
    }
    /// Reverses [`Stream::parse`](mlt_nom::v01::stream::Stream::parse)
    fn write_stream(&mut self, stream: &OwnedStream) -> io::Result<()> {
        Err(io::Error::other(
            MltError::NotImplemented("write_optional").to_string(),
        ))
    }
    fn write_boolean_stream(&mut self, bool_stream: &OwnedStream) -> io::Result<()> {
        Err(io::Error::other(
            MltError::NotImplemented("write_boolean_stream").to_string(),
        ))
    }
}

impl<T> BinarySerializer for T where T: io::Write + VarIntWriter {}
