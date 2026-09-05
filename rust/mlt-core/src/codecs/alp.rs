//! Exception-free Adaptive Lossless floating-Point compression (ALP), storing `v` as the integer `i = round(v * 10^e / 10^f)`.
//!
//! The arithmetic deviates from reference ALP, which multiplies by a rounded reciprocal
//! (`v * EXP_ARR[e] * FRAC_ARR[f]`) where this divides exactly.
//! The two disagree on roughly 0.75% of values, so these streams are not bit-interchangeable
//! with a reference implementation, and `docs/encodings.md` states the division as normative.
//! Either way is lossless here: every value is certified against [`decode_one`] before it is kept.

use crate::codecs::float::FloatValue;
use crate::decoder::AlpScale;

/// Powers of ten, indexed by exponent.
/// Stopping at `10^18` keeps `v * 10^e` inside the `i64` range.
const POW10: [f64; 19] = [
    1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16,
    1e17, 1e18,
];

/// The two powers of ten a scale multiplies and divides by, looked up once per column.
///
/// Deliberately two factors rather than one `10^(e - f)`: scaling then dividing rounds twice,
/// and that second rounding is exactly what makes some `(e, f > 0)` exact where `(e - f, 0)`
/// is not. Folding them would collapse the grid [`candidates`] walks.
#[derive(Debug, Clone, Copy)]
pub struct Powers {
    up: f64,
    down: f64,
}

impl Powers {
    /// The factors for `scale`, whose exponents are bounded to `AlpScale::MAX_EXPONENT`.
    #[must_use]
    pub fn of(scale: AlpScale) -> Self {
        Self {
            up: POW10[usize::from(scale.e)],
            down: POW10[usize::from(scale.f)],
        }
    }
}

/// Encode one value, or [`None`] if it does not fit the integer range.
fn encode_one<T: FloatValue>(value: T, powers: Powers) -> Option<i64> {
    let scaled = value.widen() * powers.up / powers.down;
    let rounded = scaled.round_ties_even();
    // The bound below is `i64::MAX` as an f64, past which the cast would saturate.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the bound below keeps the value inside the i64 range"
    )]
    let code = rounded as i64;
    (rounded.is_finite() && rounded.abs() < 9.223_372_036_854_776e18).then_some(code)
}

/// Recover one value.
#[must_use]
pub fn decode_one<T: FloatValue>(code: i64, powers: Powers) -> T {
    #[expect(
        clippy::cast_precision_loss,
        reason = "exactness is verified on encode"
    )]
    let value = code as f64 * powers.down / powers.up;
    T::narrow(value)
}

/// Encode one value, or [`None`] unless it returns bit-for-bit.
fn exact_one<T: FloatValue>(value: T, powers: Powers) -> Option<i64> {
    let code = encode_one(value, powers)?;
    decode_one::<T>(code, powers)
        .same_bits(value)
        .then_some(code)
}

/// Whether `params` carries this one value, the cheapest test that can rule a scale out.
pub fn carries<T: FloatValue>(value: T, params: AlpScale) -> bool {
    exact_one(value, Powers::of(params)).is_some()
}

/// Encode `values` with `params`, or the index of the first value that would not return
/// bit-for-bit, which lets a caller retry that one value against the scales it tries next.
pub fn encode_exact<T: FloatValue>(
    values: &[T],
    params: AlpScale,
    out: &mut Vec<i64>,
) -> Result<(), usize> {
    let powers = Powers::of(params);
    out.clear();
    out.reserve(values.len());
    for (i, &value) in values.iter().enumerate() {
        out.push(exact_one(value, powers).ok_or(i)?);
    }
    Ok(())
}

/// Decode a whole stream.
#[must_use]
pub fn decode<T: FloatValue>(codes: &[i64], params: AlpScale) -> Vec<T> {
    let powers = Powers::of(params);
    codes.iter().map(|&c| decode_one(c, powers)).collect()
}

