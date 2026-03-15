mod encode;
mod spatial;

pub use encode::*;
use num_traits::CheckedAdd;
pub use spatial::*;
mod serialize;
pub use serialize::*;
mod parse;
pub(crate) use parse::*;
mod decode;
pub use decode::*;
pub(crate) mod formatter;
use std::mem::size_of;

pub(crate) use formatter::{FmtOptVec, OptSeq, OptSeqOpt};
use serde_json::{Number, Value};

use crate::errors::AsMltError as _;
use crate::v01::RawPresence;
use crate::{Decoder, MltError};

/// Convert f32 to `GeoJSON` value: finite as number, non-finite as string per issue #978.
#[must_use]
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
#[must_use]
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
/// If the presence stream is absent (non-optional column), all values are wrapped in Some.
/// If present, values are interleaved with None according to the bitmap.
pub fn apply_present<T>(
    presence: RawPresence<'_>,
    values: Vec<T>,
    dec: &mut Decoder,
) -> Result<Vec<Option<T>>, MltError> {
    let present: Vec<bool> = if let Some(p) = presence.0 {
        p.decode_bools(dec)?
    } else {
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

    let mut result = dec.alloc::<Option<T>>(present.len())?;
    let mut val_iter = values.into_iter();
    for p in present {
        result.push(if p { val_iter.next() } else { None });
    }
    Ok(result)
}

/// Perform checked addition of three values, returning an error if any overflow occurs.
#[inline]
pub fn checked_sum2<T: CheckedAdd + Copy>(v1: T, v2: T) -> Result<T, MltError> {
    v1.checked_add(&v2).or_overflow()
}

/// Perform checked addition of three values, returning an error if any overflow occurs.
#[inline]
pub fn checked_sum3<T: CheckedAdd + Copy>(v1: T, v2: T, v3: T) -> Result<T, MltError> {
    v1.checked_add(&v2)
        .and_then(|sum| sum.checked_add(&v3))
        .or_overflow()
}

pub trait AsUsize: Eq + Copy {
    fn as_usize(&self) -> usize;
}

impl AsUsize for usize {
    #[inline]
    fn as_usize(&self) -> usize {
        *self
    }
}

impl AsUsize for u32 {
    #[inline]
    fn as_usize(&self) -> usize {
        const _: () = {
            // Some day Rust may support usize smaller than u32?
            assert!(
                size_of::<u32>() <= size_of::<usize>(),
                "usize must be able to hold all u32 values"
            );
        };
        usize::try_from(*self).unwrap()
    }
}
