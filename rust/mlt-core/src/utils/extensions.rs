use num_traits::CheckedAdd;

use crate::errors::AsMltError as _;
use crate::{MltError, MltResult};

pub trait SetOptionOnce<T> {
    fn set_once(&mut self, value: T) -> MltResult<()>;
}

impl<T> SetOptionOnce<T> for Option<T> {
    fn set_once(&mut self, value: T) -> MltResult<()> {
        if self.replace(value).is_some() {
            Err(MltError::DuplicateValue)
        } else {
            Ok(())
        }
    }
}

/// Perform checked addition of two values, returning an error if any overflow occurs.
#[inline]
pub fn checked_sum2<T: CheckedAdd + Copy>(v1: T, v2: T) -> MltResult<T> {
    v1.checked_add(&v2).or_overflow()
}

/// Perform checked addition of three values, returning an error if any overflow occurs.
#[inline]
pub fn checked_sum3<T: CheckedAdd + Copy>(v1: T, v2: T, v3: T) -> MltResult<T> {
    v1.checked_add(&v2)
        .and_then(|sum| sum.checked_add(&v3))
        .or_overflow()
}
