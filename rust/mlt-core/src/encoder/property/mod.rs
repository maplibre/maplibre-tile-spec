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
pub use shared_dict::StringGroup;
pub(crate) use shared_dict::{
    StringStatsBuilder, apply_string_groups, string_stats_without_hashes,
};
