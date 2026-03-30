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
pub use optimizer::{EncodeProperties, PropertyProfile};
pub(crate) use scalars::scalar_match;
pub use scalars::{
    EncodedScalarFam, IdentityFam, OptionFam, ParsedScalarFam, RawScalarFam, Scalar, ScalarFamily,
    ScalarMapFn, StagedScalarFam,
};
pub use strings::{build_staged_shared_dict, encode_shared_dict_prop};
