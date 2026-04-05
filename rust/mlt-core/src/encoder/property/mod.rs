mod compare;
pub(crate) mod encode;
mod model;
mod shared_dict;
mod strings;
#[cfg(test)]
mod tests;

pub use model::{
    StagedProperty, StagedScalar, StagedSharedDict, StagedSharedDictItem, StagedStrings,
};
pub use shared_dict::{StringGroup, group_string_properties};
