mod encode;
mod model;
mod optimizer;
mod owned;
mod serialize;
mod strings;

pub(crate) use encode::write_properties;
pub use model::{
    PresenceKind, PropertyKind, StagedProperty, StagedScalar, StagedSharedDict,
    StagedSharedDictItem, StagedStrings,
};
pub use optimizer::{EncodeProperties, StringGroup, group_string_properties};
