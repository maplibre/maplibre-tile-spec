pub(crate) mod encode;
mod model;
mod shared_dict;
mod strings;
#[cfg(test)]
mod tests;

pub use model::{
    StagedOptScalar, StagedProperty, StagedScalar, StagedSharedDict, StagedSharedDictItem,
    StagedStrings,
};
