use crate::MltResult;
use crate::codecs::zigzag::encode_zigzag;
use crate::decoder::{IntEncoding, LogicalEncoding, StreamMeta, StreamType};
use crate::encoder::model::StreamCtx;
use crate::encoder::stream::{DataProfile, IntEncoder};
use crate::encoder::{EncodedStreamData, Encoder};
// ─── inner helpers ────────────────────────────────────────────────────────────
//
// Each `do_write_*` function encodes one stream with a single, already-chosen
// `IntEncoder` and writes the stream header + bytes directly into `enc.data`.
// No `EncodedStream` is created or passed across function boundaries.

fn write_stream_bytes(
    stream_type: StreamType,
    int_encoding: IntEncoding,
    num_values: u32,
    stream_data: &EncodedStreamData,
    enc: &mut Encoder,
) -> MltResult<()> {
    let byte_length = match stream_data {
        EncodedStreamData::VarInt(v) | EncodedStreamData::Encoded(v) => u32::try_from(v.len())?,
    };
    StreamMeta::new(stream_type, int_encoding, num_values).write_to(enc, false, byte_length)?;
    stream_data.write_to(enc)?;
    Ok(())
}

pub(crate) fn do_write_u32(
    values: &[u32],
    stream_type: StreamType,
    enc_type: IntEncoder,
    enc: &mut Encoder,
) -> MltResult<()> {
    let (physical_u32s, logical_encoding) = enc_type.logical.encode_u32s(values)?;
    let num_values = u32::try_from(physical_u32s.len())?;
    let (stream_data, physical_encoding) = enc_type.physical.encode_u32s(physical_u32s)?;
    let e = IntEncoding::new(logical_encoding, physical_encoding);
    write_stream_bytes(stream_type, e, num_values, &stream_data, enc)
}

pub(crate) fn do_write_i32(
    values: &[i32],
    stream_type: StreamType,
    enc_type: IntEncoder,
    enc: &mut Encoder,
) -> MltResult<()> {
    let (physical_u32s, logical_encoding) = enc_type.logical.encode_i32s(values)?;
    let num_values = u32::try_from(physical_u32s.len())?;
    let (stream_data, physical_encoding) = enc_type.physical.encode_u32s(physical_u32s)?;
    let e = IntEncoding::new(logical_encoding, physical_encoding);
    write_stream_bytes(stream_type, e, num_values, &stream_data, enc)
}

pub(crate) fn do_write_u64(
    values: &[u64],
    stream_type: StreamType,
    enc_type: IntEncoder,
    enc: &mut Encoder,
) -> MltResult<()> {
    let (physical_u64s, logical_encoding) = enc_type.logical.encode_u64s(values)?;
    let num_values = u32::try_from(physical_u64s.len())?;
    let (stream_data, physical_encoding) = enc_type.physical.encode_u64s(physical_u64s)?;
    let e = IntEncoding::new(logical_encoding, physical_encoding);
    write_stream_bytes(stream_type, e, num_values, &stream_data, enc)
}

pub(crate) fn do_write_i64(
    values: &[i64],
    stream_type: StreamType,
    enc_type: IntEncoder,
    enc: &mut Encoder,
) -> MltResult<()> {
    let (physical_u64s, logical_encoding) = enc_type.logical.encode_i64s(values)?;
    let num_values = u32::try_from(physical_u64s.len())?;
    let (stream_data, physical_encoding) = enc_type.physical.encode_u64s(physical_u64s)?;
    let e = IntEncoding::new(logical_encoding, physical_encoding);
    write_stream_bytes(stream_type, e, num_values, &stream_data, enc)
}

// ─── public wrappers ──────────────────────────────────────────────────────────
//
// Each wrapper checks for an explicit encoder override (via `enc.override_int_enc`)
// and falls back to automatic candidate selection via the alternatives' machinery.
//
// These are the functions called by geometry, ID, scalar property, and string
// sub-stream encoders — replacing the previous pattern of:
//   `enc.write_stream(&EncodedStream::encode_*(…, explicit_encoder)?)?`

