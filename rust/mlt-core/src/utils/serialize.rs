use std::io;
use std::io::Write;

use integer_encoding::VarIntWriter;

use crate::MltError;
use crate::v01::{OwnedStream, OwnedStreamData};

pub trait BinarySerializer: Write + VarIntWriter {
    fn write_u8(&mut self, value: u8) -> io::Result<()> {
        self.write_all(&[value])
    }
    fn write_string(&mut self, value: &str) -> io::Result<()> {
        let size =
            u64::try_from(value.len()).map_err(|_| io::Error::other(MltError::IntegerOverflow))?;
        self.write_varint(size)?;
        self.write_all(value.as_bytes())
    }

    /// Reverses [`Stream::parse`](mlt::v01::stream::Stream::parse)
    fn write_stream(&mut self, stream: &OwnedStream) -> io::Result<()>
    where
        Self: Sized,
    {
        let byte_length = match &stream.data {
            OwnedStreamData::VarInt(d) => d.data.len(),
            OwnedStreamData::Raw(r) => r.data.len(),
        };
        let byte_length =
            u32::try_from(byte_length).map_err(|_| io::Error::other(MltError::IntegerOverflow))?;
        stream.meta.write_to(self, false, byte_length)?;
        stream.data.write_to(self)?;
        Ok(())
    }
    /// Reverses [`Stream::parse_bool`](mlt::v01::stream::Stream::parse_bool)
    fn write_boolean_stream(&mut self, stream: &OwnedStream) -> io::Result<()>
    where
        Self: Sized,
    {
        let byte_length = match &stream.data {
            OwnedStreamData::VarInt(d) => d.data.len(),
            OwnedStreamData::Raw(r) => r.data.len(),
        };
        let byte_length =
            u32::try_from(byte_length).map_err(|_| io::Error::other(MltError::IntegerOverflow))?;
        stream.meta.write_to(self, true, byte_length)?;
        stream.data.write_to(self)?;
        Ok(())
    }
}

impl<T> BinarySerializer for T where T: Write + VarIntWriter {}
