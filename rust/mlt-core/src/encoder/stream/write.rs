use bytemuck::{NoUninit, cast_slice};
use fastpfor::AnyLenCodec as _;
use integer_encoding::VarInt;
use num_traits::PrimInt;

use crate::MltError::UnsupportedPhysicalEncoding;
use crate::MltResult;
use crate::codecs::zigzag::{encode_zigzag, encode_zigzag_delta};
use crate::decoder::{LogicalEncoding, PhysicalEncoding, StreamMeta, StreamType};
use crate::encoder::Encoder;
use crate::encoder::model::StreamCtx;
use crate::encoder::stream::DataProfile;
use crate::encoder::stream::codecs::{Codecs, LogicalCodecs, PhysicalCodecs};
use crate::encoder::stream::logical::{LogicalEncoder, apply_rle};
use crate::encoder::stream::physical::PhysicalEncoder;

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

pub(crate) fn write_bool_stream(
    values: impl ExactSizeIterator<Item = bool>,
    stream_type: StreamType,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    let num_values = values.len();
    let (logical, vals) = codecs.logical.encode_bools(values)?;
    let meta = StreamMeta::new2(stream_type, logical, PhysicalEncoding::None, num_values)?;
    write_stream_payload(&mut enc.data, meta, true, vals)
}

pub(crate) trait PhysicalIntStreamKind {
    type Value: Into<u64> + NoUninit + PrimInt;
    const FASTPFOR_ALLOWED: bool;

    fn encode_physical<'a>(
        physical: &'a mut PhysicalCodecs,
        values: &'a [Self::Value],
        physical_enc: PhysicalEncoder,
    ) -> MltResult<(PhysicalEncoding, &'a [u8])> {
        Ok(match physical_enc {
            PhysicalEncoder::None => (PhysicalEncoding::None, Self::none(physical, values)),
            PhysicalEncoder::VarInt => (PhysicalEncoding::VarInt, Self::varint(physical, values)),
            PhysicalEncoder::FastPFOR => (
                PhysicalEncoding::FastPFor256,
                Self::fastpfor(physical, values)?,
            ),
        })
    }

    #[cfg(target_endian = "little")]
    fn none<'a>(_physical: &'a mut PhysicalCodecs, values: &'a [Self::Value]) -> &'a [u8] {
        cast_slice(values)
    }

    #[cfg(not(target_endian = "little"))]
    fn none<'a>(physical: &'a mut PhysicalCodecs, values: &'a [Self::Value]) -> &'a [u8] {
        physical.u8_tmp.clear();
        for &v in values {
            physical.u8_tmp.extend_from_slice(&v.to_le_bytes());
        }
        &physical.u8_tmp
    }

    fn varint<'a>(physical: &'a mut PhysicalCodecs, values: &'a [Self::Value]) -> &'a [u8] {
        // encode_var writes to a stack buffer; avoids the Vec<u8> allocation
        // that encode_var_vec() would produce for every value.
        physical.u8_tmp.clear();
        let mut buf = [0u8; 10];
        for &v in values {
            let n = v.into().encode_var(&mut buf);
            physical.u8_tmp.extend_from_slice(&buf[..n]);
        }
        &physical.u8_tmp
    }

    fn fastpfor<'a>(
        physical: &'a mut PhysicalCodecs,
        values: &'a [Self::Value],
    ) -> MltResult<&'a [u8]>;
}

pub(crate) struct U32Physical;
struct U64Physical;

impl PhysicalIntStreamKind for U32Physical {
    type Value = u32;
    const FASTPFOR_ALLOWED: bool = true;

    fn fastpfor<'a>(
        physical: &'a mut PhysicalCodecs,
        values: &'a [Self::Value],
    ) -> MltResult<&'a [u8]> {
        physical.u8_tmp.clear();
        if !values.is_empty() {
            physical.u32_tmp.clear();
            physical.fastpfor.encode(values, &mut physical.u32_tmp)?;
            for word in &mut physical.u32_tmp {
                *word = word.to_be();
            }
            physical
                .u8_tmp
                .extend_from_slice(cast_slice(&physical.u32_tmp));
        }
        Ok(&physical.u8_tmp)
    }
}

