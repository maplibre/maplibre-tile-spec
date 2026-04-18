use std::io;
use std::io::Write;

use integer_encoding::VarIntWriter;

use crate::encoder::EncodedStream;
use crate::{MltError, MltResult};

pub trait BinarySerializer: Write + VarIntWriter + Sized {
    fn write_u8(&mut self, value: u8) -> io::Result<()> {
        self.write_all(&[value])
    }
    fn write_string(&mut self, value: &str) -> io::Result<()> {
        let size = u32::try_from(value.len()).map_err(MltError::from)?;
        self.write_varint(size)?;
        self.write_all(value.as_bytes())
    }

    /// Reverses `RawStream::from_bytes` — writes a stream header then the stream data bytes.
    fn write_stream(&mut self, stream: &EncodedStream) -> io::Result<()> {
        let byte_length = u32::try_from(stream.data.len()).map_err(MltError::from)?;
        stream.meta.write_to(self, false, byte_length)?;
        stream.write_to(self)?;
        Ok(())
    }

    /// Serializes an optional stream, which is a stream with boolean values indicating presence of values in another stream.
    fn write_optional_stream(&mut self, stream: Option<&EncodedStream>) -> io::Result<()> {
        if let Some(s) = stream {
            self.write_boolean_stream(s)
        } else {
            Ok(())
        }
    }

    /// Reverses `RawStream::parse_bool` — writes a boolean stream header then the stream data bytes.
    fn write_boolean_stream(&mut self, stream: &EncodedStream) -> io::Result<()> {
        let byte_length = u32::try_from(stream.data.len()).map_err(MltError::from)?;
        stream.meta.write_to(self, true, byte_length)?;
        stream.write_to(self)?;
        Ok(())
    }
}

impl<T> BinarySerializer for T where T: Write + VarIntWriter {}

pub fn strings_to_lengths<S: AsRef<str>>(values: &[S]) -> MltResult<Vec<u32>> {
    Ok(values
        .iter()
        .map(|s| u32::try_from(s.as_ref().len()))
        .collect::<Result<Vec<_>, _>>()?)
}
