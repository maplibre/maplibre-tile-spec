mod analyze;
mod decode;
mod encode;
mod fmt;
#[cfg(all(not(test), feature = "arbitrary"))]
mod fuzzing;
mod model;
mod optimizer;
mod owned;
mod scalars;
mod serialize;
mod strings;

pub use model::*;
pub use optimizer::{EncodeProperties, StringGroup, group_string_properties};
pub use strings::encode_shared_dict_prop;