/// Every `(e, f)` worth trying for `T`, ordered by ascending net scale.
/// `f` never exceeds `e`, since it only divides out trailing zeros that `e` introduced.
///
/// Codes grow tenfold with each net scale, so the first scale that encodes every value is
/// the cheapest one, and a search may stop there rather than walking the whole grid.
/// Within a scale the pairs still differ: double rounding makes some `(e, f > 0)` exact
/// where `(e - f, 0)` is not, so every pair earns its place.
pub fn candidates() -> impl Iterator<Item = AlpScale> {
    (0..=AlpScale::MAX_EXPONENT)
        .flat_map(|net| (net..=AlpScale::MAX_EXPONENT).map(move |e| AlpScale { e, f: e - net }))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    /// The first parameters that fit, which ascending net scale makes the smallest ones.
    fn best<T: FloatValue>(values: &[T]) -> Option<(AlpScale, Vec<i64>)> {
        let mut out = Vec::new();
        candidates().find_map(|params| {
            encode_exact(values, params, &mut out)
                .ok()
                .map(|()| (params, out.clone()))
        })
    }

    #[rstest]
    #[case::one_decimal(&[1.5, 2.5, -3.5])]
    #[case::two_decimals(&[1.25, 100.75, -0.25])]
    #[case::whole_numbers(&[1.0, 2.0, 1e15])]
    #[case::coordinates(&[13.404_954, 52.520_008, -74.006])]
    fn exception_free_parameters_round_trip_bit_for_bit(#[case] values: &[f64]) {
        let (params, codes) = best(values).expect("some parameters fit");
        let decoded: Vec<f64> = decode(&codes, params);
        let bits: Vec<u64> = decoded.iter().map(|v| v.to_bits()).collect();
        let expected: Vec<u64> = values.iter().map(|v| v.to_bits()).collect();
        assert_eq!(bits, expected);
    }

    #[rstest]
    #[case::nan(&[1.5, f64::NAN])]
    #[case::infinity(&[1.5, f64::INFINITY])]
    #[case::neg_infinity(&[1.5, f64::NEG_INFINITY])]
    #[case::negative_zero(&[1.5, -0.0])]
    #[case::too_many_digits(&[f64::MIN_POSITIVE])]
    fn values_that_cannot_return_bit_for_bit_have_no_parameters(#[case] values: &[f64]) {
        assert!(best(values).is_none(), "{values:?}");
    }

    #[test]
    fn f32_values_round_trip_through_the_widened_arithmetic() {
        let values: &[f32] = &[1.5, -2.25, 100.0, 0.125];
        let (params, codes) = best(values).expect("some parameters fit");
        assert_eq!(decode::<f32>(&codes, params), values);
    }

    #[test]
    fn small_magnitude_f32_values_reach_the_exponents_they_need() {
        let values: &[f32] = &[1.234_567_8e-6, 9.876_543e-6, 4.444_444_6e-6];
        let (params, codes) = best(values).expect("some parameters fit");
        assert!(
            params.e > 10,
            "{params:?} should exceed the old f32-only cap"
        );
        assert_eq!(decode::<f32>(&codes, params), values);
    }

    #[test]
    fn every_candidate_keeps_f_no_greater_than_e() {
        assert!(candidates().all(|p| p.f <= p.e && p.e <= AlpScale::MAX_EXPONENT));
    }

    #[test]
    fn candidates_walk_net_exponents_in_ascending_order() {
        let nets: Vec<u8> = candidates().map(AlpScale::net).collect();
        assert!(nets.windows(2).all(|w| w[0] <= w[1]), "{nets:?}");
    }

    #[test]
    fn candidates_cover_every_pair_exactly_once() {
        let mut pairs: Vec<(u8, u8)> = candidates().map(|p| (p.e, p.f)).collect();
        let found = pairs.len();
        pairs.sort_unstable();
        pairs.dedup();
        assert_eq!(pairs.len(), found);
        let max = usize::from(AlpScale::MAX_EXPONENT);
        assert_eq!(found, (max + 1) * (max + 2) / 2);
    }
}
