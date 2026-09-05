//! Exception-free Adaptive Lossless floating-Point compression (ALP), storing `v` as the integer `i = round(v * 10^e / 10^f)`.

use crate::codecs::float::FloatValue;
use crate::decoder::Alp;

/// Powers of ten that are exactly representable, indexed by exponent.
/// Stopping at `10^18` keeps `v * 10^e` inside the `i64` range.
const POW10: [f64; 19] = [
    1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16,
    1e17, 1e18,
];

/// Encode one value, or [`None`] if it does not fit the integer range.
fn encode_one<T: FloatValue>(value: T, params: Alp) -> Option<i64> {
    let scaled = value.widen() * POW10[usize::from(params.e)] / POW10[usize::from(params.f)];
    let rounded = scaled.round_ties_even();
    // The bound below is `i64::MAX` as an f64, past which the cast would saturate.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the bound above keeps the value inside the i64 range"
    )]
    let code = rounded as i64;
    (rounded.is_finite() && rounded.abs() < 9.223_372_036_854_776e18).then_some(code)
}

/// Recover one value.
#[must_use]
pub fn decode_one<T: FloatValue>(code: i64, params: Alp) -> T {
    #[expect(
        clippy::cast_precision_loss,
        reason = "exactness is verified on encode"
    )]
    let value = code as f64 * POW10[usize::from(params.f)] / POW10[usize::from(params.e)];
    T::narrow(value)
}

/// Encode `values` with `params`, or [`None`] if any value would not return bit-for-bit.
pub fn encode_exact<T: FloatValue>(values: &[T], params: Alp, out: &mut Vec<i64>) -> Option<()> {
    out.clear();
    out.reserve(values.len());
    for &value in values {
        let code = encode_one(value, params)?;
        if !decode_one::<T>(code, params).same_bits(value) {
            return None;
        }
        out.push(code);
    }
    Some(())
}

/// Decode a whole stream.
#[must_use]
pub fn decode<T: FloatValue>(codes: &[i64], params: Alp) -> Vec<T> {
    codes.iter().map(|&c| decode_one(c, params)).collect()
}

/// Every `(e, f)` worth trying for `T`, in the order the search should walk them.
/// `f` never exceeds `e`, since it only divides out trailing zeros that `e` introduced.
pub fn candidates<T: FloatValue>() -> impl Iterator<Item = Alp> {
    (0..=T::MAX_EXPONENT).flat_map(|e| (0..=e).map(move |f| Alp { e, f }))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn best<T: FloatValue>(values: &[T]) -> Option<(Alp, Vec<i64>)> {
        let mut out = Vec::new();
        candidates::<T>().find_map(|params| {
            encode_exact(values, params, &mut out).map(|()| (params, out.clone()))
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
    fn every_candidate_keeps_f_no_greater_than_e() {
        assert!(candidates::<f64>().all(|p| p.f <= p.e && p.e <= f64::MAX_EXPONENT));
    }
}
