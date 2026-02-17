use crate::MltError;
use integer_encoding::VarIntWriter;
use std::io;

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
}

impl<T> BinarySerializer for T where T: io::Write + VarIntWriter {}
