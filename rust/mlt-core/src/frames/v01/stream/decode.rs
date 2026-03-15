use std::mem::size_of;

use crate::errors::{AsMltError as _, fail_if_invalid_stream_size};
use crate::utils::{
    AsUsize as _, all, decode_byte_rle, decode_bytes_to_bools, decode_bytes_to_u32s,
    decode_bytes_to_u64s, decode_fastpfor_composite, parse_varint_vec,
};
use crate::v01::{LogicalData, LogicalValue, PhysicalEncoding, RawStream, RawStreamData};
use crate::{Decoder, MltError};

impl RawStream<'_> {
    /// Decode a boolean stream: byte-RLE → packed bitmap → `Vec<bool>`, charging `dec`.
    pub fn decode_bools(self, dec: &mut Decoder) -> Result<Vec<bool>, MltError> {
        let num_values = self.meta.num_values.as_usize();
        dec.consume(u32::try_from(num_values * size_of::<bool>()).or_overflow()?)?;
        let num_bytes = num_values.div_ceil(8);
        let raw = match &self.data {
            RawStreamData::Encoded(v) => v,
            RawStreamData::VarInt(_) => {
                return Err(MltError::NotImplemented("varint bool decoding"));
            }
        };
        let decoded = decode_byte_rle(raw, num_bytes);
        Ok(decode_bytes_to_bools(&decoded, num_values))
    }

    pub fn decode_i8s(self, dec: &mut Decoder) -> Result<Vec<i8>, MltError> {
        self.decode_i32s(dec)?
            .into_iter()
            .map(i8::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn decode_u8s(self, dec: &mut Decoder) -> Result<Vec<u8>, MltError> {
        self.decode_u32s(dec)?
            .into_iter()
            .map(u8::try_from)
            .collect::<Result<Vec<u8>, _>>()
            .map_err(Into::into)
    }

    pub fn decode_i32s(self, dec: &mut Decoder) -> Result<Vec<i32>, MltError> {
        self.decode_bits_u32(dec)?.decode_i32(dec)
    }

    pub fn decode_u32s(self, dec: &mut Decoder) -> Result<Vec<u32>, MltError> {
        self.decode_bits_u32(dec)?.decode_u32(dec)
    }

    pub fn decode_u64s(self, dec: &mut Decoder) -> Result<Vec<u64>, MltError> {
        self.decode_bits_u64(dec)?.decode_u64(dec)
    }

    pub fn decode_i64s(self, dec: &mut Decoder) -> Result<Vec<i64>, MltError> {
        self.decode_bits_u64(dec)?.decode_i64(dec)
    }

    /// Decode a stream of f32 values from raw little-endian bytes, charging `dec`.
    pub fn decode_f32s(self, dec: &mut Decoder) -> Result<Vec<f32>, MltError> {
        let num = self.meta.num_values.as_usize();
        dec.consume(u32::try_from(num * size_of::<f32>()).or_overflow()?)?;
        let raw = match &self.data {
            RawStreamData::Encoded(v) => v,
            RawStreamData::VarInt(_) => {
                return Err(MltError::NotImplemented("varint f32 decoding"));
            }
        };
        fail_if_invalid_stream_size(raw.len(), num.checked_mul(4).or_overflow()?)?;

        Ok(raw
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes(chunk.try_into().expect("infallible: chunks_exact(4)")))
            .collect())
    }

    /// Decode a stream of f64 values from raw little-endian bytes, charging `dec`.
    pub fn decode_f64s(self, dec: &mut Decoder) -> Result<Vec<f64>, MltError> {
        let num = self.meta.num_values.as_usize();
        dec.consume(u32::try_from(num * size_of::<f64>()).or_overflow()?)?;
        let raw = match &self.data {
            RawStreamData::Encoded(v) => v,
            RawStreamData::VarInt(_) => {
                return Err(MltError::NotImplemented("varint f64 decoding"));
            }
        };
        fail_if_invalid_stream_size(raw.len(), num.checked_mul(8).or_overflow()?)?;

        Ok(raw
            .chunks_exact(8)
            .map(|chunk| f64::from_le_bytes(chunk.try_into().expect("infallible: chunks_exact(8)")))
            .collect())
    }

    /// Decode the physical layer into a `Vec<u32>` logical layer, charging `dec` for the
    /// intermediate allocation.
    pub fn decode_bits_u32(self, dec: &mut Decoder) -> Result<LogicalValue, MltError> {
        let value = match self.meta.encoding.physical {
            PhysicalEncoding::VarInt => match &self.data {
                RawStreamData::VarInt(v) => {
                    all(parse_varint_vec::<u32, u32>(v, self.meta.num_values)?)
                }
                RawStreamData::Encoded(_) => {
                    return Err(MltError::StreamDataMismatch("VarInt", "Encoded"));
                }
            },
            PhysicalEncoding::None => match &self.data {
                RawStreamData::Encoded(v) => all(decode_bytes_to_u32s(v, self.meta.num_values)?),
                RawStreamData::VarInt(_) => {
                    return Err(MltError::StreamDataMismatch("Encoded", "VarInt"));
                }
            },
            PhysicalEncoding::FastPFOR => match &self.data {
                RawStreamData::Encoded(v) => Ok(decode_fastpfor_composite(
                    v,
                    self.meta.num_values.as_usize(),
                )?),
                RawStreamData::VarInt(_) => {
                    return Err(MltError::StreamDataMismatch("Encoded", "VarInt"));
                }
            },
            PhysicalEncoding::Alp => return Err(MltError::UnsupportedPhysicalEncoding("ALP")),
        }?;
        dec.consume(u32::try_from(value.len() * size_of::<u32>()).or_overflow()?)?;
        Ok(LogicalValue::new(self.meta, LogicalData::VecU32(value)))
    }

    /// Decode the physical layer into a `Vec<u64>` logical layer, charging `dec` for the
    /// intermediate allocation.
    pub fn decode_bits_u64(self, dec: &mut Decoder) -> Result<LogicalValue, MltError> {
        let value = match self.meta.encoding.physical {
            PhysicalEncoding::VarInt => match &self.data {
                RawStreamData::VarInt(v) => {
                    all(parse_varint_vec::<u64, u64>(v, self.meta.num_values)?)
                }
                RawStreamData::Encoded(_) => {
                    return Err(MltError::StreamDataMismatch("VarInt", "Encoded"));
                }
            },
            PhysicalEncoding::None => match &self.data {
                RawStreamData::Encoded(v) => all(decode_bytes_to_u64s(v, self.meta.num_values)?),
                RawStreamData::VarInt(_) => {
                    return Err(MltError::StreamDataMismatch("Encoded", "VarInt"));
                }
            },
            PhysicalEncoding::FastPFOR => {
                return Err(MltError::UnsupportedPhysicalEncoding(
                    "FastPFOR decoding u64",
                ));
            }
            PhysicalEncoding::Alp => return Err(MltError::UnsupportedPhysicalEncoding("ALP")),
        }?;
        dec.consume(u32::try_from(value.len() * size_of::<u64>()).or_overflow()?)?;
        Ok(LogicalValue::new(self.meta, LogicalData::VecU64(value)))
    }
}
