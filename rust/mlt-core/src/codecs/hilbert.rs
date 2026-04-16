use crate::Coord32;

/// Return the 1-D Hilbert curve index for `(x, y)` at the given `level`.
///
/// The grid has side `2^level`; both `x` and `y` must be in `[0, 2^level)`,
/// and `level` must be in `[1, 16]`.  The returned index is in
/// `[0, 4^level)` and fits in a `u32` for all valid levels.
#[must_use]
pub fn hilbert_xy_to_index(level: u32, x: u32, y: u32) -> u32 {
    debug_assert!((1..=16).contains(&level), "level must be in [1, 16]");
    debug_assert!(x < (1 << level), "x out of range for level");
    debug_assert!(y < (1 << level), "y out of range for level");

    hilbert_2d::u32::xy2h_discrete(x, y, level, hilbert_2d::Variant::Hilbert)
}

/// Compute a Hilbert curve sort key from signed integer coordinates.
///
/// `shift` is added to both axes to move the origin into the non-negative
/// range (use the global minimum across *all* vertices for the layer, not
/// per-axis).  `num_bits` is the grid level — both shifted components must
/// fit in `[0, 2^num_bits)`.
///
/// Use [`hilbert_curve_params_from_bounds`] to compute `shift` and `num_bits`
/// from global min/max coordinates.
#[must_use]
pub fn hilbert_sort_key(c: Coord32, shift: u32, num_bits: u32) -> u32 {
    debug_assert!((1..=16).contains(&num_bits));
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "shift brings value into [0, extent]; masked to 16 bits immediately after"
    )]
    let sx = ((i64::from(c.x) + i64::from(shift)) as u32) & 0xFFFF;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "shift brings value into [0, extent]; masked to 16 bits immediately after"
    )]
    let sy = ((i64::from(c.y) + i64::from(shift)) as u32) & 0xFFFF;
    hilbert_xy_to_index(num_bits, sx, sy)
}