impl PhysicalIntStreamKind for U64Physical {
    type Value = u64;
    const FASTPFOR_ALLOWED: bool = false;

    fn fastpfor<'a>(
        _physical: &'a mut PhysicalCodecs,
        _values: &'a [Self::Value],
    ) -> MltResult<&'a [u8]> {
        Err(UnsupportedPhysicalEncoding("FastPFOR on u64"))?
    }
}

trait LogicalIntStreamKind {
    type Input;
    type Physical: PhysicalIntStreamKind;

    fn encode_logical<'a>(
        logical: &'a mut LogicalCodecs,
        values: &'a [Self::Input],
        logical_enc: LogicalEncoder,
    ) -> MltResult<(
        LogicalEncoding,
        &'a [<Self::Physical as PhysicalIntStreamKind>::Value],
    )> {
        Ok(match logical_enc {
            LogicalEncoder::None => (LogicalEncoding::None, Self::none(logical, values)),
            LogicalEncoder::Delta => (LogicalEncoding::Delta, Self::delta(logical, values)),
            LogicalEncoder::Rle => Self::rle(logical, values)?,
            LogicalEncoder::DeltaRle => Self::delta_rle(logical, values)?,
        })
    }

    fn none<'a>(
        logical: &'a mut LogicalCodecs,
        values: &'a [Self::Input],
    ) -> &'a [<Self::Physical as PhysicalIntStreamKind>::Value];

    fn delta<'a>(
        logical: &'a mut LogicalCodecs,
        values: &'a [Self::Input],
    ) -> &'a [<Self::Physical as PhysicalIntStreamKind>::Value];

    fn rle<'a>(
        logical: &'a mut LogicalCodecs,
        values: &'a [Self::Input],
    ) -> MltResult<(
        LogicalEncoding,
        &'a [<Self::Physical as PhysicalIntStreamKind>::Value],
    )>;

    fn delta_rle<'a>(
        logical: &'a mut LogicalCodecs,
        values: &'a [Self::Input],
    ) -> MltResult<(
        LogicalEncoding,
        &'a [<Self::Physical as PhysicalIntStreamKind>::Value],
    )>;
}

struct U32Stream;
struct I32Stream;
struct U64Stream;
struct I64Stream;

impl LogicalIntStreamKind for U32Stream {
    type Input = u32;
    type Physical = U32Physical;

    fn none<'a>(_logical: &'a mut LogicalCodecs, values: &'a [Self::Input]) -> &'a [u32] {
        values
    }

    fn delta<'a>(logical: &'a mut LogicalCodecs, values: &'a [Self::Input]) -> &'a [u32] {
        encode_zigzag_delta(cast_slice::<u32, i32>(values), &mut logical.u32_tmp)
    }

    fn rle<'a>(
        logical: &'a mut LogicalCodecs,
        values: &'a [Self::Input],
    ) -> MltResult<(LogicalEncoding, &'a [u32])> {
        let meta = apply_rle(values, values.len(), &mut logical.u32_tmp)?;
        Ok((LogicalEncoding::Rle(meta), &logical.u32_tmp))
    }

    fn delta_rle<'a>(
        logical: &'a mut LogicalCodecs,
        values: &'a [Self::Input],
    ) -> MltResult<(LogicalEncoding, &'a [u32])> {
        let data = encode_zigzag_delta(cast_slice::<u32, i32>(values), &mut logical.u32_tmp);
        let meta = apply_rle(data, values.len(), &mut logical.u32_tmp2)?;
        Ok((LogicalEncoding::DeltaRle(meta), &logical.u32_tmp2))
    }
}

impl LogicalIntStreamKind for I32Stream {
    type Input = i32;
    type Physical = U32Physical;

    fn none<'a>(logical: &'a mut LogicalCodecs, values: &'a [Self::Input]) -> &'a [u32] {
        encode_zigzag(values, &mut logical.u32_tmp)
    }

    fn delta<'a>(logical: &'a mut LogicalCodecs, values: &'a [Self::Input]) -> &'a [u32] {
        encode_zigzag_delta(values, &mut logical.u32_tmp)
    }

