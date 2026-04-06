mod analyze;
mod compare;
#[cfg(all(not(test), feature = "arbitrary"))]
mod fuzzing;
mod geometry;
mod id;
pub(crate) mod model;
mod optimizer;
mod property;
mod sort;
mod stream;
#[cfg(any(test, feature = "__private"))]
mod tests;
mod tile;
mod unknown;
mod writer;

#[cfg(not(feature = "__private"))]
pub(crate) use geometry::VertexBufferType;
#[cfg(feature = "__private")]
pub use geometry::VertexBufferType;
#[cfg(not(feature = "__private"))]
pub(crate) use id::IdWidth;
#[cfg(feature = "__private")]
pub use id::IdWidth;
#[cfg(feature = "__private")]
pub use model::{ColumnKind, ExplicitEncoder, StagedLayer, StagedLayer01, StrEncoding, StreamCtx};
pub use model::{EncodedUnknown, EncoderConfig};
#[cfg(all(test, not(feature = "__private")))]
pub(crate) use model::{ExplicitEncoder, StagedLayer, StagedLayer01, StrEncoding};
pub(crate) use property::*;
#[cfg(feature = "__private")]
pub use property::{StagedProperty, StagedSharedDict};
pub use sort::SortStrategy;
pub(crate) use sort::*;
pub(crate) use stream::*;
#[cfg(feature = "__private")]
pub use stream::{IntEncoder, LogicalEncoder, PhysicalEncoder};
pub use writer::Encoder;
