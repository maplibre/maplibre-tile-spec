mod analyze;
mod cost;
mod encode01;
#[cfg(feature = "unstable-v2")]
mod encode02;
#[cfg(all(not(test), feature = "arbitrary"))]
mod fuzzing;
mod geometry;
mod id;
pub(crate) mod model;
#[cfg(feature = "unstable-v2")]
mod mvalue;
#[cfg(feature = "unstable-v2")]
mod nested;
mod optimizer;
mod property;
mod sort;
mod source;
mod stream;
#[cfg(any(test, feature = "__private"))]
mod tests;
mod tile;
mod unknown;
mod writer;

#[cfg(not(feature = "__private"))]
pub(crate) use geometry::VertexBufferType;
#[cfg(all(test, not(feature = "__private")))]
pub(crate) use model::{ExplicitEncoder, FloatEncoding, StagedLayer, StrEncoding};
#[cfg(all(feature = "unstable-v2", not(feature = "__private")))]
pub(crate) use mvalue::{StagedMValue, StagedValues};
#[cfg(all(feature = "unstable-v2", not(feature = "__private")))]
pub(crate) use nested::{
    StagedInterior, StagedLeaf, StagedList, StagedNested, StagedNode, StagedStruct,
};
pub(crate) use optimizer::encode_layer;
pub(crate) use property::*;
pub(crate) use source::LayerSource;
pub(crate) use stream::*;

#[cfg(feature = "__private")]
pub use crate::encoder::geometry::VertexBufferType;
pub use crate::encoder::geometry::{wound_geometry, wound_layer};
pub use crate::encoder::id::StagedId;
#[cfg(feature = "unstable-v2")]
pub use crate::encoder::model::WireVersion;
pub use crate::encoder::model::{
    Client, EncodedUnknown, EncoderConfig, Network, Profile, Tessellation,
};
#[cfg(feature = "__private")]
pub use crate::encoder::model::{
    ColumnKind, CurveParams, ExplicitEncoder, FloatEncoding, StagedLayer, StrEncoding, StreamCtx,
};
#[cfg(all(feature = "unstable-v2", feature = "__private"))]
pub use crate::encoder::mvalue::{StagedMValue, StagedValues};
#[cfg(all(feature = "unstable-v2", feature = "__private"))]
pub use crate::encoder::nested::{
    StagedInterior, StagedLeaf, StagedList, StagedMap, StagedNested, StagedNode, StagedStruct,
};
#[cfg(any(test, feature = "__private"))]
pub use crate::encoder::optimizer::Presence;
#[cfg(feature = "__private")]
pub use crate::encoder::property::{StagedProperty, StagedSharedDict};
pub use crate::encoder::sort::SortStrategy;
#[cfg(feature = "__private")]
pub use crate::encoder::stream::{Codecs, IntEncoder, LogicalEncoder, PhysicalEncoder};
#[cfg(any(test, feature = "__private"))]
pub use crate::encoder::tests::stage_tile;
pub use crate::encoder::writer::Encoder;
