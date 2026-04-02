mod analyze;
mod column;
mod encoded;
#[cfg(all(not(test), feature = "arbitrary"))]
mod fuzzing;
mod geometry;
mod id;
mod iterators;
mod model;
pub(crate) mod property;
mod root;
pub(crate) mod sort;
pub(crate) mod stream;
pub mod tile;

// Re-export encoder types for backward compatibility
pub use geometry::*;
pub use id::*;
pub use iterators::{
    ColumnRef, FeatPropertyIter, FeatValuesIter, FeatureRef, Layer01FeatureIter,
    Layer01PropNamesIter, PropName, PropValueRef,
};
pub use model::{
    Column, ColumnType, EncodedLayer01, Layer01, OwnedColumn, ParsedLayer01, PropValue,
    StagedLayer01, TileFeature, TileLayer01,
};
pub use property::*;
pub use sort::SortStrategy;
pub use stream::*;

pub use crate::encoder::property::{
    EncodedName, EncodedPresence, EncodedProperty, EncodedScalar, EncodedSharedDict,
    EncodedSharedDictEncoding, EncodedSharedDictItem, EncodedStrings, EncodedStringsEncoding,
    PresenceKind, PropertyEncoder, PropertyKind, RawSharedDict, RawSharedDictEncoding, RawStrings,
    RawStringsEncoding, ScalarEncoder, ScalarValueEncoder, SharedDictEncoder,
    SharedDictItemEncoder, StagedProperty, StagedScalar, StagedSharedDict, StagedSharedDictItem,
    StagedStrings, StrEncoder, encode_shared_dict_prop,
};
