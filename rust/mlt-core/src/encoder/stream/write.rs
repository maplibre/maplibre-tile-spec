use bytemuck::{NoUninit, cast_slice};
use fastpfor::AnyLenCodec as _;
use integer_encoding::VarInt;
use num_traits::PrimInt;
use zigzag::ZigZag;

use crate::MltError::UnsupportedPhysicalEncoding;
use crate::MltResult;
use crate::codecs::zigzag::{encode_zigzag, encode_zigzag_delta};
use crate::decoder::{LogicalEncoding, PhysicalEncoding, StreamMeta, StreamType};
use crate::encoder::Encoder;
use crate::encoder::model::StreamCtx;
use crate::encoder::stream::codecs::{LogicalCodecs, PhysicalCodecs};
use crate::encoder::stream::logical::apply_rle;
use crate::encoder::stream::physical::PhysicalEncoder;
use crate::encoder::writer::AltSession;

#[inline]
pub(crate) fn write_stream_payload(
    data: &mut Vec<u8>,
    meta: StreamMeta,
    is_boolean: bool,
    payload: &[u8],
) -> MltResult<()> {
    let byte_length = u32::try_from(payload.len())?;
    meta.write_to(data, is_boolean, byte_length)?;
    data.extend_from_slice(payload);
    Ok(())
}

pub(crate) trait PhysicalIntStreamKind {
    type Value: Into<u64> + NoUninit + PrimInt;
    const FASTPFOR_ALLOWED: bool;

    #[cfg(target_endian = "little")]
    fn none<'a>(_physical: &'a mut PhysicalCodecs, values: &'a [Self::Value]) -> &'a [u8] {
        cast_slice(values)
    }
    #[cfg(not(target_endian = "little"))]
    fn none<'a>(physical: &'a mut PhysicalCodecs, values: &'a [Self::Value]) -> &'a [u8] {
        compile_error!("PhysicalEncoding::none is not implemented for non-little-endian targets");
    }

    fn fastpfor<'a>(
        physical: &'a mut PhysicalCodecs,
        values: &'a [Self::Value],
    ) -> MltResult<&'a [u8]>;
}

impl PhysicalCodecs {
    pub(crate) fn varint<T>(&mut self, values: &[T]) -> &[u8]
    where
        T: Copy + Into<u64>,
    {
        // encode_var writes to a stack buffer; avoids the Vec<u8> allocation
        // that encode_var_vec() would produce for every value.
        self.u8_tmp.clear();
        let mut buf = [0u8; 10];
        for &v in values {
            let n = v.into().encode_var(&mut buf);
            self.u8_tmp.extend_from_slice(&buf[..n]);
        }
        &self.u8_tmp
    }

    pub(crate) fn fastpfor(&mut self, values: &[u32]) -> MltResult<&[u8]> {
        self.u8_tmp.clear();
        if !values.is_empty() {
            self.u32_tmp.clear();
            self.fastpfor.encode(values, &mut self.u32_tmp)?;
            for word in &mut self.u32_tmp {
                *word = word.to_be();
            }
            self.u8_tmp.extend_from_slice(cast_slice(&self.u32_tmp));
        }
        Ok(&self.u8_tmp)
    }

    /// Physically encode and write stream to the output.
    pub(crate) fn write_encoded_as<P: PhysicalIntStreamKind + ?Sized>(
        &mut self,
        ctx: &StreamCtx,
        enc: &mut Encoder,
        le: LogicalEncoding,
        values: &[P::Value],
        encode_as: PhysicalEncoder,
    ) -> MltResult<()> {
        use PhysicalEncoding as PE;
        let (pe, vals) = match encode_as {
            PhysicalEncoder::None => (PE::None, P::none(self, values)),
            PhysicalEncoder::VarInt => (PE::VarInt, self.varint(values)),
            PhysicalEncoder::FastPFOR => (PE::FastPFor256, P::fastpfor(self, values)?),
        };
        let meta = StreamMeta::new2(ctx.stream_type, le, pe, values.len())?;
        write_stream_payload(&mut enc.data, meta, false, vals)
    }

