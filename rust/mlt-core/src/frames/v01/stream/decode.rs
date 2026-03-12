use crate::MltError;
use crate::errors::AsMltError as _;
use crate::utils::{
    AsUsize as _, all, decode_byte_rle, decode_bytes_to_bools, decode_bytes_to_u32s,
    decode_bytes_to_u64s, decode_fastpfor_composite, parse_varint_vec,
};
use crate::v01::{LogicalData, LogicalValue, PhysicalEncoding, Stream, StreamData};

impl Stream<'_> {
    /// Decode a boolean stream: byte-RLE → packed bitmap → `Vec<bool>`
    pub fn decode_bools(self) -> Result<Vec<bool>, MltError> {
        let num_values = self.meta.num_values.as_usize();
        let num_bytes = num_values.div_ceil(8);
        let raw = match &self.data {
            StreamData::Encoded(v) => v,
            StreamData::VarInt(_) => {
                return Err(MltError::NotImplemented("varint bool decoding"));
            }
        };
        let decoded = decode_byte_rle(raw, num_bytes);
        Ok(decode_bytes_to_bools(&decoded, num_values))
    }

    pub fn decode_i8s(self) -> Result<Vec<i8>, MltError> {
        self.decode_bits_u32()?
            .decode_i32()?
            .into_iter()
            .map(i8::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn decode_u8s(self) -> Result<Vec<u8>, MltError> {
        let decoded = self
            .decode_bits_u32()?
            .decode_u32()?
            .into_iter()
            .map(u8::try_from)
            .collect::<Result<Vec<u8>, _>>()?;
        Ok(decoded)
    }

    pub fn decode_i32s(self) -> Result<Vec<i32>, MltError> {
        self.decode_bits_u32()?.decode_i32()
    }

    pub fn decode_u32s(self) -> Result<Vec<u32>, MltError> {
        self.decode_bits_u32()?.decode_u32()
    }

    pub fn decode_bits_u32(&self) -> Result<LogicalValue, MltError> {
        let value = match self.meta.encoding.physical {
            PhysicalEncoding::VarInt => match &self.data {
                StreamData::VarInt(v) => {
                    all(parse_varint_vec::<u32, u32>(v, self.meta.num_values)?)
                }
                StreamData::Encoded(_) => {
                    return Err(MltError::StreamDataMismatch("VarInt", "Encoded"));
                }
            },
            PhysicalEncoding::None => match &self.data {
                StreamData::Encoded(v) => all(decode_bytes_to_u32s(v, self.meta.num_values)?),
                StreamData::VarInt(_) => {
                    return Err(MltError::StreamDataMismatch("Encoded", "VarInt"));
                }
            },
            PhysicalEncoding::FastPFOR => match &self.data {
                StreamData::Encoded(v) => Ok(decode_fastpfor_composite(
                    v,
                    self.meta.num_values.as_usize(),
                )?),
                StreamData::VarInt(_) => {
                    return Err(MltError::StreamDataMismatch("Encoded", "VarInt"));
                }
            },
            PhysicalEncoding::Alp => return Err(MltError::UnsupportedPhysicalEncoding("ALP")),
        }?;

        Ok(LogicalValue::new(self.meta, LogicalData::VecU32(value)))
    }

    pub fn decode_u64(self) -> Result<Vec<u64>, MltError> {
        self.decode_bits_u64()?.decode_u64()
    }

    /// Decode a signed i64 stream
    pub fn decode_i64(self) -> Result<Vec<i64>, MltError> {
        self.decode_bits_u64()?.decode_i64()
    }

    pub fn decode_bits_u64(self) -> Result<LogicalValue, MltError> {
        let value = match self.meta.encoding.physical {
            PhysicalEncoding::VarInt => match self.data {
                StreamData::VarInt(v) => {
                    all(parse_varint_vec::<u64, u64>(v, self.meta.num_values)?)
                }
                StreamData::Encoded(_) => {
                    return Err(MltError::StreamDataMismatch("VarInt", "Encoded"));
                }
            },
            PhysicalEncoding::None => match self.data {
                StreamData::Encoded(v) => all(decode_bytes_to_u64s(v, self.meta.num_values)?),
                StreamData::VarInt(_) => {
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

        Ok(LogicalValue::new(self.meta, LogicalData::VecU64(value)))
    }

    /// Decode a stream of f32 values from raw little-endian bytes
    pub fn decode_f32(self) -> Result<Vec<f32>, MltError> {
        let raw = match &self.data {
            StreamData::Encoded(v) => v,
            StreamData::VarInt(_) => {
                return Err(MltError::NotImplemented("varint f32 decoding"));
            }
        };
        let num = self.meta.num_values.as_usize();
        let expected_bytes = num.checked_mul(4).or_overflow()?;
        if raw.len() != expected_bytes {
            return Err(MltError::InvalidDecodingStreamSize(
                raw.len(),
                expected_bytes,
            ));
        }
        Ok(raw
            .chunks_exact(4)
            .map(|chunk| {
                let bytes = chunk
                    .try_into()
                    .expect("infallible because of `chunks_exact`");
                f32::from_le_bytes(bytes)
            })
            .collect())
    }

    /// Decode a stream of f64 values from raw little-endian bytes
    pub fn decode_f64(self) -> Result<Vec<f64>, MltError> {
        let raw = match &self.data {
            StreamData::Encoded(v) => v,
            StreamData::VarInt(_) => {
                return Err(MltError::NotImplemented("varint f64 decoding"));
            }
        };
        let num = self.meta.num_values.as_usize();
        let expected_bytes = num.checked_mul(8).or_overflow()?;
        if raw.len() != expected_bytes {
            return Err(MltError::InvalidDecodingStreamSize(
                raw.len(),
                expected_bytes,
            ));
        }
        Ok(raw
            .chunks_exact(8)
            .map(|chunk| {
                let bytes = chunk
                    .try_into()
                    .expect("infallible because of `chunks_exact`");
                f64::from_le_bytes(bytes)
            })
            .collect())
    }
}