    fn rle<'a>(
        logical: &'a mut LogicalCodecs,
        values: &'a [Self::Input],
    ) -> MltResult<(LogicalEncoding, &'a [u32])> {
        let data = encode_zigzag(values, &mut logical.u32_tmp);
        let meta = apply_rle(data, values.len(), &mut logical.u32_tmp2)?;
        Ok((LogicalEncoding::Rle(meta), &logical.u32_tmp2))
    }

    fn delta_rle<'a>(
        logical: &'a mut LogicalCodecs,
        values: &'a [Self::Input],
    ) -> MltResult<(LogicalEncoding, &'a [u32])> {
        let data = encode_zigzag_delta(values, &mut logical.u32_tmp);
        let meta = apply_rle(data, values.len(), &mut logical.u32_tmp2)?;
        Ok((LogicalEncoding::DeltaRle(meta), &logical.u32_tmp2))
    }
}

impl LogicalIntStreamKind for U64Stream {
    type Input = u64;
    type Physical = U64Physical;

    fn none<'a>(_logical: &'a mut LogicalCodecs, values: &'a [Self::Input]) -> &'a [u64] {
        values
    }

    fn delta<'a>(logical: &'a mut LogicalCodecs, values: &'a [Self::Input]) -> &'a [u64] {
        encode_zigzag_delta(cast_slice::<u64, i64>(values), &mut logical.u64_tmp)
    }

    fn rle<'a>(
        logical: &'a mut LogicalCodecs,
        values: &'a [Self::Input],
    ) -> MltResult<(LogicalEncoding, &'a [u64])> {
        let meta = apply_rle(values, values.len(), &mut logical.u64_tmp)?;
        Ok((LogicalEncoding::Rle(meta), &logical.u64_tmp))
    }

    fn delta_rle<'a>(
        logical: &'a mut LogicalCodecs,
        values: &'a [Self::Input],
    ) -> MltResult<(LogicalEncoding, &'a [u64])> {
        let data = encode_zigzag_delta(cast_slice::<u64, i64>(values), &mut logical.u64_tmp);
        let meta = apply_rle(data, values.len(), &mut logical.u64_tmp2)?;
        Ok((LogicalEncoding::DeltaRle(meta), &logical.u64_tmp2))
    }
}

impl LogicalIntStreamKind for I64Stream {
    type Input = i64;
    type Physical = U64Physical;

    fn none<'a>(logical: &'a mut LogicalCodecs, values: &'a [Self::Input]) -> &'a [u64] {
        encode_zigzag(values, &mut logical.u64_tmp)
    }

    fn delta<'a>(logical: &'a mut LogicalCodecs, values: &'a [Self::Input]) -> &'a [u64] {
        encode_zigzag_delta(values, &mut logical.u64_tmp)
    }

    fn rle<'a>(
        logical: &'a mut LogicalCodecs,
        values: &'a [Self::Input],
    ) -> MltResult<(LogicalEncoding, &'a [u64])> {
        let data = encode_zigzag(values, &mut logical.u64_tmp);
        let meta = apply_rle(data, values.len(), &mut logical.u64_tmp2)?;
        Ok((LogicalEncoding::Rle(meta), &logical.u64_tmp2))
    }

    fn delta_rle<'a>(
        logical: &'a mut LogicalCodecs,
        values: &'a [Self::Input],
    ) -> MltResult<(LogicalEncoding, &'a [u64])> {
        let data = encode_zigzag_delta(values, &mut logical.u64_tmp);
        let meta = apply_rle(data, values.len(), &mut logical.u64_tmp2)?;
        Ok((LogicalEncoding::DeltaRle(meta), &logical.u64_tmp2))
    }
}

