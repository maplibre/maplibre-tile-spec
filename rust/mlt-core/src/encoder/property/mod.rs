pub(crate) mod encode;
mod model;
mod optimizer;
mod owned;
mod serialize;
mod strings;

pub use model::{
    EncodedFsstData, EncodedPlainData, EncodedStringsEncoding, PresenceKind, PropertyKind,
    StagedProperty, StagedScalar, StagedSharedDict, StagedSharedDictItem, StagedStrings,
};
pub use optimizer::{EncodeProperties, StringGroup, group_string_properties};
