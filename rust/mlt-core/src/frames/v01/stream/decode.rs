use std::mem::size_of;

use crate::errors::{AsMltError as _, fail_if_invalid_stream_size};
use crate::utils::{
    AsUsize as _, decode_byte_rle, decode_bytes_to_bools, decode_bytes_to_u32s,
    decode_bytes_to_u64s, decode_fastpfor_composite, parse_varint_vec,
};
use crate::v01::{LogicalEncoding, LogicalValue, PhysicalEncoding, RawStream, RawStreamData};
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
        let meta = self.meta;
        // i32 always needs a logical transform (zigzag at minimum) — use scratch buffer.
        self.decode_bits_u32(&mut dec.buffer_u32)?;
        let buf = std::mem::take(&mut dec.buffer_u32);
        let result = LogicalValue::new(meta).decode_i32(&buf, dec);
        dec.buffer_u32 = buf;
        dec.buffer_u32.clear();
        result
    }

    pub fn decode_u32s(self, dec: &mut Decoder) -> Result<Vec<u32>, MltError> {
        let meta = self.meta;
        if meta.encoding.logical == LogicalEncoding::None {
            // No logical transform: physical words are the output — decode into a fresh Vec.
            let mut out = Vec::new();
            self.decode_bits_u32(&mut out)?;
            dec.consume(u32::try_from(out.len() * size_of::<u32>()).or_overflow()?)?;
            Ok(out)
        } else {
            // Logical transform needed — use the reusable scratch buffer.
            self.decode_bits_u32(&mut dec.buffer_u32)?;
            let buf = std::mem::take(&mut dec.buffer_u32);
            let result = LogicalValue::new(meta).decode_u32(&buf, dec);
            dec.buffer_u32 = buf;
            dec.buffer_u32.clear();
            result
        }
    }

    pub fn decode_u64s(self, dec: &mut Decoder) -> Result<Vec<u64>, MltError> {
        let meta = self.meta;
        if meta.encoding.logical == LogicalEncoding::None {
            // No logical transform: physical words are the output — decode into a fresh Vec.
            let mut out = Vec::new();
            self.decode_bits_u64(&mut out)?;
            dec.consume(u32::try_from(out.len() * size_of::<u64>()).or_overflow()?)?;
            Ok(out)
        } else {
            // Logical transform needed — use the reusable scratch buffer.
            self.decode_bits_u64(&mut dec.buffer_u64)?;
            let buf = std::mem::take(&mut dec.buffer_u64);
            let result = LogicalValue::new(meta).decode_u64(&buf, dec);
            dec.buffer_u64 = buf;
            dec.buffer_u64.clear();
            result
        }
    }

    pub fn decode_i64s(self, dec: &mut Decoder) -> Result<Vec<i64>, MltError> {
        let meta = self.meta;
        // i64 always needs a logical transform (zigzag at minimum) — use scratch buffer.
        self.decode_bits_u64(&mut dec.buffer_u64)?;
        let buf = std::mem::take(&mut dec.buffer_u64);
        let result = LogicalValue::new(meta).decode_i64(&buf, dec);
        dec.buffer_u64 = buf;
        dec.buffer_u64.clear();
        result
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

    /// Physically decode the stream into `buf` as `u32` values.
    ///
    /// `buf` is cleared and filled with the decoded words. The caller owns the
    /// buffer and is responsible for deciding whether it constitutes a final
    /// persistent allocation (and therefore should be charged to a [`Decoder`]).
    /// No budget is charged here.
    pub fn decode_bits_u32(self, buf: &mut Vec<u32>) -> Result<(), MltError> {
        buf.clear();
        match self.meta.encoding.physical {
            PhysicalEncoding::VarInt => match &self.data {
                RawStreamData::VarInt(v) => {
                    let (_, values) = parse_varint_vec::<u32, u32>(v, self.meta.num_values)?;
                    *buf = values;
                }
                RawStreamData::Encoded(_) => {
                    return Err(MltError::StreamDataMismatch("VarInt", "Encoded"));
                }
            },
            PhysicalEncoding::None => match &self.data {
                RawStreamData::Encoded(v) => {
                    let (_, values) = decode_bytes_to_u32s(v, self.meta.num_values)?;
                    *buf = values;
                }
                RawStreamData::VarInt(_) => {
                    return Err(MltError::StreamDataMismatch("Encoded", "VarInt"));
                }
            },
            PhysicalEncoding::FastPFOR => match &self.data {
                RawStreamData::Encoded(v) => {
                    *buf = decode_fastpfor_composite(v, self.meta.num_values.as_usize())?;
                }
                RawStreamData::VarInt(_) => {
                    return Err(MltError::StreamDataMismatch("Encoded", "VarInt"));
                }
            },
            PhysicalEncoding::Alp => return Err(MltError::UnsupportedPhysicalEncoding("ALP")),
        }
        Ok(())
    }

    /// Physically decode the stream into `buf` as `u64` values.
    ///
    /// `buf` is cleared and filled with the decoded words. The caller owns the
    /// buffer and is responsible for deciding whether it constitutes a final
    /// persistent allocation (and therefore should be charged to a [`Decoder`]).
    /// No budget is charged here.
    pub fn decode_bits_u64(self, buf: &mut Vec<u64>) -> Result<(), MltError> {
        buf.clear();
        match self.meta.encoding.physical {
            PhysicalEncoding::VarInt => match &self.data {
                RawStreamData::VarInt(v) => {
                    let (_, values) = parse_varint_vec::<u64, u64>(v, self.meta.num_values)?;
                    *buf = values;
                }
                RawStreamData::Encoded(_) => {
                    return Err(MltError::StreamDataMismatch("VarInt", "Encoded"));
                }
            },
            PhysicalEncoding::None => match &self.data {
                RawStreamData::Encoded(v) => {
                    let (_, values) = decode_bytes_to_u64s(v, self.meta.num_values)?;
                    *buf = values;
                }
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
        }
        Ok(())
    }
}
