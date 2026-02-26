mod encode;
pub use encode::*;
mod serialize;
pub use serialize::*;
mod parse;
pub(crate) use parse::*;
mod decode;
pub(crate) use decode::*;
mod formatter;
pub(crate) use formatter::{FmtOptVec, OptSeq, OptSeqOpt, fmt_byte_array};

use crate::MltError;

/// Convert f32 to JSON using the shortest decimal representation (matches Java's `Float.toString()`)
pub fn f32_to_json(f: f32) -> serde_json::Value {
    if f.is_nan() {
        return serde_json::Value::String("f32::NAN".to_string());
    }
    if f.is_infinite() {
        let s = if f > 0.0 {
            "f32::INFINITY"
        } else {
            "f32::NEG_INFINITY"
        };
        return serde_json::Value::String(s.to_string());
    }
    let serialized = &serde_json::to_string(&f).expect("f32 serialization should not fail");
    serde_json::from_str(serialized).expect("serialized f32 should parse as JSON")
}

/// Convert f64 to JSON. NaN and ±Infinity use string tags; finite values delegate to [`f32_to_json`].
#[expect(
    clippy::cast_possible_truncation,
    reason = "f64 stored as f32 in wire format"
)]
pub fn f64_to_json(f: f64) -> serde_json::Value {
    if f.is_nan() {
        return serde_json::Value::String("f64::NAN".to_string());
    }
    if f.is_infinite() {
        let s = if f > 0.0 {
            "f64::INFINITY"
        } else {
            "f64::NEG_INFINITY"
        };
        return serde_json::Value::String(s.to_string());
    }
    f32_to_json(f as f32)
}

pub trait SetOptionOnce<T> {
    fn set_once(&mut self, value: T) -> Result<(), MltError>;
}

impl<T> SetOptionOnce<T> for Option<T> {
    fn set_once(&mut self, value: T) -> Result<(), MltError> {
        if self.replace(value).is_some() {
            Err(MltError::DuplicateValue)
        } else {
            Ok(())
        }
    }
}

/// Apply an optional present bitmap to a vector of values.
/// If present is None (non-optional column), all values are wrapped in Some.
/// If present is Some, values are interleaved with None according to the bitmap.
pub fn apply_present<T>(
    present: Option<Vec<bool>>,
    values: Vec<T>,
) -> Result<Vec<Option<T>>, MltError> {
    let Some(present) = present else {
        return Ok(values.into_iter().map(Some).collect());
    };
    let present_bit_count = present.iter().filter(|&&b| b).count();
    if present_bit_count != values.len() {
        return Err(MltError::PresenceValueCountMismatch(
            present_bit_count,
            values.len(),
        ));
    }
    debug_assert!(
        values.len() <= present.len(),
        "Since the number of present bits is an upper bound on the number of values and equals values.len(), there cannot be more values than entries in the present bitmap"
    );

    let mut result = Vec::with_capacity(present.len());
    let mut val_iter = values.into_iter();
    for p in present {
        result.push(if p { val_iter.next() } else { None });
    }
    Ok(result)
}
