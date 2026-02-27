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
use serde_json::{Number, Value};

use crate::MltError;

/// Convert f32 to `GeoJSON` value: finite as number, non-finite as string per issue #978.
pub fn f32_to_json(f: f32) -> Value {
    if f.is_nan() {
        Value::String("f32::NAN".to_owned())
    } else if f == f32::INFINITY {
        Value::String("f32::INFINITY".to_owned())
    } else if f == f32::NEG_INFINITY {
        Value::String("f32::NEG_INFINITY".to_owned())
    } else {
        Number::from_f64(f64::from(f)).expect("finite f32").into()
    }
}

/// Convert f64 to `GeoJSON` value: finite as number, non-finite as string per issue #978.
pub fn f64_to_json(f: f64) -> Value {
    if f.is_nan() {
        Value::String("f64::NAN".to_owned())
    } else if f == f64::INFINITY {
        Value::String("f64::INFINITY".to_owned())
    } else if f == f64::NEG_INFINITY {
        Value::String("f64::NEG_INFINITY".to_owned())
    } else {
        Number::from_f64(f).expect("finite f64").into()
    }
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
