mod codecs;
pub(crate) use codecs::LogicalCodecs;
#[cfg(feature = "__private")]
pub use codecs::{Codecs, PhysicalCodecs};
#[cfg(not(feature = "__private"))]
pub(crate) use codecs::{Codecs, PhysicalCodecs};

mod encode_stream;
pub(crate) use encode_stream::dedup_strings;

mod encoder;
pub use encoder::IntEncoder;
pub(crate) mod logical;
#[cfg(feature = "__private")]
pub use logical::LogicalEncoder;
#[cfg(all(test, not(feature = "__private")))]
pub(crate) use logical::LogicalEncoder;

mod model;
#[cfg(any(test, feature = "__private"))]
pub use model::*;

mod optimizer;
mod physical;
#[cfg(feature = "__private")]
pub use physical::PhysicalEncoder;
#[cfg(all(test, not(feature = "__private")))]
pub(crate) use physical::PhysicalEncoder;

#[cfg(test)]
mod tests;

pub(crate) mod write;
pub(crate) use write::{LogicalIntCodec, LogicalIntStreamKind, write_stream_payload};
