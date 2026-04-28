mod codecs;
mod encode_stream;
pub(crate) use encode_stream::dedup_strings;
mod encoder;
pub(crate) mod logical;
mod model;
mod optimizer;
mod physical;
#[cfg(test)]
mod tests;
pub(crate) mod write;

#[cfg(feature = "__private")]
pub use codecs::{Codecs, PhysicalCodecs};
#[cfg(not(feature = "__private"))]
pub(crate) use codecs::{Codecs, PhysicalCodecs};
pub use encoder::IntEncoder;
#[cfg(feature = "__private")]
pub use logical::LogicalEncoder;
#[cfg(all(test, not(feature = "__private")))]
pub(crate) use logical::LogicalEncoder;
pub use model::*;
pub use optimizer::DataProfile;
#[cfg(feature = "__private")]
pub use physical::PhysicalEncoder;
#[cfg(not(feature = "__private"))]
pub(crate) use physical::PhysicalEncoder;
pub(crate) use write::{
    PhysicalIntStreamKind, U32Physical, write_bool_stream, write_i32_stream, write_i64_stream,
    write_stream_payload, write_u32_stream, write_u64_stream,
};
