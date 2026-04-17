use wide::u32x8;

use crate::decoder::GridParams;
use crate::{Coord32, Decoder, MltError, MltResult};

const LANES: usize = 8;

// ── Bit interleaving ─────────────────────────────────────────────────────────

/// Interleave the lower 16 bits of `x` and `y` into a 32-bit Morton code.
///
/// Even bit positions (0, 2, 4, …) encode `x`; odd positions (1, 3, 5, …)
/// encode `y`. Spatially adjacent `(x, y)` pairs produce numerically
/// adjacent codes, giving Z-order locality when used as a sort key.
#[must_use]
pub fn interleave_bits(x: u32, y: u32) -> u32 {
    // Spread each input's lower 16 bits into every other bit position, then
    // OR the two together: x occupies even positions (0, 2, 4, …) and y
    // occupies odd positions (1, 3, 5, …).
    let mut sx = x & 0xFFFF;
    sx = (sx | (sx << 8)) & 0x00FF_00FF;
    sx = (sx | (sx << 4)) & 0x0F0F_0F0F;
    sx = (sx | (sx << 2)) & 0x3333_3333;
    sx = (sx | (sx << 1)) & 0x5555_5555;

    let mut sy = y & 0xFFFF;
    sy = (sy | (sy << 8)) & 0x00FF_00FF;
    sy = (sy | (sy << 4)) & 0x0F0F_0F0F;
    sy = (sy | (sy << 2)) & 0x3333_3333;
    sy = (sy | (sy << 1)) & 0x5555_5555;

    sx | (sy << 1)
}

/// Compute a Z-order (Morton) sort key from signed integer coordinates.
///
/// `shift` is applied to both axes before bit-interleaving to move the
/// coordinate origin into the non-negative range. It should be computed
/// once across the entire feature set (typically `min.unsigned_abs()` when
/// `min < 0`, else `0`) so that the keys are comparable across features.
///
/// Each shifted component is truncated to 16 bits before interleaving, so
/// the returned key fits in a `u32` (32 interleaved bits). This is
/// sufficient for any tile coordinate system with extent ≤ 65 535.
#[must_use]
pub fn morton_sort_key(c: Coord32, params: GridParams) -> u32 {
    debug_assert!((1..=16).contains(&params.bits));
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "shift brings value into [0, extent]; masked to 16 bits immediately after"
    )]
    let sx = ((i64::from(c.x) + i64::from(params.shift)) as u32) & 0xFFFF;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "shift brings value into [0, extent]; masked to 16 bits immediately after"
    )]
    let sy = ((i64::from(c.y) + i64::from(params.shift)) as u32) & 0xFFFF;
    interleave_bits(sx, sy)
}

// ── Encoder ─────────────────────────────────────────────────────────────────

/// Encode a single `(x, y)` coordinate pair to its Z-order (Morton) code.
///
/// `params.bits` (≤ 16) bits are used per axis; `params.shift` is added to each
/// component before interleaving so that negative coordinates map to non-negative values.
pub fn encode_morton(x: i32, y: i32, params: GridParams) -> MltResult<u32> {
    let sx = u32::try_from(i64::from(x) + i64::from(params.shift))?;
    let sy = u32::try_from(i64::from(y) + i64::from(params.shift))?;
    let mut code = 0u32;
    for i in 0..params.bits {
        // bits is capped at 16, so 2*i+1 ≤ 31 — no shift overflow.
        code |= ((sx >> i) & 1) << (2 * i);
        code |= ((sy >> i) & 1) << (2 * i + 1);
    }
    Ok(code)
}

/// Compute `ZOrderCurve` parameters from the vertex value range.
///
/// Returns a [`GridParams`] whose `bits` and `shift` match Java's `SpaceFillingCurve` implementation.
pub fn z_order_params(vertices: &[i32]) -> MltResult<GridParams> {
    let min_v = vertices.iter().copied().min().unwrap_or(0);
    let max_v = vertices.iter().copied().max().unwrap_or(0);
    let shift: u32 = if min_v < 0 { min_v.unsigned_abs() } else { 0 };
    let tile_extent = i64::from(max_v) + i64::from(shift);
    let bits = if let Ok(extent) = u32::try_from(tile_extent) {
        // ceil(log2(extent + 1)), matching Java's Math.ceil(Math.log(...) / Math.log(2)).
        // Computed with integer arithmetic: for te >= 1, this equals `u32::BITS - te.leading_zeros()`.
        // Capped at 16: Morton codes are u32, so each axis may use at most 16 bits.
        let required_bits = u32::BITS - extent.leading_zeros();
        if required_bits > 16 {
            return Err(MltError::VertexMortonNotCompatibleWithExtent {
                extent,
                required_bits,
            });
        }
        required_bits
    } else {
        0u32
    };
    Ok(GridParams { bits, shift })
}

