pub(crate) mod encode;
mod model;
mod optimizer;
mod owned;
mod strings;
#[cfg(test)]
mod tests;

pub use model::{
    EncodedFsstData, EncodedPlainData, EncodedStringsEncoding, StagedProperty, StagedScalar,
    StagedSharedDict, StagedSharedDictItem, StagedStrings,
};
pub use optimizer::{StringGroup, group_string_properties};