    pub(crate) fn write_alternatives<P: PhysicalIntStreamKind + ?Sized>(
        &mut self,
        alt: &mut AltSession<'_>,
        values: &[P::Value],
        logical: LogicalEncoding,
        stream_type: StreamType,
        allow_fpf: bool,
    ) -> MltResult<()> {
        use PhysicalEncoding as PE;
        // `FASTPFOR_ALLOWED` is the type-level capability: FastPFOR only supports u32.
        // `allow_fpf` is the caller's runtime preference.
        // Both must hold to try FastPFOR.
        if P::FASTPFOR_ALLOWED && allow_fpf {
            alt.with(|enc| {
                let meta = StreamMeta::new2(stream_type, logical, PE::FastPFor256, values.len())?;
                write_stream_payload(&mut enc.data, meta, false, P::fastpfor(self, values)?)
            })?;
        }
        alt.with(|enc| {
            let meta = StreamMeta::new2(stream_type, logical, PE::VarInt, values.len())?;
            write_stream_payload(&mut enc.data, meta, false, self.varint(values))
        })
    }
}

impl PhysicalIntStreamKind for [u32] {
    type Value = u32;
    const FASTPFOR_ALLOWED: bool = true;

    fn fastpfor<'a>(
        physical: &'a mut PhysicalCodecs,
        values: &'a [Self::Value],
    ) -> MltResult<&'a [u8]> {
        physical.fastpfor(values)
    }
}

impl PhysicalIntStreamKind for [u64] {
    type Value = u64;
    const FASTPFOR_ALLOWED: bool = false;

    fn fastpfor<'a>(
        _physical: &'a mut PhysicalCodecs,
        _values: &'a [Self::Value],
    ) -> MltResult<&'a [u8]> {
        Err(UnsupportedPhysicalEncoding("FastPFOR on u64"))?
    }
}

pub(crate) trait LogicalIntStreamKind {
    type Input;
    type Output: PhysicalIntStreamKind + ?Sized;
    type Profile: ZigZag<UInt = <Self::Output as PhysicalIntStreamKind>::Value>;
}

pub(crate) trait LogicalIntCodec<T: LogicalIntStreamKind + ?Sized> {
    fn none<'a>(&'a mut self, values: &'a T) -> &'a [<T::Output as PhysicalIntStreamKind>::Value];

    fn delta<'a>(&'a mut self, values: &'a T) -> &'a [<T::Output as PhysicalIntStreamKind>::Value];

    fn rle<'a>(
        &'a mut self,
        values: &'a T,
    ) -> MltResult<(
        LogicalEncoding,
        &'a [<T::Output as PhysicalIntStreamKind>::Value],
    )>;

    fn delta_rle<'a>(
        &'a mut self,
        values: &'a T,
    ) -> MltResult<(
        LogicalEncoding,
        &'a [<T::Output as PhysicalIntStreamKind>::Value],
    )>;
}

fn encode_u8_as_u32<'a>(values: &[u8], target: &'a mut Vec<u32>) -> &'a [u32] {
    target.clear();
    target.extend(values.iter().map(|&v| u32::from(v)));
    target
}

fn encode_i8_zigzag<'a>(values: &[i8], target: &'a mut Vec<u32>) -> &'a [u32] {
    target.clear();
    target.extend(values.iter().map(|&v| i32::encode(i32::from(v))));
    target
}

fn encode_u8_delta<'a>(values: &[u8], target: &'a mut Vec<u32>) -> &'a [u32] {
    target.clear();
    target.reserve(values.len());
    let mut prev = 0_i32;
    for &v in values {
        let v = i32::from(v);
        target.push(i32::encode(v.wrapping_sub(prev)));
        prev = v;
    }
    target
}

fn encode_i8_delta<'a>(values: &[i8], target: &'a mut Vec<u32>) -> &'a [u32] {
    target.clear();
    target.reserve(values.len());
    let mut prev = 0_i32;
    for &v in values {
        let v = i32::from(v);
        target.push(i32::encode(v.wrapping_sub(prev)));
        prev = v;
    }
    target
}

