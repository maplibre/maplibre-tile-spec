use std::collections::HashMap;

use fastpfor::{AnyLenCodec, FastPFor256};
use integer_encoding::VarInt;

use crate::MltError::UnsupportedPhysicalEncoding;
use crate::codecs::bytes::encode_bools_to_bytes;
use crate::codecs::rle::encode_byte_rle;
use crate::codecs::zigzag::{encode_componentwise_delta_vec2s, encode_zigzag, encode_zigzag_delta};
use crate::decoder::{LogicalEncoding, PhysicalEncoding, RleMeta};
use crate::encoder::stream::logical::{LogicalEncoder, apply_rle};
use crate::encoder::stream::physical::PhysicalEncoder;
use crate::errors::MltResult;

#[derive(Default)]
pub struct LogicalCodecs {
    u32_values: Vec<u32>,
    u32_scratch: Vec<u32>,
    u64_values: Vec<u64>,
    u64_scratch: Vec<u64>,
    bool_packed: Vec<u8>,
    bool_rle: Vec<u8>,

    /// Reusable scratch for the Hilbert vertex-dictionary path. Held here so
    /// allocations amortise across geometry columns. Owned-and-returned around
    /// stream writes via `mem::take` because `write_geo_*_stream` requires
    /// `&mut Codecs`, which conflicts with a borrowed `&[u32]` / `&[i32]`
    /// view into these fields.
    pub(crate) hilbert_offsets: Vec<u32>,
    pub(crate) hilbert_indexed: Vec<u64>,
    pub(crate) hilbert_dict_xy: Vec<i32>,
    pub(crate) hilbert_remap: HashMap<u32, u32>,
}

impl LogicalCodecs {
    pub(crate) fn encode_componentwise_delta_vec2s(&mut self, vertices: &[i32]) -> &[u32] {
        encode_componentwise_delta_vec2s(vertices, &mut self.u32_values);
        &self.u32_values
    }

    /// Delta-encode a sorted slice of Morton codes: `[codes[0], codes[1]-codes[0], ...]`.
    /// Clears `target` and fills it with the delta-encoded values.
    #[inline]
    pub(crate) fn encode_morton_deltas(&mut self, codes: &[u32]) -> &[u32] {
        self.u32_values.clear();
        if let Some(&first) = codes.first() {
            self.u32_values.reserve(codes.len());
            self.u32_values
                .extend(std::iter::once(first).chain(codes.windows(2).map(|w| w[1] - w[0])));
        }
        &self.u32_values
    }

    pub(crate) fn encode_bools(
        &mut self,
        values: impl ExactSizeIterator<Item = bool>,
    ) -> MltResult<(LogicalEncoding, &[u8])> {
        let num_values = u32::try_from(values.len())?;
        encode_bools_to_bytes(values, &mut self.bool_packed);
        encode_byte_rle(&self.bool_packed, &mut self.bool_rle);
        let meta = LogicalEncoding::Rle(RleMeta {
            runs: num_values.div_ceil(8),
            num_rle_values: u32::try_from(self.bool_rle.len())?,
        });
        Ok((meta, &self.bool_rle))
    }

    #[hotpath::measure]
    pub(crate) fn encode_u32<'a>(
        &'a mut self,
        values: &'a [u32],
        logical: LogicalEncoder,
    ) -> MltResult<(LogicalEncoding, &'a [u32])> {
        Ok(match logical {
            LogicalEncoder::None => (LogicalEncoding::None, values),
            LogicalEncoder::Delta => {
                encode_zigzag_delta(
                    bytemuck::cast_slice::<u32, i32>(values),
                    &mut self.u32_values,
                );
                (LogicalEncoding::Delta, &self.u32_values)
            }
            LogicalEncoder::Rle => {
                let meta = apply_rle(values, values.len(), &mut self.u32_values)?;
                (LogicalEncoding::Rle(meta), &self.u32_values)
            }
            LogicalEncoder::DeltaRle => {
                encode_zigzag_delta(
                    bytemuck::cast_slice::<u32, i32>(values),
                    &mut self.u32_scratch,
                );
                let meta = apply_rle(&self.u32_scratch, values.len(), &mut self.u32_values)?;
                (LogicalEncoding::DeltaRle(meta), &self.u32_values)
            }
        })
    }

    #[hotpath::measure]
    pub(crate) fn encode_i32(
        &mut self,
        values: &[i32],
        logical: LogicalEncoder,
    ) -> MltResult<(LogicalEncoding, &[u32])> {
        let e = match logical {
            LogicalEncoder::None => {
                encode_zigzag(values, &mut self.u32_values);
                LogicalEncoding::None
            }
            LogicalEncoder::Delta => {
                encode_zigzag_delta(values, &mut self.u32_values);
                LogicalEncoding::Delta
            }
            LogicalEncoder::Rle => {
                encode_zigzag(values, &mut self.u32_scratch);
                let meta = apply_rle(&self.u32_scratch, values.len(), &mut self.u32_values)?;
                LogicalEncoding::Rle(meta)
            }
            LogicalEncoder::DeltaRle => {
                encode_zigzag_delta(values, &mut self.u32_scratch);
                let meta = apply_rle(&self.u32_scratch, values.len(), &mut self.u32_values)?;
                LogicalEncoding::DeltaRle(meta)
            }
        };
        Ok((e, &self.u32_values))
    }

    #[hotpath::measure]
    pub(crate) fn encode_u64<'a>(
        &'a mut self,
        values: &'a [u64],
        logical: LogicalEncoder,
    ) -> MltResult<(LogicalEncoding, &'a [u64])> {
        Ok(match logical {
            LogicalEncoder::None => (LogicalEncoding::None, values),
            LogicalEncoder::Delta => {
                encode_zigzag_delta(
                    bytemuck::cast_slice::<u64, i64>(values),
                    &mut self.u64_values,
                );
                (LogicalEncoding::Delta, &self.u64_values)
            }
            LogicalEncoder::Rle => {
                let meta = apply_rle(values, values.len(), &mut self.u64_values)?;
                (LogicalEncoding::Rle(meta), &self.u64_values)
            }
            LogicalEncoder::DeltaRle => {
                encode_zigzag_delta(
                    bytemuck::cast_slice::<u64, i64>(values),
                    &mut self.u64_scratch,
                );
                let meta = apply_rle(&self.u64_scratch, values.len(), &mut self.u64_values)?;
                (LogicalEncoding::DeltaRle(meta), &self.u64_values)
            }
        })
    }

    #[hotpath::measure]
    pub(crate) fn encode_i64(
        &mut self,
        values: &[i64],
        logical_enc: LogicalEncoder,
    ) -> MltResult<(LogicalEncoding, &[u64])> {
        let logical = match logical_enc {
            LogicalEncoder::None => {
                encode_zigzag(values, &mut self.u64_values);
                LogicalEncoding::None
            }
            LogicalEncoder::Delta => {
                encode_zigzag_delta(values, &mut self.u64_values);
                LogicalEncoding::Delta
            }
            LogicalEncoder::Rle => {
                encode_zigzag(values, &mut self.u64_scratch);
                let meta = apply_rle(&self.u64_scratch, values.len(), &mut self.u64_values)?;
                LogicalEncoding::Rle(meta)
            }
            LogicalEncoder::DeltaRle => {
                encode_zigzag_delta(values, &mut self.u64_scratch);
                let meta = apply_rle(&self.u64_scratch, values.len(), &mut self.u64_values)?;
                LogicalEncoding::DeltaRle(meta)
            }
        };
        Ok((logical, &self.u64_values))
    }

    pub(crate) fn encode_zigzag_i32(&mut self, values: &[i32]) -> &[u32] {
        encode_zigzag(values, &mut self.u32_values);
        &self.u32_values
    }

    pub(crate) fn encode_zigzag_i64(&mut self, values: &[i64]) -> &[u64] {
        encode_zigzag(values, &mut self.u64_values);
        &self.u64_values
    }
}

