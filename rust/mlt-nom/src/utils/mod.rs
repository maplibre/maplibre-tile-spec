mod encode;
//pub(crate) use encode::*;
// mod serialize;
// pub use serialize::*;
mod parse;
pub(crate) use parse::*;
mod decode;
pub(crate) use decode::*;
mod formater;
pub(crate) use formater::{OptSeq, OptSeqOpt, fmt_byte_array};

use crate::MltError;

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