impl LogicalIntStreamKind for [u8] {
    type Input = u8;
    type Output = [u32];
    type Profile = i32;
}

impl LogicalIntCodec<[u8]> for LogicalCodecs {
    fn none<'a>(&'a mut self, values: &'a [u8]) -> &'a [u32] {
        encode_u8_as_u32(values, &mut self.u32_tmp)
    }

    fn delta<'a>(&'a mut self, values: &'a [u8]) -> &'a [u32] {
        encode_u8_delta(values, &mut self.u32_tmp)
    }

    fn rle<'a>(&'a mut self, values: &'a [u8]) -> MltResult<(LogicalEncoding, &'a [u32])> {
        let data = encode_u8_as_u32(values, &mut self.u32_tmp);
        let meta = apply_rle(data, values.len(), &mut self.u32_tmp2)?;
        Ok((LogicalEncoding::Rle(meta), &self.u32_tmp2))
    }

    fn delta_rle<'a>(&'a mut self, values: &'a [u8]) -> MltResult<(LogicalEncoding, &'a [u32])> {
        let data = encode_u8_delta(values, &mut self.u32_tmp);
        let meta = apply_rle(data, values.len(), &mut self.u32_tmp2)?;
        Ok((LogicalEncoding::DeltaRle(meta), &self.u32_tmp2))
    }
}

impl LogicalIntStreamKind for [i8] {
    type Input = i8;
    type Output = [u32];
    type Profile = i32;
}

impl LogicalIntCodec<[i8]> for LogicalCodecs {
    fn none<'a>(&'a mut self, values: &'a [i8]) -> &'a [u32] {
        encode_i8_zigzag(values, &mut self.u32_tmp)
    }

    fn delta<'a>(&'a mut self, values: &'a [i8]) -> &'a [u32] {
        encode_i8_delta(values, &mut self.u32_tmp)
    }

    fn rle<'a>(&'a mut self, values: &'a [i8]) -> MltResult<(LogicalEncoding, &'a [u32])> {
        let data = encode_i8_zigzag(values, &mut self.u32_tmp);
        let meta = apply_rle(data, values.len(), &mut self.u32_tmp2)?;
        Ok((LogicalEncoding::Rle(meta), &self.u32_tmp2))
    }

    fn delta_rle<'a>(&'a mut self, values: &'a [i8]) -> MltResult<(LogicalEncoding, &'a [u32])> {
        let data = encode_i8_delta(values, &mut self.u32_tmp);
        let meta = apply_rle(data, values.len(), &mut self.u32_tmp2)?;
        Ok((LogicalEncoding::DeltaRle(meta), &self.u32_tmp2))
    }
}

impl LogicalIntStreamKind for [u32] {
    type Input = u32;
    type Output = [u32];
    type Profile = i32;
}

impl LogicalIntCodec<[u32]> for LogicalCodecs {
    fn none<'a>(&'a mut self, values: &'a [u32]) -> &'a [u32] {
        values
    }

    fn delta<'a>(&'a mut self, values: &'a [u32]) -> &'a [u32] {
        encode_zigzag_delta(cast_slice::<u32, i32>(values), &mut self.u32_tmp)
    }

    fn rle<'a>(&'a mut self, values: &'a [u32]) -> MltResult<(LogicalEncoding, &'a [u32])> {
        let meta = apply_rle(values, values.len(), &mut self.u32_tmp)?;
        Ok((LogicalEncoding::Rle(meta), &self.u32_tmp))
    }

    fn delta_rle<'a>(&'a mut self, values: &'a [u32]) -> MltResult<(LogicalEncoding, &'a [u32])> {
        let data = encode_zigzag_delta(cast_slice::<u32, i32>(values), &mut self.u32_tmp);
        let meta = apply_rle(data, values.len(), &mut self.u32_tmp2)?;
        Ok((LogicalEncoding::DeltaRle(meta), &self.u32_tmp2))
    }
}

