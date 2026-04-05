mod compare;
pub(crate) mod encode;
mod model;
mod optimizer;
mod strings;
#[cfg(test)]
mod tests;

pub use model::{
    StagedProperty, StagedScalar, StagedSharedDict, StagedSharedDictItem, StagedStrings,
};
pub use optimizer::{StringGroup, group_string_properties};
