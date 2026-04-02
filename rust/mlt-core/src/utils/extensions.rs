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
