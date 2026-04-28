use crate::MltResult;
use crate::decoder::{PhysicalEncoding, StreamMeta, StreamType};
use crate::encoder::Encoder;
use crate::encoder::model::StreamCtx;
use crate::encoder::stream::codecs::Codecs;
use crate::encoder::stream::{DataProfile, IntEncoder};

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

pub(crate) fn write_u32_stream_as(
    values: &[u32],
    stream_type: StreamType,
    enc_type: IntEncoder,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    let (logical, values) = codecs.logical.encode_u32(values, enc_type.logical)?;
    let (phys, vals) = codecs.physical.encode_u32(values, enc_type.physical)?;
    let meta = StreamMeta::new2(stream_type, logical, phys, values.len())?;
    write_stream_payload(&mut enc.data, meta, false, vals)
}

pub(crate) fn write_i32_stream_as(
    values: &[i32],
    stream_type: StreamType,
    enc_type: IntEncoder,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    let (logical, values) = codecs.logical.encode_i32(values, enc_type.logical)?;
    let (phys, vals) = codecs.physical.encode_u32(values, enc_type.physical)?;
    let meta = StreamMeta::new2(stream_type, logical, phys, values.len())?;
    write_stream_payload(&mut enc.data, meta, false, vals)
}

pub(crate) fn write_u64_stream_as(
    values: &[u64],
    stream_type: StreamType,
    enc_type: IntEncoder,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    let (logical, values) = codecs.logical.encode_u64(values, enc_type.logical)?;
    let (phys, vals) = codecs.physical.encode_u64(values, enc_type.physical)?;
    let meta = StreamMeta::new2(stream_type, logical, phys, values.len())?;
    write_stream_payload(&mut enc.data, meta, false, vals)
}

pub(crate) fn write_i64_stream_as(
    values: &[i64],
    stream_type: StreamType,
    enc_type: IntEncoder,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    let (logical, data) = codecs.logical.encode_i64(values, enc_type.logical)?;
    let (phys, vals) = codecs.physical.encode_u64(data, enc_type.physical)?;
    let meta = StreamMeta::new2(stream_type, logical, phys, data.len())?;
    write_stream_payload(&mut enc.data, meta, false, vals)
}

pub(crate) fn write_alternatives(
    enc: &mut Encoder,
    codecs: &mut Codecs,
    candidates: impl IntoIterator<Item = IntEncoder>,
    mut write_one: impl FnMut(&mut Encoder, &mut Codecs, IntEncoder) -> MltResult<()>,
) -> MltResult<()> {
    let mut alt = enc.try_alternatives();
    for cand in candidates {
        alt.with(|enc| write_one(enc, codecs, cand))?;
    }
    Ok(())
}

/// Write a `u32` integer stream: use the explicit encoder if configured,
/// otherwise compete all pruned candidates and keep the shortest.
pub(crate) fn write_u32_stream(
    values: &[u32],
    ctx: &StreamCtx<'_>,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    let stream_type = ctx.stream_type;
    if let Some(int_enc) = enc.override_int_enc(ctx) {
        write_u32_stream_as(values, stream_type, int_enc, enc, codecs)?;
    } else {
        let candidates = DataProfile::prune_candidates::<i32>(values);
        write_alternatives(enc, codecs, candidates, |enc, codecs, cand| {
            write_u32_stream_as(values, stream_type, cand, enc, codecs)
        })?;
    }
    Ok(())
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
    let stream_type = ctx.stream_type;
    if let Some(int_enc) = enc.override_int_enc(ctx) {
        write_i32_stream_as(values, stream_type, int_enc, enc, codecs)?;
    } else {
        let profiled = codecs.logical.encode_zigzag_i32(values);
        let candidates = DataProfile::prune_candidates::<i32>(profiled);
        write_alternatives(enc, codecs, candidates, |enc, codecs, cand| {
            write_i32_stream_as(values, stream_type, cand, enc, codecs)
        })?;
    }
    Ok(())
}

/// Write a `u64` integer stream.
pub(crate) fn write_u64_stream(
    values: &[u64],
    ctx: &StreamCtx<'_>,
    enc: &mut Encoder,
    codecs: &mut Codecs,
) -> MltResult<()> {
    let stream_type = ctx.stream_type;
    if let Some(int_enc) = enc.override_int_enc(ctx) {
        write_u64_stream_as(values, stream_type, int_enc, enc, codecs)?;
    } else {
        let candidates = DataProfile::prune_candidates::<i64>(values);
        write_alternatives(enc, codecs, candidates, |enc, codecs, cand| {
            write_u64_stream_as(values, stream_type, cand, enc, codecs)
        })?;
    }
    Ok(())
}