/// Delta-encode a sorted slice of Morton codes: `[codes[0], codes[1]-codes[0], ...]`.
/// Clears `target` and fills it with the delta-encoded values.
#[inline]
pub fn morton_deltas(codes: &[u32], target: &mut Vec<u32>) {
    target.clear();
    let Some(&first) = codes.first() else {
        return;
    };
    target.extend(std::iter::once(first).chain(codes.windows(2).map(|w| w[1] - w[0])));
}

// ── Decoder ─────────────────────────────────────────────────────────────────

/// Decode a single Morton code to a `Coord32`, applying the coordinate shift.
#[inline]
fn decode_morton_one(morton_code: u32, params: GridParams) -> Coord32 {
    let mut x = 0u32;
    let mut y = 0u32;
    for i in 0..params.bits {
        let bit_mask = 1u32 << (2 * i);
        x |= (morton_code & bit_mask) >> i;
        y |= ((morton_code >> 1) & bit_mask) >> i;
    }
    Coord32 {
        x: x.cast_signed() - params.shift.cast_signed(),
        y: y.cast_signed() - params.shift.cast_signed(),
    }
}

/// Decode delta-encoded Morton codes to flat `[x0, y0, x1, y1, ...]`, charging `dec` for the output.
///
/// Each input value is a signed delta (stored as u32 with wrapping arithmetic)
/// relative to the previous Morton code. The sequential prefix sum is computed
/// in chunks of 8 into a stack-allocated buffer, which is then SIMD-decoded.
/// This keeps the working set in registers / L1 cache.
pub fn decode_morton_delta(
    data: &[u32],
    params: GridParams,
    dec: &mut Decoder,
) -> MltResult<Vec<i32>> {
    let alloc_size = data.len() * 2;
    let mut out = dec.alloc(alloc_size)?;
    let shift_vec = u32x8::splat(params.shift);

    let mut prev = 0i32;
    let mut chunks = data.chunks_exact(LANES);

    for chunk in chunks.by_ref() {
        // Sequential prefix sum into a stack buffer — no heap allocation.
        let mut buf = [0u32; LANES];
        for (b, &d) in buf.iter_mut().zip(chunk.iter()) {
            prev = prev.wrapping_add(d.cast_signed());
            *b = prev.cast_unsigned();
        }
        decode_morton_chunk(buf, params.bits, shift_vec, &mut out);
    }

    // Scalar tail for any codes that didn't fill a full SIMD chunk.
    for &d in chunks.remainder() {
        prev = prev.wrapping_add(d.cast_signed());
        let coord = decode_morton_one(prev.cast_unsigned(), params);
        out.push(coord.x);
        out.push(coord.y);
    }

    dec.adjust_alloc(&out, alloc_size)?;
    Ok(out)
}

/// Decode Morton codes (no delta) to flat `[x0, y0, x1, y1, ...]`, charging `dec` for the output.
///
/// Processes 8 codes at a time with `wide::u32x8`. Each lane extracts the
/// compacted even-bit (x) and odd-bit (y) components in parallel, then applies
/// the coordinate shift. A scalar tail handles any remaining codes.
pub fn decode_morton_codes(
    data: &[u32],
    params: GridParams,
    dec: &mut Decoder,
) -> MltResult<Vec<i32>> {
    let alloc_size = data.len() * 2;
    let mut out = dec.alloc(alloc_size)?;
    let shift_vec = u32x8::splat(params.shift);

    let mut chunks = data.chunks_exact(LANES);

    for chunk in chunks.by_ref() {
        let buf = [
            chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
        ];
        decode_morton_chunk(buf, params.bits, shift_vec, &mut out);
    }

    // Scalar tail for any codes that didn't fill a full SIMD chunk.
    for &code in chunks.remainder() {
        let coord = decode_morton_one(code, params);
        out.push(coord.x);
        out.push(coord.y);
    }

    dec.adjust_alloc(&out, alloc_size)?;
    Ok(out)
}