fn write_profiled_int_stream<L: LogicalIntStreamKind>(
    values: &[L::Input],
    profile: &DataProfile,
    ctx: &StreamCtx<'_>,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    use LogicalEncoding as LE;
    let stream = ctx.stream_type;
    // FIXME: does StreamMeta encode values.len() or vals1.len()?
    if let Some(int_enc) = enc.override_int_enc(ctx) {
        let (logical, vals1) = L::encode_logical(&mut codecs.logical, values, int_enc.logical)?;
        let (phys, vals2) =
            L::Physical::encode_physical(&mut codecs.physical, vals1, int_enc.physical)?;
        let meta = StreamMeta::new2(stream, logical, phys, vals1.len())?;
        return write_stream_payload(&mut enc.data, meta, false, vals2);
    }

    if values.is_empty() {
        let vals1 = L::none(&mut codecs.logical, values);
        let vals2 = L::Physical::none(&mut codecs.physical, vals1);
        let meta = StreamMeta::new2(stream, LE::None, PhysicalEncoding::None, vals1.len())?;
        return write_stream_payload(&mut enc.data, meta, false, vals2);
    }

    let Codecs { logical, physical } = codecs;
    let mut alt = enc.try_alternatives();

    if profile.delta_is_beneficial() && (profile.rle_is_viable() || profile.delta_rle_is_viable()) {
        let (logical, values) = L::delta_rle(logical, values)?;
        write_alternatives::<L::Physical>(&mut alt, physical, values, logical, stream)?;
    }
    if profile.delta_is_beneficial() {
        let values = L::delta(logical, values);
        write_alternatives::<L::Physical>(&mut alt, physical, values, LE::Delta, stream)?;
    }
    if profile.rle_is_viable() {
        let (logical, values) = L::rle(logical, values)?;
        write_alternatives::<L::Physical>(&mut alt, physical, values, logical, stream)?;
    }
    let values = L::none(logical, values);
    write_alternatives::<L::Physical>(&mut alt, physical, values, LE::None, stream)
}

fn write_alternatives<P: PhysicalIntStreamKind>(
    alt: &mut crate::encoder::writer::AltSession<'_>,
    physical: &mut PhysicalCodecs,
    values: &[P::Value],
    logical: LogicalEncoding,
    stream_type: StreamType,
) -> MltResult<()> {
    use PhysicalEncoding as PE;
    if P::FASTPFOR_ALLOWED {
        alt.with(|enc| {
            let meta = StreamMeta::new2(stream_type, logical, PE::FastPFor256, values.len())?;
            write_stream_payload(&mut enc.data, meta, false, P::fastpfor(physical, values)?)
        })?;
    }
    alt.with(|enc| {
        let meta = StreamMeta::new2(stream_type, logical, PE::VarInt, values.len())?;
        write_stream_payload(&mut enc.data, meta, false, P::varint(physical, values))
    })
}

/// Write a `u32` integer stream: use the explicit encoder if configured,
/// otherwise compete all pruned candidates and keep the shortest.
pub(crate) fn write_u32_stream(
    values: &[u32],
    ctx: &StreamCtx<'_>,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    let profile = DataProfile::from_values::<i32>(values);
    write_profiled_int_stream::<U32Stream>(values, &profile, ctx, enc, codecs)
}

/// Write a `u64` integer stream.
pub(crate) fn write_u64_stream(
    values: &[u64],
    ctx: &StreamCtx<'_>,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    let profile = DataProfile::from_values::<i64>(values);
    write_profiled_int_stream::<U64Stream>(values, &profile, ctx, enc, codecs)
}

/// Write an `i32` integer stream.
///
/// Zigzag-encodes the values for candidate pruning but encodes the original
/// signed values via the logical encoder's `encode_i32s`.
pub(crate) fn write_i32_stream(
    values: &[i32],
    ctx: &StreamCtx<'_>,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    let data = encode_zigzag(values, &mut codecs.logical.u32_tmp);
    let profile = DataProfile::from_values::<i32>(data);
    write_profiled_int_stream::<I32Stream>(values, &profile, ctx, enc, codecs)
}

/// Write an `i64` integer stream.
///
/// Zigzag-encodes the values for candidate pruning but encodes the original
/// signed values via the logical encoder's `encode_i64s`.
pub(crate) fn write_i64_stream(
    values: &[i64],
    ctx: &StreamCtx<'_>,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    let data = encode_zigzag(values, &mut codecs.logical.u64_tmp);
    let profile = DataProfile::from_values::<i64>(data);
    write_profiled_int_stream::<I64Stream>(values, &profile, ctx, enc, codecs)
}
