mod analyze;
#[cfg(all(not(test), feature = "arbitrary"))]
pub mod fuzzing;
mod geometry;
mod id;
mod into_tile;
mod iterators;
mod layer;
mod limits;
mod model;
mod model01;
#[cfg(feature = "unstable-v2")]
mod model02;
mod property;
mod root01;
#[cfg(feature = "unstable-v2")]
mod root02;
pub(crate) mod stream;

// ── Public API ────────────────────────────────────────────────────────────────

// ── Crate-internal re-exports ─────────────────────────────────────────────────
// Allow internal modules to keep using `crate::decoder::*` paths without
// reaching into sub-module paths explicitly.
pub(crate) use geometry::{Geometry, RawGeometry};
pub use geometry::{GeometryType, GeometryValues};
pub use id::ParsedId;
// pub (not pub(crate)) so __private module can re-export it
pub(crate) use id::{Id, RawId, RawIdValue};
pub use iterators::{
    ColNames, ColumnRef, FeatureRef, Layer01FeatureIter, LendingIterator, PropName, PropNamesIter,
    PropValueRef,
};
pub use limits::{Decoder, Parser};
pub use model::{Layer, Layer01, ParsedLayer, ParsedLayer01, Unknown};
pub(crate) use model01::Column;
pub use model01::ColumnType;
#[cfg(feature = "unstable-v2")]
pub(crate) use model02::{ColumnType02, DataType02, GeoLayout, LayerLayout, Presence02};
// Re-export strings sub-module so encoder can use `crate::decoder::strings::*`
pub(crate) use property::strings;
pub(crate) use property::{
    DictRange, ParsedProperty, ParsedScalar, ParsedSharedDict, ParsedSharedDictItem, ParsedStrings,
    Property, RawFloats, RawFloatsEncoding, RawFsstData, RawPlainData, RawPresence, RawProperty,
    RawScalar, RawSharedDict, RawSharedDictEncoding, RawSharedDictItem, RawStrings,
    RawStringsEncoding,
};
pub(crate) use stream::model::{
    BoolLogical, DictionaryType, FloatLogical, IntEncoding, IntLogical, LengthType,
    LogicalCombination, LogicalEncoding, LogicalTechnique, LogicalValue, Morton, OffsetType,
    PhysicalEncoding, RawStream, RleLayout, RleMeta, StreamMeta, StreamType, ValueKind,
    VertexLogical,
};