/// SIMD-decode a chunk of exactly 8 resolved Morton codes into the output buffer.
///
/// Each code has already been resolved to its absolute value (no delta pending).
/// Even-indexed bits encode x, odd-indexed bits encode y.
#[inline]
fn decode_morton_chunk(buf: [u32; LANES], num_bits: u32, shift_vec: u32x8, out: &mut Vec<i32>) {
    let codes = u32x8::from(buf);
    // Odd bits become even after shifting right by 1, giving the y component.
    let codes_y = codes >> 1;

    let mut x_vec = u32x8::ZERO;
    let mut y_vec = u32x8::ZERO;

    for i in 0..num_bits {
        // Mask for the bit position 2*i in the original Morton code.
        let bit_mask = u32x8::splat(1u32 << (2 * i));
        // Extract bit 2*i from each code and shift it down to position i.
        x_vec |= (codes & bit_mask) >> i;
        y_vec |= (codes_y & bit_mask) >> i;
    }

    let xs: [u32; LANES] = (x_vec - shift_vec).into();
    let ys: [u32; LANES] = (y_vec - shift_vec).into();

    for lane in 0..LANES {
        out.push(xs[lane].cast_signed());
        out.push(ys[lane].cast_signed());
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoder::GridParams;
    use crate::test_helpers::dec;

    const fn c(x: i32, y: i32) -> Coord32 {
        Coord32 { x, y }
    }

    // ── interleave_bits / morton_sort_key ─────────────────────────────────────

    /// Spread the lower 16 bits of `tx` into the even bit positions (0, 2, 4, …)
    /// of a 32-bit word, inserting a 0 between every original bit.
    fn spread_bits(mut tx: u32) -> u32 {
        tx = (tx | (tx << 8)) & 0x00FF_00FF;
        tx = (tx | (tx << 4)) & 0x0F0F_0F0F;
        tx = (tx | (tx << 2)) & 0x3333_3333;
        tx = (tx | (tx << 1)) & 0x5555_5555;
        tx
    }

    /// Compact the bits at even positions (0, 2, 4, …) of `tx` into the lower
    /// 16 bits, discarding the interleaved zeros.
    fn compact_bits(mut tx: u32) -> u32 {
        tx &= 0x5555_5555;
        tx = (tx | (tx >> 1)) & 0x3333_3333;
        tx = (tx | (tx >> 2)) & 0x0F0F_0F0F;
        tx = (tx | (tx >> 4)) & 0x00FF_00FF;
        tx = (tx | (tx >> 8)) & 0x0000_FFFF;
        tx
    }

    #[test]
    fn spread_then_compact_is_identity() {
        for x in 0u32..=0xFFFF {
            assert_eq!(compact_bits(spread_bits(x)), x, "round-trip failed for {x}");
        }
    }

    #[test]
    fn spread_bits_places_bit0_at_position0() {
        assert_eq!(spread_bits(1), 1);
    }

    #[test]
    fn spread_bits_places_bit1_at_position2() {
        assert_eq!(spread_bits(2), 4);
    }

    #[test]
    fn spread_bits_places_bit2_at_position4() {
        assert_eq!(spread_bits(4), 16);
    }

    #[test]
    fn origin_maps_to_zero() {
        assert_eq!(
            morton_sort_key(c(0, 0), GridParams { shift: 0, bits: 16 }),
            0
        );
    }

    #[test]
    fn x_axis_produces_even_bits() {
        // x=1, y=0  →  only bit 0 of x is set → Morton bit 0 set → code = 1
        assert_eq!(
            morton_sort_key(c(1, 0), GridParams { shift: 0, bits: 16 }),
            1
        );
        // x=2, y=0  →  only bit 1 of x is set → Morton bit 2 set → code = 4
        assert_eq!(
            morton_sort_key(c(2, 0), GridParams { shift: 0, bits: 16 }),
            4
        );
    }

    #[test]
    fn y_axis_produces_odd_bits() {
        // x=0, y=1  →  only bit 0 of y is set → Morton bit 1 set → code = 2
        assert_eq!(
            morton_sort_key(c(0, 1), GridParams { shift: 0, bits: 16 }),
            2
        );
        // x=0, y=2  →  only bit 1 of y is set → Morton bit 3 set → code = 8
        assert_eq!(
            morton_sort_key(c(0, 2), GridParams { shift: 0, bits: 16 }),
            8
        );
    }

    #[test]
    fn negative_coords_shift_correctly() {
        // Shifting (-1, -1) by 1 maps to (0, 0) → Morton code 0
        assert_eq!(
            morton_sort_key(c(-1, -1), GridParams { shift: 1, bits: 16 }),
            0
        );
        // Shifting (-1, 0) by 1 maps to (0, 1) → Morton code 2
        assert_eq!(
            morton_sort_key(c(-1, 0), GridParams { shift: 1, bits: 16 }),
            2
        );
    }

    #[test]
    fn spatial_locality_z_order() {
        // After shifting, (0,0) < (1,0) < (0,1) < (1,1) in Z-order
        let k00 = morton_sort_key(c(0, 0), GridParams { shift: 0, bits: 16 });
        let k10 = morton_sort_key(c(1, 0), GridParams { shift: 0, bits: 16 });
        let k01 = morton_sort_key(c(0, 1), GridParams { shift: 0, bits: 16 });
        let k11 = morton_sort_key(c(1, 1), GridParams { shift: 0, bits: 16 });
        assert!(k00 < k10);
        assert!(k10 < k01);
        assert!(k01 < k11);
    }

    #[test]
    fn interleave_round_trips_via_deinterleave() {
        // Reconstruct x and y from interleaved bits and verify round-trip.
        for x in 0u32..16 {
            for y in 0u32..16 {
                let code = interleave_bits(x, y);
                let mut rx = 0u32;
                let mut ry = 0u32;
                for bit in 0..16 {
                    rx |= ((code >> (2 * bit)) & 1) << bit;
                    ry |= ((code >> (2 * bit + 1)) & 1) << bit;
                }
                assert_eq!(rx, x, "x mismatch for ({x}, {y})");
                assert_eq!(ry, y, "y mismatch for ({x}, {y})");
            }
        }
    }

    // ── Morton encode/decode tests ────────────────────────────────────────────

    const NUM_BITS: u32 = 15;
    const COORD_SHIFT: u32 = 1 << (NUM_BITS - 1); // 16384

    /// Interleave `x` and `y` into a single Morton code using 15 bits per component.
    ///
    /// Even bit positions encode `x`, odd positions encode `y`.
    /// This is the inverse of [`decode_morton_codes`] / [`decode_morton_delta`].
    #[must_use]
    #[inline]
    pub fn encode_morton_15(x: u32, y: u32) -> u32 {
        let mut code = 0u32;
        for bit in 0..15 {
            code |= ((x >> bit) & 1) << (2 * bit);
            code |= ((y >> bit) & 1) << (2 * bit + 1);
        }
        code
    }

    fn morton_params() -> GridParams {
        GridParams {
            bits: NUM_BITS,
            shift: COORD_SHIFT,
        }
    }

    // --- decode_morton_codes tests ---

    #[test]
    fn test_decode_morton_codes_empty() {
        assert!(
            decode_morton_codes(&[], morton_params(), &mut dec())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn test_decode_morton_codes_origin() {
        // Morton code for (COORD_SHIFT, COORD_SHIFT) should decode to (0, 0).
        let code = encode_morton_15(COORD_SHIFT, COORD_SHIFT);
        let decoded = decode_morton_codes(&[code], morton_params(), &mut dec()).unwrap();
        assert_eq!(decoded, [0, 0]);
    }

    #[test]
    fn test_decode_morton_codes_known_values() {
        // x=1, y=2 (pre-shift) → decoded (1 - COORD_SHIFT, 2 - COORD_SHIFT)
        let x: u32 = 1;
        let y: u32 = 2;
        let code = encode_morton_15(x, y);
        let expected_x = x.cast_signed() - COORD_SHIFT.cast_signed();
        let expected_y = y.cast_signed() - COORD_SHIFT.cast_signed();
        let decoded = decode_morton_codes(&[code], morton_params(), &mut dec()).unwrap();
        assert_eq!(decoded, [expected_x, expected_y]);
    }

    #[test]
    fn test_decode_morton_codes_scalar_tail() {
        // 3 codes — exercises the scalar tail path (< 8 codes).
        let pairs = [(0u32, 1u32), (2, 3), (4, 5)];
        let codes: Vec<u32> = pairs.iter().map(|&(x, y)| encode_morton_15(x, y)).collect();
        let result = decode_morton_codes(&codes, morton_params(), &mut dec()).unwrap();
        let expected = expected_coords(&pairs);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_decode_morton_codes_full_simd_chunk() {
        // 8 codes — exercises exactly one SIMD chunk, no scalar tail.
        let pairs: [(u32, u32); 8] = [
            (0, 0),
            (1, 0),
            (0, 1),
            (1, 1),
            (2, 3),
            (7, 5),
            (10, 9),
            (15, 15),
        ];
        let codes: Vec<u32> = pairs.iter().map(|&(x, y)| encode_morton_15(x, y)).collect();
        let result = decode_morton_codes(&codes, morton_params(), &mut dec()).unwrap();
        let expected = expected_coords(&pairs);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_decode_morton_codes_simd_plus_tail() {
        // 11 codes — one full SIMD chunk of 8 plus a scalar tail of 3.
        let pairs: Vec<(u32, u32)> = (0..11u32).map(|i| (i * 3 % 100, i * 7 % 100)).collect();
        let codes: Vec<u32> = pairs.iter().map(|&(x, y)| encode_morton_15(x, y)).collect();
        let result = decode_morton_codes(&codes, morton_params(), &mut dec()).unwrap();
        let expected = expected_coords(&pairs);
        assert_eq!(result, expected);
    }

    // --- decode_morton_delta tests ---

    #[test]
    fn test_decode_morton_delta_empty() {
        assert!(
            decode_morton_delta(&[], morton_params(), &mut dec())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn test_decode_morton_delta_identity_with_zero_deltas() {
        // All-zero deltas: every resolved code is 0, which decodes to (-COORD_SHIFT, -COORD_SHIFT).
        let deltas = vec![0u32; 3];
        let result = decode_morton_delta(&deltas, morton_params(), &mut dec()).unwrap();
        let shift = -COORD_SHIFT.cast_signed();
        assert_eq!(result, vec![shift, shift, shift, shift, shift, shift]);
    }

    #[test]
    fn test_decode_morton_delta_matches_codes_after_prefix_sum() {
        // Build a sequence of absolute codes, compute their deltas, then verify that
        // decode_morton_delta produces the same output as decode_morton_codes on the
        // original absolute codes.
        let pairs: Vec<(u32, u32)> = (0..11u32).map(|i| (i * 5 % 200, i * 9 % 200)).collect();
        let codes: Vec<u32> = pairs.iter().map(|&(x, y)| encode_morton_15(x, y)).collect();
        let deltas = signed_deltas(&codes);

        let from_codes = decode_morton_codes(&codes, morton_params(), &mut dec()).unwrap();
        let from_deltas = decode_morton_delta(&deltas, morton_params(), &mut dec()).unwrap();
        assert_eq!(from_codes, from_deltas);
    }

    #[test]
    fn test_decode_morton_delta_scalar_tail() {
        // 3 codes via deltas — scalar tail path only.
        let codes: Vec<u32> = vec![
            encode_morton_15(10, 20),
            encode_morton_15(30, 40),
            encode_morton_15(50, 60),
        ];
        let deltas = signed_deltas(&codes);
        let from_codes = decode_morton_codes(&codes, morton_params(), &mut dec()).unwrap();
        let from_deltas = decode_morton_delta(&deltas, morton_params(), &mut dec()).unwrap();
        assert_eq!(from_codes, from_deltas);
    }

    #[test]
    fn test_decode_morton_delta_wrapping() {
        // A single wrapping delta: start from a large code, subtract more than it — should
        // still round-trip correctly via wrapping arithmetic.
        let code_a = encode_morton_15(500, 300);
        let code_b = encode_morton_15(10, 10); // numerically smaller than code_a
        let delta_b = code_b
            .cast_signed()
            .wrapping_sub(code_a.cast_signed())
            .cast_unsigned();
        assert_eq!(
            decode_morton_delta(&[code_a, delta_b], morton_params(), &mut dec()).unwrap(),
            decode_morton_codes(&[code_a, code_b], morton_params(), &mut dec()).unwrap()
        );
    }

    /// Compute expected decoded `[x0, y0, x1, y1, ...]` from raw (pre-shift) coordinate pairs.
    fn expected_coords(pairs: &[(u32, u32)]) -> Vec<i32> {
        pairs
            .iter()
            .flat_map(|&(x, y)| {
                [
                    x.cast_signed() - COORD_SHIFT.cast_signed(),
                    y.cast_signed() - COORD_SHIFT.cast_signed(),
                ]
            })
            .collect()
    }

    /// Compute wrapping signed deltas between consecutive Morton codes.
    fn signed_deltas(codes: &[u32]) -> Vec<u32> {
        let mut prev = 0i32;
        codes
            .iter()
            .map(|&c| {
                let delta = c.cast_signed().wrapping_sub(prev).cast_unsigned();
                prev = c.cast_signed();
                delta
            })
            .collect()
    }
}
