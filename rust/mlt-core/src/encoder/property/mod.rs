pub(crate) mod encode;
mod model;
mod optimizer;
mod owned;
mod serialize;
mod strings;
#[cfg(test)]
mod tests;

pub use model::{
    EncodedFsstData, EncodedPlainData, EncodedStringsEncoding, PropertyKind, StagedProperty,
    StagedScalar, StagedSharedDict, StagedSharedDictItem, StagedStrings,
};
pub use optimizer::{StringGroup, group_string_properties};
