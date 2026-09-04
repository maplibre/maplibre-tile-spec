mod analyze;
mod encode01;
#[cfg(feature = "unstable-v2")]
mod encode02;
#[cfg(all(not(test), feature = "arbitrary"))]
mod fuzzing;
mod geometry;
mod id;
pub(crate) mod model;
#[cfg(feature = "unstable-v2")]
pub(crate) mod names02;
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
#[cfg(feature = "unstable-v2")]
pub use model::WireVersion;
#[cfg(feature = "__private")]
pub use model::{
    ColumnKind, CurveParams, ExplicitEncoder, FloatEncoding, StagedLayer, StrEncoding, StreamCtx,
};
pub use model::{EncodedUnknown, EncoderConfig};
#[cfg(all(test, not(feature = "__private")))]
pub(crate) use model::{ExplicitEncoder, FloatEncoding, StagedLayer, StrEncoding};
#[cfg(any(test, feature = "__private"))]
pub use optimizer::Presence;
pub use optimizer::encode_tile;
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