impl LogicalIntStreamKind for [i32] {
    type Input = i32;
    type Output = [u32];
    type Profile = i32;
}

impl LogicalIntCodec<[i32]> for LogicalCodecs {
    fn none<'a>(&'a mut self, values: &'a [i32]) -> &'a [u32] {
        encode_zigzag(values, &mut self.u32_tmp)
    }

    fn delta<'a>(&'a mut self, values: &'a [i32]) -> &'a [u32] {
        encode_zigzag_delta(values, &mut self.u32_tmp)
    }

    fn rle<'a>(&'a mut self, values: &'a [i32]) -> MltResult<(LogicalEncoding, &'a [u32])> {
        let data = encode_zigzag(values, &mut self.u32_tmp);
        let meta = apply_rle(data, values.len(), &mut self.u32_tmp2)?;
        Ok((LogicalEncoding::Rle(meta), &self.u32_tmp2))
    }

    fn delta_rle<'a>(&'a mut self, values: &'a [i32]) -> MltResult<(LogicalEncoding, &'a [u32])> {
        let data = encode_zigzag_delta(values, &mut self.u32_tmp);
        let meta = apply_rle(data, values.len(), &mut self.u32_tmp2)?;
        Ok((LogicalEncoding::DeltaRle(meta), &self.u32_tmp2))
    }
}

impl LogicalIntStreamKind for [u64] {
    type Input = u64;
    type Output = [u64];
    type Profile = i64;
}

impl LogicalIntCodec<[u64]> for LogicalCodecs {
    fn none<'a>(&'a mut self, values: &'a [u64]) -> &'a [u64] {
        values
    }

    fn delta<'a>(&'a mut self, values: &'a [u64]) -> &'a [u64] {
        encode_zigzag_delta(cast_slice::<u64, i64>(values), &mut self.u64_tmp)
    }

    fn rle<'a>(&'a mut self, values: &'a [u64]) -> MltResult<(LogicalEncoding, &'a [u64])> {
        let meta = apply_rle(values, values.len(), &mut self.u64_tmp)?;
        Ok((LogicalEncoding::Rle(meta), &self.u64_tmp))
    }

    fn delta_rle<'a>(&'a mut self, values: &'a [u64]) -> MltResult<(LogicalEncoding, &'a [u64])> {
        let data = encode_zigzag_delta(cast_slice::<u64, i64>(values), &mut self.u64_tmp);
        let meta = apply_rle(data, values.len(), &mut self.u64_tmp2)?;
        Ok((LogicalEncoding::DeltaRle(meta), &self.u64_tmp2))
    }
}

impl LogicalIntStreamKind for [i64] {
    type Input = i64;
    type Output = [u64];
    type Profile = i64;
}

impl LogicalIntCodec<[i64]> for LogicalCodecs {
    fn none<'a>(&'a mut self, values: &'a [i64]) -> &'a [u64] {
        encode_zigzag(values, &mut self.u64_tmp)
    }

    fn delta<'a>(&'a mut self, values: &'a [i64]) -> &'a [u64] {
        encode_zigzag_delta(values, &mut self.u64_tmp)
    }

    fn rle<'a>(&'a mut self, values: &'a [i64]) -> MltResult<(LogicalEncoding, &'a [u64])> {
        let data = encode_zigzag(values, &mut self.u64_tmp);
        let meta = apply_rle(data, values.len(), &mut self.u64_tmp2)?;
        Ok((LogicalEncoding::Rle(meta), &self.u64_tmp2))
    }

    fn delta_rle<'a>(&'a mut self, values: &'a [i64]) -> MltResult<(LogicalEncoding, &'a [u64])> {
        let data = encode_zigzag_delta(values, &mut self.u64_tmp);
        let meta = apply_rle(data, values.len(), &mut self.u64_tmp2)?;
        Ok((LogicalEncoding::DeltaRle(meta), &self.u64_tmp2))
    }
}
