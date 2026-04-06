mod encode_stream;
pub(crate) use encode_stream::dedup_strings;
mod encoder;
mod logical;
mod model;
mod optimizer;
mod physical;
#[cfg(test)]
mod tests;
mod write;

pub use encoder::IntEncoder;
#[cfg(any(test, feature = "__private"))]
pub use logical::LogicalEncoder;
#[cfg(not(any(test, feature = "__private")))]
pub(crate) use logical::LogicalEncoder;
pub use model::*;
pub use optimizer::DataProfile;
#[cfg(any(test, feature = "__private"))]
pub use physical::PhysicalEncoder;
pub(crate) use write::{
    do_write_u32, do_write_u64, write_i32_stream, write_i64_stream, write_precomputed_u32,
    write_u32_stream, write_u64_stream,
};
