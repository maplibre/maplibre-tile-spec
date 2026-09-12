pub(crate) mod encode;
mod model;
pub(crate) mod shared_dict;
#[cfg(feature = "unstable-v2")]
mod shared_dict02;
mod strings;
#[cfg(test)]
mod tests;

pub use model::{
    StagedOptScalar, StagedProperty, StagedScalar, StagedSharedDict, StagedSharedDictItem,
    StagedStrings,
};