/// Write a `u32` integer stream: use the explicit encoder if configured,
/// otherwise compete all pruned candidates and keep the shortest.
pub(crate) fn write_u32_stream(
    values: &[u32],
    ctx: &StreamCtx<'_>,
    enc: &mut Encoder,
) -> MltResult<()> {
    let stream_type = ctx.stream_type;
    if let Some(int_enc) = enc.override_int_enc(ctx) {
        do_write_u32(values, stream_type, int_enc, enc)?;
    } else {
        let mut alt = enc.try_alternatives();
        for cand in DataProfile::prune_candidates::<i32>(values) {
            alt.with(|enc| do_write_u32(values, stream_type, cand, enc))?;
        }
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
) -> MltResult<()> {
    let stream_type = ctx.stream_type;
    if let Some(int_enc) = enc.override_int_enc(ctx) {
        do_write_i32(values, stream_type, int_enc, enc)?;
    } else {
        let test_vals = encode_zigzag(values);
        let mut alt = enc.try_alternatives();
        for cand in DataProfile::prune_candidates::<i32>(&test_vals) {
            alt.with(|enc| do_write_i32(values, stream_type, cand, enc))?;
        }
    }
    Ok(())
}

/// Write a `u64` integer stream.
pub(crate) fn write_u64_stream(
    values: &[u64],
    ctx: &StreamCtx<'_>,
    enc: &mut Encoder,
) -> MltResult<()> {
    let stream_type = ctx.stream_type;
    if let Some(int_enc) = enc.override_int_enc(ctx) {
        do_write_u64(values, stream_type, int_enc, enc)?;
    } else {
        let mut alt = enc.try_alternatives();
        for cand in DataProfile::prune_candidates::<i64>(values) {
            alt.with(|enc| do_write_u64(values, stream_type, cand, enc))?;
        }
    }
    Ok(())
}

/// Write an `i64` integer stream.
///
/// Zigzag-encodes the values for candidate pruning but encodes the original
/// signed values via the logical encoder's `encode_i64s`.
pub(crate) fn write_i64_stream(
    values: &[i64],
    ctx: &StreamCtx<'_>,
    enc: &mut Encoder,
) -> MltResult<()> {
    let stream_type = ctx.stream_type;
    if let Some(int_enc) = enc.override_int_enc(ctx) {
        do_write_i64(values, stream_type, int_enc, enc)?;
    } else {
        let test_vals: Vec<u64> = encode_zigzag(values);
        let mut alt = enc.try_alternatives();
        for cand in DataProfile::prune_candidates::<i64>(&test_vals) {
            alt.with(|enc| do_write_i64(values, stream_type, cand, enc))?;
        }
    }
    Ok(())
}

/// Write a pre-logically-encoded `u32` stream, competing physical encoders only.
///
/// Unlike [`write_u32_stream`], no logical transformation is applied.  The
/// `logical_encoding` tag is written verbatim into the stream metadata so the
/// decoder knows what transformation was applied by the caller.
///
/// Use this for streams whose logical step (e.g. `ComponentwiseDelta` or
/// `MortonDelta`) is performed externally before calling this function.
pub(crate) fn write_precomputed_u32(
    values: &[u32],
    logical_encoding: LogicalEncoding,
    ctx: &StreamCtx<'_>,
    enc: &mut Encoder,
) -> MltResult<()> {
    let stream_type = ctx.stream_type;
    if let Some(int_enc) = enc.override_int_enc(ctx) {
        let num_values = u32::try_from(values.len())?;
        let (stream_data, physical_encoding) = int_enc.physical.encode_u32s(values.to_vec())?;
        let e = IntEncoding::new(logical_encoding, physical_encoding);
        write_stream_bytes(stream_type, e, num_values, &stream_data, enc)
    } else {
        let mut alt = enc.try_alternatives();
        for cand in DataProfile::prune_candidates::<i32>(values) {
            alt.with(|enc| {
                let num_values = u32::try_from(values.len())?;
                let (stream_data, physical_encoding) =
                    cand.physical.encode_u32s(values.to_vec())?;
                let e = IntEncoding::new(logical_encoding, physical_encoding);
                write_stream_bytes(stream_type, e, num_values, &stream_data, enc)
            })?;
        }
        Ok(())
    }
}
