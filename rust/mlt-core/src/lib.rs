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
#[doc(hidden)]
pub(crate) use validate_stream;

pub(crate) mod analyze;
pub(crate) mod codecs;
pub(crate) mod convert;
pub(crate) mod decoder;
pub mod encoder;
pub(crate) mod errors;
pub(crate) mod lazy_state;
pub(crate) mod utils;

pub use analyze::{Analyze, StatType};
pub use convert::{geojson, mvt};
pub use decoder::{
    Decoder, DictionaryType, GeometryType, GeometryValues, IdValues, Layer, Layer01, LengthType,
    LogicalEncoder, LogicalEncoding, OffsetType, ParsedLayer, ParsedLayer01, Parser,
    PhysicalEncoding, PropValue, PropValueRef, RawFsstData, RawGeometry, RawPlainData, RawPresence,
    RawSharedDict, RawSharedDictEncoding, RawSharedDictItem, RawStrings, RawStringsEncoding,
    StreamMeta, StreamType, TileFeature, TileLayer01,
};
pub use encoder::{EncodedLayer, StagedLayer};
pub use errors::{MltError, MltRefResult, MltResult};
pub use lazy_state::{Decode, DecodeState, Lazy, LazyParsed, Parsed};

#[cfg(any(test, feature = "__private"))]
pub mod test_helpers;

/// Private re-exports for benchmarks and integration tests. Not part of the public API.
#[cfg(any(test, feature = "__private"))]
#[doc(hidden)]
pub mod __private {
    pub use crate::analyze::*;
    pub use crate::codecs::*;
    pub use crate::convert::*;
    pub use crate::decoder::*;
    pub use crate::errors::*;
    pub use crate::lazy_state::*;
    pub use crate::test_helpers::*;
    pub use crate::utils::*;
}
