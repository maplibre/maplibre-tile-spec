pub(crate) mod encode;
mod model;
mod shared_dict;
#[cfg(feature = "unstable-v2")]
mod shared_dict02;
mod strings;
#[cfg(test)]
mod tests;

pub(crate) use shared_dict::group_string_properties;

pub use crate::encoder::property::model::{
    StagedOptScalar, StagedProperty, StagedScalar, StagedSharedDict, StagedSharedDictItem,
    StagedStrings,
};
