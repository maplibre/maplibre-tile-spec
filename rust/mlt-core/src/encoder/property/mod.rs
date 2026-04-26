pub(crate) mod encode;
mod model;
mod shared_dict;
mod strings;
#[cfg(test)]
mod tests;

pub(crate) use encode::{
    write_opt_u32_scalar_col, write_opt_u64_scalar_col, write_u32_scalar_col, write_u64_scalar_col,
};
pub use model::{
    StagedOptScalar, StagedProperty, StagedScalar, StagedSharedDict, StagedSharedDictItem,
    StagedStrings,
};
pub use shared_dict::{StringGroup, group_string_properties};
