mod encode;
//pub(crate) use encode::*;
// mod serialize;
// pub use serialize::*;
mod parse;
pub(crate) use parse::*;
mod decode;
pub(crate) use decode::*;
mod formatter;
pub(crate) use formatter::{OptSeq, OptSeqOpt, fmt_byte_array};

use crate::MltError;

/// Convert f32 to JSON using the shortest decimal representation (matches Java's `Float.toString()`)
pub fn f32_to_json(f: f32) -> serde_json::Value {
    let serialized = &serde_json::to_string(&f).expect("f32 serialization should not fail");
    serde_json::from_str(serialized).expect("serialized f32 should parse as JSON")
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