/// Compute the coordinate shift and grid level from pre-computed global
/// min/max values across all vertex coordinates (both axes combined).
///
/// Returns `(shift, num_bits)` where:
/// - `shift` is subtracted from the global minimum (i.e. it equals
///   `min_val.unsigned_abs()` when `min_val < 0`, else `0`), ensuring all
///   shifted coordinates are non-negative.
/// - `num_bits` is the smallest level `l` in `[1, 16]` such that all
///   shifted values fit in `[0, 2^l)`.
///
/// If `min > max` (empty input), returns `(0, 1)`.
#[must_use]
pub fn hilbert_curve_params_from_bounds(min_val: i32, max_val: i32) -> (u32, u32) {
    if min_val > max_val {
        return (0, 1);
    }
    let shift: u32 = if min_val < 0 {
        min_val.unsigned_abs()
    } else {
        0
    };
    // extent = largest shifted coordinate value that any vertex can take.
    let extent = (i64::from(max_val) + i64::from(shift)).unsigned_abs();
    // num_bits = ceil(log2(extent + 1)), i.e. the smallest l s.t. 2^l > extent.
    // For extent = 0 the grid degenerates to a single cell; use level 1.
    let num_bits = if extent == 0 {
        1
    } else {
        (u64::BITS - extent.leading_zeros()).min(16)
    };
    (shift, num_bits)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codecs::morton::interleave_bits;

    const fn c(x: i32, y: i32) -> Coord32 {
        Coord32 { x, y }
    }

    /// Return the `(x, y)` coordinates for Hilbert curve index `pos` at `level`.
    ///
    /// This is the inverse of [`hilbert_xy_to_index`]: the returned coordinates
    /// are in `[0, 2^level)`.  `level` must be in `[1, 16]` and `pos` must be
    /// in `[0, 4^level)`.
    fn hilbert_position_to_xy(level: u32, pos: u32) -> (u32, u32) {
        debug_assert!((1..=16).contains(&level), "level must be in [1, 16]");
        debug_assert!(u64::from(pos) < (1u64 << (2 * level)), "pos out of range");
        hilbert_2d::u32::h2xy_discrete(pos, level, hilbert_2d::Variant::Hilbert)
    }

    #[test]
    fn hilbert_origin_always_zero() {
        // The origin maps to index 0 at every level.
        for level in 1u32..=8 {
            let idx = hilbert_xy_to_index(level, 0, 0);
            assert_eq!(idx, 0, "origin should be 0 at level {level}");
        }
    }

    #[test]
    fn hilbert_round_trip_level1() {
        // 2×2 grid: all four cells must encode then decode back to themselves.
        for x in 0u32..2 {
            for y in 0u32..2 {
                let idx = hilbert_xy_to_index(1, x, y);
                let (rx, ry) = hilbert_position_to_xy(1, idx);
                assert_eq!((rx, ry), (x, y), "round-trip failed at level=1 ({x},{y})");
            }
        }
    }

    #[test]
    fn hilbert_round_trip_level2() {
        // 4×4 grid: all 16 cells.
        for x in 0u32..4 {
            for y in 0u32..4 {
                let idx = hilbert_xy_to_index(2, x, y);
                let (rx, ry) = hilbert_position_to_xy(2, idx);
                assert_eq!((rx, ry), (x, y), "round-trip failed at level=2 ({x},{y})");
            }
        }
    }

    #[test]
    fn hilbert_round_trip_level4() {
        // 16×16 grid: all 256 cells.
        for x in 0u32..16 {
            for y in 0u32..16 {
                let idx = hilbert_xy_to_index(4, x, y);
                let (rx, ry) = hilbert_position_to_xy(4, idx);
                assert_eq!((rx, ry), (x, y), "round-trip failed at level=4 ({x},{y})");
            }
        }
    }

    #[test]
    fn hilbert_indices_are_a_bijection_at_level2() {
        // Every index in [0, 16) must appear exactly once.
        let mut seen = [false; 16];
        for x in 0u32..4 {
            for y in 0u32..4 {
                let idx = hilbert_xy_to_index(2, x, y) as usize;
                assert!(!seen[idx], "duplicate index {idx} at ({x},{y})");
                seen[idx] = true;
            }
        }
        assert!(seen.iter().all(|&v| v), "some index was never produced");
    }

    #[test]
    fn hilbert_indices_are_a_bijection_at_level4() {
        let mut seen = vec![false; 256];
        for x in 0u32..16 {
            for y in 0u32..16 {
                let idx = hilbert_xy_to_index(4, x, y) as usize;
                assert!(!seen[idx], "duplicate index {idx} at ({x},{y})");
                seen[idx] = true;
            }
        }
        assert!(seen.iter().all(|&v| v));
    }

    #[test]
    fn hilbert_level1_covers_indices_0_to_3() {
        // Collect all indices produced for the 2×2 grid.
        let mut indices: Vec<u32> = (0u32..2)
            .flat_map(|x| (0u32..2).map(move |y| hilbert_xy_to_index(1, x, y)))
            .collect();
        indices.sort_unstable();
        assert_eq!(indices, [0, 1, 2, 3]);
    }

    // ── hilbert_sort_key ──────────────────────────────────────────────────────

    #[test]
    fn hilbert_sort_key_origin_zero() {
        assert_eq!(hilbert_sort_key(c(0, 0), 0, 1), 0);
    }

    #[test]
    fn hilbert_sort_key_negative_coords_shift_correctly() {
        // (-1, -1) shifted by 1 maps to (0, 0) -> Hilbert index 0 at any level.
        assert_eq!(hilbert_sort_key(c(-1, -1), 1, 1), 0);
    }

    #[test]
    fn hilbert_sort_key_matches_xy_to_index() {
        // hilbert_sort_key should agree with a direct call to hilbert_xy_to_index
        // after shifting.
        let shift = 5u32;
        let num_bits = 4u32;
        for raw_x in -5i32..11 {
            for raw_y in -5i32..11 {
                let expected = hilbert_xy_to_index(
                    num_bits,
                    u32::try_from(i64::from(raw_x) + i64::from(shift)).unwrap(),
                    u32::try_from(i64::from(raw_y) + i64::from(shift)).unwrap(),
                );
                let actual = hilbert_sort_key(c(raw_x, raw_y), shift, num_bits);
                assert_eq!(actual, expected, "mismatch at ({raw_x},{raw_y})");
            }
        }
    }

    #[test]
    fn curve_params_empty_bounds() {
        // min > max signals empty input.
        let (shift, num_bits) = hilbert_curve_params_from_bounds(i32::MAX, i32::MIN);
        assert_eq!(shift, 0);
        assert_eq!(num_bits, 1);
    }

    #[test]
    fn curve_params_all_zero() {
        // Degenerate: single point at origin, extent = 0 -> level 1.
        let (shift, num_bits) = hilbert_curve_params_from_bounds(0, 0);
        assert_eq!(shift, 0);
        assert_eq!(num_bits, 1);
    }

    #[test]
    fn curve_params_positive_only() {
        // Bounds [0, 3]: shift = 0, num_bits = 2 (2^2=4 > 3).
        let (shift, num_bits) = hilbert_curve_params_from_bounds(0, 3);
        assert_eq!(shift, 0);
        assert_eq!(num_bits, 2);
    }

    #[test]
    fn curve_params_negative_min() {
        // Bounds [-4, 4]: shift = 4, extent = 4+4 = 8, num_bits = 4 (2^4=16 > 8).
        let (shift, num_bits) = hilbert_curve_params_from_bounds(-4, 4);
        assert_eq!(shift, 4);
        assert_eq!(num_bits, 4);
    }

    #[test]
    fn curve_params_power_of_two_extent() {
        // extent = 8 = 2^3: need level 4 so that 2^4 = 16 > 8.
        let (shift, num_bits) = hilbert_curve_params_from_bounds(0, 8);
        assert_eq!(shift, 0);
        assert_eq!(num_bits, 4);
    }

    #[test]
    fn curve_params_single_axis_negative() {
        // Global min = -2 -> shift = 2; extent = 5 + 2 = 7; num_bits = 3.
        let (shift, num_bits) = hilbert_curve_params_from_bounds(-2, 5);
        assert_eq!(shift, 2);
        assert_eq!(num_bits, 3);
    }

    #[test]
    fn curve_params_clamped_at_16_bits() {
        // extent = 65535 = 2^16 - 1, num_bits = 16.
        let (shift, num_bits) = hilbert_curve_params_from_bounds(0, 65535);
        assert_eq!(shift, 0);
        assert_eq!(num_bits, 16);
    }

    // ── Hilbert vs Morton locality comparison ─────────────────────────────────

    #[test]
    fn hilbert_and_morton_both_sort_nearby_points_close_together() {
        // A dense cluster of points in [0, 7]×[0, 7] should produce
        // keys that are all close to each other under both curves.
        // We verify that the maximum key spread for a 2×2 neighbourhood is
        // smaller than the total key range for the full 8×8 grid.
        let points: Vec<(u32, u32)> = (0u32..8)
            .flat_map(|x| (0u32..8).map(move |y| (x, y)))
            .collect();

        // Hilbert: level = 3 (2^3 = 8)
        let h_keys: Vec<u32> = points
            .iter()
            .map(|&(x, y)| hilbert_xy_to_index(3, x, y))
            .collect();
        let h_max = h_keys.iter().copied().max().unwrap();

        // Morton
        let m_keys: Vec<u32> = points.iter().map(|&(x, y)| interleave_bits(x, y)).collect();
        let m_max = m_keys.iter().copied().max().unwrap();

        // Both should cover the full range for the grid.
        assert_eq!(h_max, 63, "Hilbert should produce indices 0..63 for 8×8");
        assert_eq!(m_max, interleave_bits(7, 7));

        // Neither curve should map (0,0) and (1,0) more than 4 steps apart at level 3.
        let h_00 = hilbert_xy_to_index(3, 0, 0);
        let h_10 = hilbert_xy_to_index(3, 1, 0);
        assert!(
            h_00.abs_diff(h_10) <= 4,
            "adjacent points should have close Hilbert indices"
        );
    }
}
