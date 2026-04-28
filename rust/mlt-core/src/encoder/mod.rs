mod analyze;
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
pub use id::StagedId;
#[cfg(feature = "__private")]
pub use model::{ColumnKind, CurveParams, ExplicitEncoder, StagedLayer, StrEncoding, StreamCtx};
#[cfg(all(test, not(feature = "__private")))]
pub(crate) use model::{CurveParams, ExplicitEncoder, StagedLayer, StrEncoding};
pub use model::{EncodedUnknown, EncoderConfig};
#[cfg(any(test, feature = "__private"))]
pub use optimizer::Presence;
pub(crate) use property::*;
#[cfg(feature = "__private")]
pub use property::{StagedProperty, StagedSharedDict};
pub use sort::SortStrategy;
pub(crate) use sort::spatial_sort_likely_to_help;
pub(crate) use stream::*;
#[cfg(feature = "__private")]
pub use stream::{Codecs, IntEncoder, LogicalEncoder, PhysicalEncoder};
#[cfg(any(test, feature = "__private"))]
pub use tests::stage_tile;
pub use writer::Encoder;
