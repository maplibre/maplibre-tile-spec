use crate::MltResult;
use crate::codecs::zigzag::encode_zigzag;
use crate::decoder::{IntEncoding, LogicalEncoding, StreamMeta, StreamType};
use crate::encoder::Encoder;
use crate::encoder::model::StreamCtx;
use crate::encoder::stream::physical::PhysicalEncoder;
use crate::encoder::stream::{DataProfile, IntEncoder};
// ─── inner helpers ────────────────────────────────────────────────────────────
//
// Each `do_write_*` function encodes one stream with a single, already-chosen
// `IntEncoder` and writes the stream header + bytes directly into `enc.data`.
//
// Strategy (two scratch buffers, no allocation after warm-up):
//   1. Logical step  — write the logically-transformed integer sequence into
//      `enc.tmp_u32` / `enc.tmp_u64` (cleared on entry by the logical encoder).
//   2. Physical step — compress those integers into bytes in `enc.tmp_u8`
//      (cleared on entry by the physical encoder).
//   3. Write the stream header to `enc.data` (byte-length is now known from
//      `enc.tmp_u8.len()`), then copy `enc.tmp_u8` into `enc.data`.
//
// This avoids both the intermediate `Vec` allocations of the old approach *and*
// the `memmove` that would be required to insert the header before already-written data.

/// Write the physical payload already in `enc.tmp_u8` to `enc.data`,
/// prefixed by the stream header.
///
/// Reads `enc.tmp_u8.len()` as the byte-length written into the header.
#[inline]
fn write_header_then_scratch(
    stream_type: StreamType,
    int_encoding: IntEncoding,
    num_values: u32,
    enc: &mut Encoder,
) -> MltResult<()> {
    let byte_length = u32::try_from(enc.tmp_u8.len())?;
    StreamMeta::new(stream_type, int_encoding, num_values).write_to(
        &mut enc.data,
        false,
        byte_length,
    )?;
    enc.data.extend_from_slice(&enc.tmp_u8);
    Ok(())
}

pub(crate) fn do_write_u32(
    values: &[u32],
    stream_type: StreamType,
    enc_type: IntEncoder,
    enc: &mut Encoder,
) -> MltResult<()> {
    let logical_encoding = enc_type.logical.encode_u32s(values, &mut enc.tmp_u32)?;
    let num_values = u32::try_from(enc.tmp_u32.len())?;
    let physical_encoding = enc_type
        .physical
        .encode_u32s(&enc.tmp_u32, &mut enc.tmp_u8)?;
    let e = IntEncoding::new(logical_encoding, physical_encoding);
    write_header_then_scratch(stream_type, e, num_values, enc)
}

pub(crate) fn do_write_i32(
    values: &[i32],
    stream_type: StreamType,
    enc_type: IntEncoder,
    enc: &mut Encoder,
) -> MltResult<()> {
    let logical_encoding = enc_type.logical.encode_i32s(values, &mut enc.tmp_u32)?;
    let num_values = u32::try_from(enc.tmp_u32.len())?;
    let physical_encoding = enc_type
        .physical
        .encode_u32s(&enc.tmp_u32, &mut enc.tmp_u8)?;
    let e = IntEncoding::new(logical_encoding, physical_encoding);
    write_header_then_scratch(stream_type, e, num_values, enc)
}

pub(crate) fn do_write_u64(
    values: &[u64],
    stream_type: StreamType,
    enc_type: IntEncoder,
    enc: &mut Encoder,
) -> MltResult<()> {
    let logical_encoding = enc_type.logical.encode_u64s(values, &mut enc.tmp_u64)?;
    let num_values = u32::try_from(enc.tmp_u64.len())?;
    let physical_encoding = enc_type
        .physical
        .encode_u64s(&enc.tmp_u64, &mut enc.tmp_u8)?;
    let e = IntEncoding::new(logical_encoding, physical_encoding);
    write_header_then_scratch(stream_type, e, num_values, enc)
}

pub(crate) fn do_write_i64(
    values: &[i64],
    stream_type: StreamType,
    enc_type: IntEncoder,
    enc: &mut Encoder,
) -> MltResult<()> {
    let logical_encoding = enc_type.logical.encode_i64s(values, &mut enc.tmp_u64)?;
    let num_values = u32::try_from(enc.tmp_u64.len())?;
    let physical_encoding = enc_type
        .physical
        .encode_u64s(&enc.tmp_u64, &mut enc.tmp_u8)?;
    let e = IntEncoding::new(logical_encoding, physical_encoding);
    write_header_then_scratch(stream_type, e, num_values, enc)
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
        // Zigzag-encode into tmp_u32 for pruning; prune_candidates returns an owned Vec
        // so the borrow ends before the loop calls do_write_i32 (which overwrites tmp_u32).
        encode_zigzag(values, &mut enc.tmp_u32);
        let candidates = DataProfile::prune_candidates::<i32>(&enc.tmp_u32);
        let mut alt = enc.try_alternatives();
        for cand in candidates {
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
        encode_zigzag(values, &mut enc.tmp_u64);
        let candidates = DataProfile::prune_candidates::<i64>(&enc.tmp_u64);
        let mut alt = enc.try_alternatives();
        for cand in candidates {
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
    let num_values = u32::try_from(values.len())?;
    let write_one = |phys: PhysicalEncoder, enc: &mut Encoder| -> MltResult<()> {
        let physical_encoding = phys.encode_u32s(values, &mut enc.tmp_u8)?;
        let e = IntEncoding::new(logical_encoding, physical_encoding);
        write_header_then_scratch(ctx.stream_type, e, num_values, enc)
    };
    if let Some(int_enc) = enc.override_int_enc(ctx) {
        write_one(int_enc.physical, enc)?;
    } else {
        let mut alt = enc.try_alternatives();
        for cand in DataProfile::prune_candidates::<i32>(values) {
            alt.with(|enc| write_one(cand.physical, enc))?;
        }
    }
    Ok(())
}
