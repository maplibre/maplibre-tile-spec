mod serialize;
pub use serialize::*;
pub(crate) mod formatter;
mod parse;

pub(crate) use formatter::{FmtOptVec, OptSeq, OptSeqOpt};
use num_traits::CheckedAdd;
pub(crate) use parse::*;

use crate::errors::AsMltError as _;
use crate::v01::RawPresence;
use crate::{Decoder, MltError, MltResult};

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

/// Apply an optional present bitmap to a vector of values.
/// If the presence stream is absent (non-optional column), all values are wrapped in Some.
/// If present, values are interleaved with None according to the bitmap.
pub fn apply_present<T>(
    presence: RawPresence<'_>,
    values: Vec<T>,
    dec: &mut Decoder,
) -> MltResult<Vec<Option<T>>> {
    let present: Vec<bool> = if let Some(p) = presence.0 {
        p.decode_bools(dec)?
    } else {
        let mut result = dec.alloc::<Option<T>>(values.len())?;
        result.extend(values.into_iter().map(Some));
        return Ok(result);
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

pub fn strings_to_lengths<S: AsRef<str>>(values: &[S]) -> MltResult<Vec<u32>> {
    Ok(values
        .iter()
        .map(|s| u32::try_from(s.as_ref().len()))
        .collect::<Result<Vec<_>, _>>()?)
}
