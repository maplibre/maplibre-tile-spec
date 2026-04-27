#![doc = include_str!("../README.md")]
extern crate core;

/// Validates stream metadata in constructors (crate-internal).
macro_rules! validate_stream {
    ($stream:expr, $expected:pat $(,)?) => {
        if !matches!($stream.meta.stream_type, $expected) {
            return Err($crate::MltError::UnexpectedStreamType2(
                $stream.meta.stream_type,
                stringify!($expected),
                stringify!($stream),
            ));
        }
    };
}

// re-export geo types
pub use geo_types;

pub(crate) mod codecs;
pub(crate) mod convert;
pub(crate) mod decoder;
pub mod encoder;
pub(crate) mod errors;
pub(crate) mod utils;

pub use convert::{geojson, mvt};
pub use decoder::{
    ColumnRef, Decoder, FeatureRef, GeometryType, GeometryValues, Layer, Layer01,
    Layer01FeatureIter, LendingIterator, ParsedLayer, ParsedLayer01, Parser, PropName, PropValue,
    PropValueRef, TileFeature, TileLayer, Unknown,
};
// Crate-internal re-exports: allow internal modules to use `crate::Lazy` etc.
// without exposing these implementation details to external users.
pub(crate) use decoder::{
    ColumnType, DictRange, DictionaryType, LengthType, OffsetType, RawPresence, RawSharedDict,
    RawSharedDictItem, StreamType,
};
pub(crate) use errors::MltRefResult;
pub use errors::{MltError, MltResult};
pub(crate) use utils::analyze::{Analyze, StatType};
pub(crate) use utils::lazy_state::{Decode, DecodeState, Lazy, LazyParsed, Parsed};

/// Wire-level encoding metadata — for tile analysis and tooling.
///
/// These types describe the physical and logical encoding of streams inside an
/// MLT tile. Normal tile consumers (parse → iterate features) do not need this
/// module; it is intended for tools that inspect or report encoding statistics.
pub mod wire {
    pub use crate::decoder::ColumnType;
    pub use crate::decoder::stream::model::{
        DictionaryType, IntEncoding, LengthType, LogicalEncoding, LogicalTechnique, Morton,
        OffsetType, PhysicalEncoding, RleMeta, StreamMeta, StreamType,
    };
    pub use crate::utils::analyze::{Analyze, StatType};
}

#[cfg(any(test, feature = "__private"))]
pub use crate::utils::test_helpers;

/// Private re-exports for benchmarks and integration tests. Not part of the public API.
#[cfg(any(test, feature = "__private"))]
#[doc(hidden)]
pub mod __private {
    pub use crate::codecs::*;
    pub use crate::convert::*;
    pub use crate::decoder::*;
    pub use crate::errors::*;
    pub use crate::test_helpers::*;
    pub use crate::utils::analyze::*;
    pub use crate::utils::lazy_state::*;
    pub use crate::utils::*;
}
