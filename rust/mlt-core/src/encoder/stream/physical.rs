use bytemuck;
use fastpfor::{AnyLenCodec as _, FastPFor256};
use integer_encoding::VarInt as _;

use crate::decoder::PhysicalEncoding;
use crate::{MltError, MltResult};

impl PhysicalEncoding {
    pub fn parse(value: u8) -> MltResult<Self> {
        Self::try_from(value).or(Err(MltError::ParsingPhysicalEncoding(value)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumIter)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum PhysicalEncoder {
    None,
    /// Can produce better results in combination with a heavyweight compression scheme like `Gzip`.
    /// Simple compression scheme where the encoding is easier to implement compared to `FastPFOR`.
    VarInt,
    /// Preferred, tends to produce the best compression ratio and decoding performance.
    ///
    /// Does not support u64/i64 integers
    FastPFOR,
}

impl PhysicalEncoder {
    /// Physically encode a `u32` sequence into `target`.
    ///
    /// `target` is treated as a scratch buffer: it is cleared before writing.
    /// After the call, `target.len()` is the number of encoded bytes.
    ///
    /// `scratch` is a reusable `Vec<u32>` for intermediate codec output (used by
    /// FastPFOR). Passing a long-lived buffer avoids a fresh allocation per call.
    #[cfg_attr(feature = "__hotpath", hotpath::measure)]
    pub fn encode_u32s(
        self,
        values: &[u32],
        target: &mut Vec<u8>,
        scratch: &mut Vec<u32>,
    ) -> MltResult<PhysicalEncoding> {
        target.clear();
        match self {
            Self::None => {
                // On little-endian targets native byte order == LE wire format:
                // cast_slice is a zero-copy reinterpret, extend_from_slice is one memcpy.
                #[cfg(target_endian = "little")]
                target.extend_from_slice(bytemuck::cast_slice(values));
                #[cfg(not(target_endian = "little"))]
                for &v in values {
                    target.extend_from_slice(&v.to_le_bytes());
                }
                Ok(PhysicalEncoding::None)
            }
            Self::VarInt => {
                // encode_var writes to a stack buffer; avoids the Vec<u8> allocation
                // that encode_var_vec() would produce for every value.
                let mut buf = [0u8; 10];
                for &v in values {
                    let n = u64::from(v).encode_var(&mut buf);
                    target.extend_from_slice(&buf[..n]);
                }
                Ok(PhysicalEncoding::VarInt)
            }
            Self::FastPFOR => {
                if !values.is_empty() {
                    scratch.clear();
                    FastPFor256::default().encode(values, scratch)?;
                    for word in scratch.iter_mut() {
                        *word = word.to_be();
                    }
                    target.extend_from_slice(bytemuck::cast_slice(scratch));
                }
                Ok(PhysicalEncoding::FastPFor256)
            }
        }
    }

    /// Physically encode a `u64` sequence into `target`.
    ///
    /// `target` is treated as a scratch buffer: it is cleared before writing.
    /// After the call, `target.len()` is the number of encoded bytes.
    ///
    /// Note: `FastPFOR` is not supported for `u64` streams.
    #[cfg_attr(feature = "__hotpath", hotpath::measure)]
    pub fn encode_u64s(self, values: &[u64], target: &mut Vec<u8>) -> MltResult<PhysicalEncoding> {
        target.clear();
        match self {
            Self::None => {
                #[cfg(target_endian = "little")]
                target.extend_from_slice(bytemuck::cast_slice(values));
                #[cfg(not(target_endian = "little"))]
                for &v in values {
                    target.extend_from_slice(&v.to_le_bytes());
                }
                Ok(PhysicalEncoding::None)
            }
            Self::VarInt => {
                let mut buf = [0u8; 10];
                for &v in values {
                    let n = v.encode_var(&mut buf);
                    target.extend_from_slice(&buf[..n]);
                }
                Ok(PhysicalEncoding::VarInt)
            }
            Self::FastPFOR => Err(MltError::UnsupportedPhysicalEncoding("FastPFOR on u64")),
        }
    }
}
