use std::mem;

use crate::codecs::bytes::{decode_bytes_to_bools, decode_bytes_to_u32s, decode_bytes_to_u64s};
use crate::codecs::fastpfor::decode_fastpfor;
use crate::codecs::rle::decode_byte_rle;
use crate::codecs::varint::parse_varint_vec;
use crate::decoder::{LogicalEncoding, LogicalValue, PhysicalEncoding, RawStream};
use crate::errors::{AsMltError as _, fail_if_invalid_stream_size};
use crate::utils::AsUsize as _;
use crate::{Decoder, MltError, MltResult};

impl RawStream<'_> {
    /// Decode a boolean stream: byte-RLE → packed bitmap → `Vec<bool>`, charging `dec`.
    pub fn decode_bools(self, dec: &mut Decoder) -> MltResult<Vec<bool>> {
        if self.meta.encoding.physical == PhysicalEncoding::VarInt {
            return Err(MltError::NotImplemented("varint bool decoding"));
        }
        let num_values = self.meta.num_values.as_usize();
        let num_bytes = num_values.div_ceil(8);
        let decoded = decode_byte_rle(self.data, num_bytes, dec)?;
        decode_bytes_to_bools(&decoded, num_values, dec)
    }

    pub fn decode_i8s(self, dec: &mut Decoder) -> MltResult<Vec<i8>> {
        self.decode_i32s(dec)?
            .into_iter()
            .map(i8::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn decode_u8s(self, dec: &mut Decoder) -> MltResult<Vec<u8>> {
        self.decode_u32s(dec)?
            .into_iter()
            .map(u8::try_from)
            .collect::<Result<Vec<u8>, _>>()
            .map_err(Into::into)
    }

    pub fn decode_i32s(self, dec: &mut Decoder) -> MltResult<Vec<i32>> {
        let meta = self.meta;
        // i32 always needs a logical transform (zigzag at minimum) — use scratch buffer.
        let mut buf = mem::take(&mut dec.buffer_u32);
        self.decode_bits_u32(&mut buf, dec)?;
        let result = LogicalValue::new(meta).decode_i32(&buf, dec);
        dec.buffer_u32 = buf;
        dec.buffer_u32.clear();
        result
    }

    pub fn decode_u32s(self, dec: &mut Decoder) -> MltResult<Vec<u32>> {
        let meta = self.meta;
        if meta.encoding.logical == LogicalEncoding::None {
            // No logical transform: physical words are the output — decode into a fresh Vec.
            let mut out = Vec::new();
            self.decode_bits_u32(&mut out, dec)?;
            Ok(out)
        } else {
            // Logical transform needed — use the reusable scratch buffer.
            let mut buf = mem::take(&mut dec.buffer_u32);
            self.decode_bits_u32(&mut buf, dec)?;
            let result = LogicalValue::new(meta).decode_u32(&buf, dec);
            dec.buffer_u32 = buf;
            dec.buffer_u32.clear();
            result
        }
    }

    pub fn decode_u64s(self, dec: &mut Decoder) -> MltResult<Vec<u64>> {
        let meta = self.meta;
        if meta.encoding.logical == LogicalEncoding::None {
            // No logical transform: physical words are the output — decode into a fresh Vec.
            let mut out = Vec::new();
            self.decode_bits_u64(&mut out, dec)?;
            Ok(out)
        } else {
            // Logical transform needed — use the reusable scratch buffer.
            let mut buf = mem::take(&mut dec.buffer_u64);
            self.decode_bits_u64(&mut buf, dec)?;
            let result = LogicalValue::new(meta).decode_u64(&buf, dec);
            dec.buffer_u64 = buf;
            dec.buffer_u64.clear();
            result
        }
    }

    pub fn decode_i64s(self, dec: &mut Decoder) -> MltResult<Vec<i64>> {
        let meta = self.meta;
        // i64 always needs a logical transform (zigzag at minimum) — use scratch buffer.
        let mut buf = mem::take(&mut dec.buffer_u64);
        self.decode_bits_u64(&mut buf, dec)?;
        let result = LogicalValue::new(meta).decode_i64(&buf, dec);
        dec.buffer_u64 = buf;
        dec.buffer_u64.clear();
        result
    }

    /// Decode a stream of f32 values from raw little-endian bytes, charging `dec`.
    pub fn decode_f32s(self, dec: &mut Decoder) -> MltResult<Vec<f32>> {
        if self.meta.encoding.physical == PhysicalEncoding::VarInt {
            return Err(MltError::NotImplemented("varint f32 decoding"));
        }
        let num = self.meta.num_values.as_usize();
        dec.consume_items::<f32>(num)?;
        fail_if_invalid_stream_size(self.data.len(), num.checked_mul(4).or_overflow()?)?;

        Ok(self
            .data
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes(chunk.try_into().expect("infallible: chunks_exact(4)")))
            .collect())
    }

    /// Decode a stream of f64 values from raw little-endian bytes, charging `dec`.
    pub fn decode_f64s(self, dec: &mut Decoder) -> MltResult<Vec<f64>> {
        if self.meta.encoding.physical == PhysicalEncoding::VarInt {
            return Err(MltError::NotImplemented("varint f64 decoding"));
        }
        let num = self.meta.num_values.as_usize();
        fail_if_invalid_stream_size(self.data.len(), num.checked_mul(8).or_overflow()?)?;

        dec.consume_items::<f64>(num)?;
        Ok(self
            .data
            .chunks_exact(8)
            .map(|chunk| f64::from_le_bytes(chunk.try_into().expect("infallible: chunks_exact(8)")))
            .collect())
    }

    /// Physically decode the stream into `buf` as `u32` values.
    ///
    /// `buf` is cleared and filled with the decoded words. The caller owns the
    /// buffer and is responsible for deciding whether it constitutes a final
    /// persistent allocation (and therefore should be charged to a [`Decoder`]).
    pub fn decode_bits_u32(self, buf: &mut Vec<u32>, dec: &mut Decoder) -> MltResult<()> {
        buf.clear();
        match self.meta.encoding.physical {
            PhysicalEncoding::VarInt => {
                let (_, values) =
                    parse_varint_vec::<u32, u32>(self.data, self.meta.num_values, dec)?;
                *buf = values;
            }
            PhysicalEncoding::None => {
                let (_, values) = decode_bytes_to_u32s(self.data, self.meta.num_values, dec)?;
                *buf = values;
            }
            PhysicalEncoding::FastPFor256 => {
                *buf = decode_fastpfor(self.data, self.meta.num_values, dec)?;
            }
            PhysicalEncoding::Alp => return Err(MltError::UnsupportedPhysicalEncoding("ALP")),
        }
        Ok(())
    }

    /// Physically decode the stream into `buf` as `u64` values.
    ///
    /// `buf` is cleared and filled with the decoded words. The caller owns the
    /// buffer and is responsible for deciding whether it constitutes a final
    /// persistent allocation (and therefore should be charged to a [`Decoder`]).
    pub fn decode_bits_u64(self, buf: &mut Vec<u64>, dec: &mut Decoder) -> MltResult<()> {
        buf.clear();
        match self.meta.encoding.physical {
            PhysicalEncoding::VarInt => {
                let (_, values) =
                    parse_varint_vec::<u64, u64>(self.data, self.meta.num_values, dec)?;
                *buf = values;
            }
            PhysicalEncoding::None => {
                let (_, values) = decode_bytes_to_u64s(self.data, self.meta.num_values, dec)?;
                *buf = values;
            }
            PhysicalEncoding::FastPFor256 => {
                return Err(MltError::UnsupportedPhysicalEncoding(
                    "FastPFOR decoding u64",
                ));
            }
            PhysicalEncoding::Alp => return Err(MltError::UnsupportedPhysicalEncoding("ALP")),
        }
        Ok(())
    }
}