#[derive(Default)]
pub struct Codecs {
    pub(crate) logical: LogicalCodecs,
    pub(crate) physical: PhysicalCodecs,
}

#[derive(Default)]
pub struct PhysicalCodecs {
    tmp_u32: Vec<u32>,
    tmp_u8: Vec<u8>,
    fastpfor: FastPFor256,
}

impl PhysicalCodecs {
    pub(crate) fn encode_u32<'a>(
        &'a mut self,
        values: &'a [u32],
        physical_enc: PhysicalEncoder,
    ) -> MltResult<(PhysicalEncoding, &'a [u8])> {
        Ok(match physical_enc {
            PhysicalEncoder::None => {
                #[cfg(target_endian = "little")]
                {
                    (PhysicalEncoding::None, bytemuck::cast_slice(values))
                }
                #[cfg(not(target_endian = "little"))]
                {
                    self.tmp_u8.clear();
                    for &v in values {
                        self.tmp_u8.extend_from_slice(&v.to_le_bytes());
                    }
                    (PhysicalEncoding::None, &self.tmp_u8)
                }
            }
            PhysicalEncoder::VarInt => {
                // encode_var writes to a stack buffer; avoids the Vec<u8> allocation
                // that encode_var_vec() would produce for every value.
                self.tmp_u8.clear();
                let mut buf = [0u8; 10];
                for &v in values {
                    let n = u64::from(v).encode_var(&mut buf);
                    self.tmp_u8.extend_from_slice(&buf[..n]);
                }
                (PhysicalEncoding::VarInt, &self.tmp_u8)
            }
            PhysicalEncoder::FastPFOR => {
                self.tmp_u8.clear();
                if !values.is_empty() {
                    self.tmp_u32.clear();
                    self.fastpfor.encode(values, &mut self.tmp_u32)?;
                    for word in &mut self.tmp_u32 {
                        *word = word.to_be();
                    }
                    self.tmp_u8
                        .extend_from_slice(bytemuck::cast_slice(&self.tmp_u32));
                }
                (PhysicalEncoding::FastPFor256, &self.tmp_u8)
            }
        })
    }

    pub(crate) fn encode_u64<'a>(
        &'a mut self,
        values: &'a [u64],
        physical_enc: PhysicalEncoder,
    ) -> MltResult<(PhysicalEncoding, &'a [u8])> {
        Ok(match physical_enc {
            PhysicalEncoder::None => {
                #[cfg(target_endian = "little")]
                {
                    (PhysicalEncoding::None, bytemuck::cast_slice(values))
                }
                #[cfg(not(target_endian = "little"))]
                {
                    self.tmp_u8.clear();
                    for &v in values {
                        self.tmp_u8.extend_from_slice(&v.to_le_bytes());
                    }
                    (PhysicalEncoding::None, &self.tmp_u8)
                }
            }
            PhysicalEncoder::VarInt => {
                self.tmp_u8.clear();
                let mut buf = [0u8; 10];
                for &v in values {
                    let n = v.encode_var(&mut buf);
                    self.tmp_u8.extend_from_slice(&buf[..n]);
                }
                (PhysicalEncoding::VarInt, self.tmp_u8.as_ref())
            }
            PhysicalEncoder::FastPFOR => Err(UnsupportedPhysicalEncoding("FastPFOR on u64"))?,
        })
    }
}
