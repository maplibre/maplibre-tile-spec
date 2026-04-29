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
#[cfg(feature = "__private")]
pub use physical::PhysicalEncoder;
#[cfg(all(test, not(feature = "__private")))]
pub(crate) use physical::PhysicalEncoder;
pub(crate) use write::write_stream_payload;
