mod encode_stream;
pub(crate) use encode_stream::dedup_strings;
pub mod encoder;
mod model;
mod optimizer;
mod physical;
#[cfg(test)]
mod tests;
mod write;
pub use encoder::{FsstStrEncoder, IntEncoder};
pub use model::*;
pub use optimizer::DataProfile;
#[cfg(any(test, feature = "__private"))]
pub use physical::PhysicalEncoder;
pub(crate) use write::{
    do_write_u32, do_write_u64, write_i8_stream, write_i32_stream, write_i64_stream,
    write_precomputed_u32, write_u8_stream, write_u32_stream, write_u64_stream,
};
