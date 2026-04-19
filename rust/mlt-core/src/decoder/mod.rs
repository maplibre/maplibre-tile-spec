mod analyze;
mod column;
#[cfg(all(not(test), feature = "arbitrary"))]
pub mod fuzzing;
mod geometry;
mod id;
mod iterators;
mod layer;
mod model;
mod property;
mod root;
pub(crate) mod stream;
mod tile;

// ── Public API ────────────────────────────────────────────────────────────────

// ── Crate-internal re-exports ─────────────────────────────────────────────────
// Allow internal modules to keep using `crate::decoder::*` paths without
// reaching into sub-module paths explicitly.
pub(crate) use geometry::{Geometry, RawGeometry};
pub use geometry::{GeometryType, GeometryValues};
// pub (not pub(crate)) so __private module can re-export it
pub use id::IdValues;
pub(crate) use id::{Id, RawId, RawIdValue};
pub use iterators::{ColumnRef, FeatureRef, PropName, PropValueRef};
pub(crate) use model::Column;
pub use model::{
    ColumnType, Layer, Layer01, ParsedLayer, ParsedLayer01, PropValue, TileFeature, TileLayer01,
    Unknown,
};
// Re-export strings sub-module so encoder can use `crate::decoder::strings::*`
pub(crate) use property::strings;
pub(crate) use property::{
    DictRange, ParsedProperty, ParsedScalar, ParsedSharedDict, ParsedSharedDictItem, ParsedStrings,
    Property, RawFsstData, RawPlainData, RawPresence, RawProperty, RawScalar, RawSharedDict,
    RawSharedDictEncoding, RawSharedDictItem, RawStrings, RawStringsEncoding,
};
pub use root::{Decoder, Parser};
pub(crate) use stream::model::{
    DictionaryType, IntEncoding, LengthType, LogicalEncoding, LogicalTechnique, LogicalValue,
    Morton, OffsetType, PhysicalEncoding, RawStream, RleMeta, StreamMeta, StreamType,
};
