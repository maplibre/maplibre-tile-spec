mod codecs;
#[cfg(feature = "unstable-v2")]
mod float_alp;
#[cfg(feature = "unstable-v2")]
mod float_cost;
#[cfg(feature = "unstable-v2")]
mod float_dict;
#[cfg(not(feature = "__private"))]
pub(crate) use codecs::{Codecs, PhysicalCodecs};

pub(crate) use crate::encoder::stream::codecs::LogicalCodecs;
#[cfg(feature = "__private")]
pub use crate::encoder::stream::codecs::{Codecs, PhysicalCodecs};

mod encode_stream;
pub(crate) use encode_stream::dedup_strings;

mod encoder;
pub use crate::encoder::stream::encoder::IntEncoder;
pub(crate) mod logical;
#[cfg(all(test, not(feature = "__private")))]
pub(crate) use logical::LogicalEncoder;

#[cfg(feature = "__private")]
pub use crate::encoder::stream::logical::LogicalEncoder;

mod model;
#[cfg(any(test, feature = "__private"))]
pub use model::*;

mod optimizer;
mod physical;
#[cfg(all(test, not(feature = "__private")))]
pub(crate) use physical::PhysicalEncoder;

#[cfg(feature = "__private")]
pub use crate::encoder::stream::physical::PhysicalEncoder;

#[cfg(test)]
mod tests;

pub(crate) mod write;
pub(crate) use write::{LogicalIntCodec, LogicalIntStreamKind, write_stream_payload};
