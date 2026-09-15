use geo_types::Coord;
use wide::u32x8;

use crate::decoder::Morton;
use crate::encoder::model::CurveParams;
use crate::{Decoder, MltError, MltResult};

const LANES: usize = 8;

// ── Bit interleaving ─────────────────────────────────────────────────────────

/// Interleave the lower 16 bits of `x` and `y` into a 32-bit Morton code.
///
/// Even bit positions (0, 2, 4, …) encode `x`; odd positions (1, 3, 5, …)
/// encode `y`. Spatially adjacent `(x, y)` pairs produce numerically
/// adjacent codes, giving Z-order locality when used as a sort key.
#[must_use]
#[inline]
pub fn interleave_bits(coord: Coord<u32>) -> u32 {
    // Spread each input's lower 16 bits into every other bit position, then
    // OR the two together: x occupies even positions (0, 2, 4, …) and y
    // occupies odd positions (1, 3, 5, …).
    let mut sx = coord.x & 0xFFFF;
    sx = (sx | (sx << 8)) & 0x00FF_00FF;
    sx = (sx | (sx << 4)) & 0x0F0F_0F0F;
    sx = (sx | (sx << 2)) & 0x3333_3333;
    sx = (sx | (sx << 1)) & 0x5555_5555;

    let mut sy = coord.y & 0xFFFF;
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
#[inline]
pub fn morton_sort_key(c: Coord<i32>, params: CurveParams) -> u32 {
    debug_assert!(params.bits >= 1);
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
    interleave_bits((sx, sy).into())
}

// ── Encoder ─────────────────────────────────────────────────────────────────
impl Morton {
    /// Compute `ZOrderCurve` parameters from the vertex value range.
    ///
    /// Returns a [`Morton`] whose `bits` and `shift` match Java's
    /// `SpaceFillingCurve` implementation.
    pub fn from_vertices(vertices: &[i32]) -> MltResult<Self> {
        let min_v = vertices.iter().copied().min().unwrap_or(0);
        let max_v = vertices.iter().copied().max().unwrap_or(0);
        let shift: u32 = if min_v < 0 { min_v.unsigned_abs() } else { 0 };
        let tile_extent = i64::from(max_v) + i64::from(shift);
        let extent =
            u32::try_from(tile_extent).expect("infallible: max_v + shift lies in 0..=u32::MAX");
        // ceil(log2(extent + 1)), matching Java's Math.ceil(Math.log(...) / Math.log(2)).
        // Computed with integer arithmetic: for te >= 1, this equals `te.bit_width()`.
        // Capped at 16: Morton codes are u32, so each axis may use at most 16 bits.
        let required_bits = extent.bit_width();
        if required_bits > 16 {
            return Err(MltError::VertexMortonNotCompatibleWithExtent {
                extent,
                required_bits,
            });
        }
        Self::new(required_bits, shift)
    }

    /// Encode a single `(x, y)` coordinate pair to its Z-order (Morton) code.
    ///
    /// `bits` (≤ 16) bits are used per axis; `shift` is added to each
    /// component before interleaving so that negative coordinates map to non-negative values.
    #[inline]
    pub fn encode_morton(self, x: i32, y: i32) -> MltResult<u32> {
        let sx = u32::try_from(i64::from(x) + i64::from(self.shift))?;
        let sy = u32::try_from(i64::from(y) + i64::from(self.shift))?;
        let mut code = 0u32;
        for i in 0..self.bits {
            // bits are capped at 16, so 2*i+1 ≤ 31 - no shift overflow.
            code |= ((sx >> i) & 1) << (2 * i);
            code |= ((sy >> i) & 1) << (2 * i + 1);
        }
        Ok(code)
    }
}

impl Morton {
    /// Decode a single Morton code to a `Coord<i32>`, applying `shift`.
    #[inline]
    fn decode_one(self, morton_code: u32) -> Coord<i32> {
        let mut x = 0u32;
        let mut y = 0u32;
        for i in 0..self.bits {
            let bit_mask = 1u32 << (2 * i);
            x |= (morton_code & bit_mask) >> i;
            y |= ((morton_code >> 1) & bit_mask) >> i;
        }
        Coord::<i32> {
            x: x.wrapping_sub(self.shift).cast_signed(),
            y: y.wrapping_sub(self.shift).cast_signed(),
        }
    }

    /// Decode Morton codes (no delta) to flat `[x0, y0, x1, y1, ...]`, charging `dec` for the output.
    ///
    /// Processes 8 codes at a time with `wide::u32x8`. Each lane extracts the
    /// compacted even-bit (x) and odd-bit (y) components in parallel, then applies
    /// the coordinate shift. A scalar tail handles any remaining codes.
    pub fn decode_codes(self, data: &[u32], dec: &mut Decoder) -> MltResult<Vec<i32>> {
        let alloc_size = data.len() * 2;
        let mut out = dec.alloc(alloc_size)?;
        let shift_vec = u32x8::splat(self.shift);

        let (chunks, remainder) = data.as_chunks::<LANES>();

        for &chunk in chunks {
            self.decode_chunk(chunk, shift_vec, &mut out);
        }

        // Scalar tail for any codes that didn't fill a full SIMD chunk.
        for &code in remainder {
            let coord = self.decode_one(code);
            out.push(coord.x);
            out.push(coord.y);
        }

        dec.adjust_alloc(&out, alloc_size)
            .expect("infallible: two coordinates pushed per code fill alloc_size exactly");
        Ok(out)
    }

    /// Decode delta-encoded Morton codes to flat `[x0, y0, x1, y1, ...]`, charging `dec` for the output.
    ///
    /// Each input value is a signed delta (stored as u32 with wrapping arithmetic)
    /// relative to the previous Morton code. The sequential prefix sum is computed
    /// in chunks of 8 into a stack-allocated buffer, which is then SIMD-decoded.
    /// This keeps the working set in registers / L1 cache.
    pub fn decode_delta(self, data: &[u32], dec: &mut Decoder) -> MltResult<Vec<i32>> {
        let alloc_size = data.len() * 2;
        let mut out = dec.alloc(alloc_size)?;
        let shift_vec = u32x8::splat(self.shift);

        let mut prev = 0i32;
        let (chunks, remainder) = data.as_chunks::<LANES>();

        for chunk in chunks {
            // Sequential prefix sum into a stack buffer - no heap allocation.
            let mut buf = [0u32; LANES];
            for (b, &d) in buf.iter_mut().zip(chunk.iter()) {
                prev = prev.wrapping_add(d.cast_signed());
                *b = prev.cast_unsigned();
            }
            self.decode_chunk(buf, shift_vec, &mut out);
        }

        // Scalar tail for any codes that didn't fill a full SIMD chunk.
        for &d in remainder {
            prev = prev.wrapping_add(d.cast_signed());
            let coord = self.decode_one(prev.cast_unsigned());
            out.push(coord.x);
            out.push(coord.y);
        }

        dec.adjust_alloc(&out, alloc_size)
            .expect("infallible: two coordinates pushed per code fill alloc_size exactly");
        Ok(out)
    }

    /// SIMD-decode a chunk of exactly 8 resolved Morton codes into the output buffer.
    ///
    /// Each code has already been resolved to its absolute value (no delta pending).
    /// Even-indexed bits encode x, odd-indexed bits encode y.
    #[inline]
    fn decode_chunk(self, buf: [u32; LANES], shift_vec: u32x8, out: &mut Vec<i32>) {
        let codes = u32x8::from(buf);
        // Odd bits become even after shifting right by 1, giving the y component.
        let codes_y = codes >> 1;

        let mut x_vec = u32x8::ZERO;
        let mut y_vec = u32x8::ZERO;

        for i in 0..self.bits {
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
}

#[cfg(test)]
mod tests {

    use rstest::rstest;

    use super::*;
    use crate::test_helpers::{dec, starved_dec};

    const fn c(x: i32, y: i32) -> Coord<i32> {
        Coord::<i32> { x, y }
    }

    const fn p(shift: u32, bits: u32) -> CurveParams {
        CurveParams { shift, bits }
    }

    // ── interleave_bits / morton_sort_key ─────────────────────────────────────

    fn spread_bits(mut tx: u32) -> u32 {
        tx = (tx | (tx << 8)) & 0x00FF_00FF;
        tx = (tx | (tx << 4)) & 0x0F0F_0F0F;
        tx = (tx | (tx << 2)) & 0x3333_3333;
        tx = (tx | (tx << 1)) & 0x5555_5555;
        tx
    }

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
        assert_eq!(morton_sort_key(c(0, 0), p(0, 16)), 0);
    }

    #[test]
    fn x_axis_produces_even_bits() {
        assert_eq!(
            morton_sort_key(c(1, 0), p(0, 16)),
            1,
            "x bit 0 lands at Morton bit 0"
        );
        assert_eq!(
            morton_sort_key(c(2, 0), p(0, 16)),
            4,
            "x bit 1 lands at Morton bit 2"
        );
    }

    #[test]
    fn y_axis_produces_odd_bits() {
        assert_eq!(
            morton_sort_key(c(0, 1), p(0, 16)),
            2,
            "y bit 0 lands at Morton bit 1"
        );
        assert_eq!(
            morton_sort_key(c(0, 2), p(0, 16)),
            8,
            "y bit 1 lands at Morton bit 3"
        );
    }

    #[test]
    fn negative_coords_shift_correctly() {
        assert_eq!(
            morton_sort_key(c(-1, -1), p(1, 16)),
            0,
            "a shift of 1 maps (-1, -1) onto (0, 0)"
        );
        assert_eq!(
            morton_sort_key(c(-1, 0), p(1, 16)),
            2,
            "a shift of 1 maps (-1, 0) onto (0, 1)"
        );
    }

    #[test]
    fn spatial_locality_z_order() {
        let k00 = morton_sort_key(c(0, 0), p(0, 16));
        let k10 = morton_sort_key(c(1, 0), p(0, 16));
        let k01 = morton_sort_key(c(0, 1), p(0, 16));
        let k11 = morton_sort_key(c(1, 1), p(0, 16));
        assert!(k00 < k10, "(0,0) precedes (1,0) in Z-order");
        assert!(k10 < k01, "(1,0) precedes (0,1) in Z-order");
        assert!(k01 < k11, "(0,1) precedes (1,1) in Z-order");
    }

    #[test]
    fn interleave_round_trips_via_deinterleave() {
        for x in 0u32..16 {
            for y in 0u32..16 {
                let code = interleave_bits((x, y).into());
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
    const COORD_SHIFT: u32 = 1 << (NUM_BITS - 1);
    const MORTON: Morton = Morton {
        bits: NUM_BITS,
        shift: COORD_SHIFT,
    };

    #[must_use]
    #[inline]
    pub fn encode_morton_15(coord: Coord<u32>) -> u32 {
        let mut code = 0u32;
        for bit in 0..15 {
            code |= ((coord.x >> bit) & 1) << (2 * bit);
            code |= ((coord.y >> bit) & 1) << (2 * bit + 1);
        }
        code
    }

    #[test]
    fn test_decode_morton_codes_empty() {
        assert_eq!(
            MORTON.decode_codes(&[], &mut dec()).unwrap(),
            [] as [i32; 0]
        );
    }

    #[test]
    fn test_decode_morton_codes_origin() {
        let code = encode_morton_15((COORD_SHIFT, COORD_SHIFT).into());
        let decoded = MORTON.decode_codes(&[code], &mut dec()).unwrap();
        assert_eq!(
            decoded,
            [0, 0],
            "the code for (COORD_SHIFT, COORD_SHIFT) sits at the origin"
        );
    }

    #[test]
    fn test_decode_morton_codes_known_values() {
        let x: u32 = 1;
        let y: u32 = 2;
        let code = encode_morton_15((x, y).into());
        let expected_x = x.cast_signed() - COORD_SHIFT.cast_signed();
        let expected_y = y.cast_signed() - COORD_SHIFT.cast_signed();
        let decoded = MORTON.decode_codes(&[code], &mut dec()).unwrap();
        assert_eq!(
            decoded,
            [expected_x, expected_y],
            "decoding subtracts COORD_SHIFT from each axis"
        );
    }

    #[test]
    fn test_decode_morton_codes_scalar_tail() {
        let pairs: [Coord<u32>; _] = [(0, 1).into(), (2, 3).into(), (4, 5).into()];
        let codes: Vec<u32> = pairs.iter().map(|&c| encode_morton_15(c)).collect();
        let result = MORTON.decode_codes(&codes, &mut dec()).unwrap();
        let expected = expected_coords(&pairs);
        assert_eq!(result, expected, "3 codes take the scalar tail alone");
    }

    #[test]
    fn test_decode_morton_codes_full_simd_chunk() {
        let pairs: [Coord<u32>; _] = [
            (0, 0).into(),
            (1, 0).into(),
            (0, 1).into(),
            (1, 1).into(),
            (2, 3).into(),
            (7, 5).into(),
            (10, 9).into(),
            (15, 15).into(),
        ];
        let codes: Vec<u32> = pairs.iter().map(|&c| encode_morton_15(c)).collect();
        let result = MORTON.decode_codes(&codes, &mut dec()).unwrap();
        let expected = expected_coords(&pairs);
        assert_eq!(result, expected, "8 codes fill one SIMD chunk with no tail");
    }

    #[test]
    fn test_decode_morton_codes_simd_plus_tail() {
        let pairs: Vec<Coord<u32>> = (0..11u32)
            .map(|i| (i * 3 % 100, i * 7 % 100).into())
            .collect();
        let codes: Vec<u32> = pairs.iter().map(|&c| encode_morton_15(c)).collect();
        let result = MORTON.decode_codes(&codes, &mut dec()).unwrap();
        let expected = expected_coords(&pairs);
        assert_eq!(
            result, expected,
            "11 codes take one SIMD chunk plus a tail of 3"
        );
    }

    #[test]
    fn test_decode_morton_delta_empty() {
        assert_eq!(
            MORTON.decode_delta(&[], &mut dec()).unwrap(),
            [] as [i32; 0]
        );
    }

    #[test]
    fn test_decode_morton_delta_identity_with_zero_deltas() {
        let deltas = vec![0u32; 3];
        let result = MORTON.decode_delta(&deltas, &mut dec()).unwrap();
        let shift = -COORD_SHIFT.cast_signed();
        assert_eq!(
            result,
            vec![shift, shift, shift, shift, shift, shift],
            "zero deltas resolve every code to 0, which decodes to (-COORD_SHIFT, -COORD_SHIFT)"
        );
    }

    #[test]
    fn test_decode_morton_delta_matches_codes_after_prefix_sum() {
        let pairs: Vec<Coord<u32>> = (0..11u32)
            .map(|i| (i * 5 % 200, i * 9 % 200).into())
            .collect();
        let codes: Vec<u32> = pairs.iter().map(|&c| encode_morton_15(c)).collect();
        let deltas = signed_deltas(&codes);

        let from_codes = MORTON.decode_codes(&codes, &mut dec()).unwrap();
        let from_deltas = MORTON.decode_delta(&deltas, &mut dec()).unwrap();
        assert_eq!(
            from_codes, from_deltas,
            "the prefix sum rebuilds the absolute codes"
        );
    }

    #[test]
    fn test_decode_morton_delta_scalar_tail() {
        let codes: Vec<u32> = vec![
            encode_morton_15((10, 20).into()),
            encode_morton_15((30, 40).into()),
            encode_morton_15((50, 60).into()),
        ];
        let deltas = signed_deltas(&codes);
        let from_codes = MORTON.decode_codes(&codes, &mut dec()).unwrap();
        let from_deltas = MORTON.decode_delta(&deltas, &mut dec()).unwrap();
        assert_eq!(
            from_codes, from_deltas,
            "3 deltas take the scalar tail alone"
        );
    }

    #[test]
    fn test_decode_morton_delta_wrapping() {
        let code_a = encode_morton_15((500, 300).into());
        let code_b = encode_morton_15((10, 10).into());
        let delta_b = code_b
            .cast_signed()
            .wrapping_sub(code_a.cast_signed())
            .cast_unsigned();
        assert_eq!(
            MORTON.decode_delta(&[code_a, delta_b], &mut dec()).unwrap(),
            MORTON.decode_codes(&[code_a, code_b], &mut dec()).unwrap(),
            "a delta onto a smaller code wraps and still resolves"
        );
    }

    #[test]
    fn decode_codes_past_the_memory_budget_is_rejected() {
        let err = MORTON
            .decode_codes(&[0, 1], &mut starved_dec())
            .unwrap_err();
        assert!(
            matches!(err, MltError::MemoryLimitExceeded { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn decode_delta_past_the_memory_budget_is_rejected() {
        let err = MORTON
            .decode_delta(&[0, 1], &mut starved_dec())
            .unwrap_err();
        assert!(
            matches!(err, MltError::MemoryLimitExceeded { .. }),
            "{err:?}"
        );
    }

    // ── from_vertices / encode_morton ─────────────────────────────────────────

    #[rstest]
    #[case::empty(&[], 0, 0)]
    #[case::non_negative_needs_no_shift(&[0, 100], 0, 7)]
    #[case::negative_min_shifts_onto_zero(&[-5, 10], 5, 4)]
    #[case::widest_accepted_extent(&[0, 65_535], 0, 16)]
    fn from_vertices_derives_shift_and_bits(
        #[case] vertices: &[i32],
        #[case] shift: u32,
        #[case] bits: u32,
    ) {
        let morton = Morton::from_vertices(vertices).unwrap();
        assert_eq!(
            (morton.shift, morton.bits),
            (shift, bits),
            "shift cancels a negative min, bits covers max + shift"
        );
    }

    #[rstest]
    #[case::past_the_cap_outright(&[0, 65_536])]
    #[case::pushed_past_the_cap_by_the_shift(&[-1, 65_535])]
    fn from_vertices_rejects_an_extent_wider_than_sixteen_bits(#[case] vertices: &[i32]) {
        let err = Morton::from_vertices(vertices).unwrap_err();
        assert!(
            matches!(
                err,
                MltError::VertexMortonNotCompatibleWithExtent {
                    extent: 65_536,
                    required_bits: 17,
                }
            ),
            "{err:?}"
        );
    }

    #[rstest]
    #[case::origin(0, 0)]
    #[case::x_only(1, 0)]
    #[case::y_only(0, 1)]
    #[case::both_axes(500, 300)]
    #[case::negative_coords(-4000, -9000)]
    fn encode_morton_round_trips_through_decode_codes(#[case] x: i32, #[case] y: i32) {
        let code = MORTON.encode_morton(x, y).unwrap();
        assert_eq!(MORTON.decode_codes(&[code], &mut dec()).unwrap(), [x, y]);
    }

    #[rstest]
    #[case::x_below_the_shift(-20_000, 0)]
    #[case::y_below_the_shift(0, -20_000)]
    fn encode_morton_rejects_a_coordinate_the_shift_cannot_lift(#[case] x: i32, #[case] y: i32) {
        let err = MORTON.encode_morton(x, y).unwrap_err();
        assert!(matches!(err, MltError::TryFromIntError(_)), "{err:?}");
    }

    fn expected_coords(pairs: &[Coord<u32>]) -> Vec<i32> {
        pairs
            .iter()
            .flat_map(|&Coord { x, y }| {
                [
                    x.cast_signed() - COORD_SHIFT.cast_signed(),
                    y.cast_signed() - COORD_SHIFT.cast_signed(),
                ]
            })
            .collect()
    }

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
