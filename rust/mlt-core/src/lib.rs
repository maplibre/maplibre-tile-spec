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

pub(crate) mod codecs;
pub(crate) mod convert;
pub(crate) mod decoder;
pub mod encoder;
pub(crate) mod errors;
pub(crate) mod utils;

pub use convert::{geojson, mvt};
pub use decoder::*;
pub use errors::{MltError, MltRefResult, MltResult};
pub use utils::analyze::{Analyze, StatType};
pub use utils::lazy_state::{Decode, DecodeState, Lazy, LazyParsed, Parsed};

#[cfg(any(test, feature = "__private"))]
pub use crate::utils::test_helpers;

/// `GeoJSON` geometry with `i32` tile coordinates
pub type Geom32 = geo_types::Geometry<i32>;

/// A single `i32` coordinate (x, y)
pub type Coord32 = geo_types::Coord<i32>;

/// `GeoJSON` geometry with `i16` tile coordinates
pub type Geom16 = geo_types::Geometry<i16>;

/// A single `i16` coordinate (x, y)
pub type Coord16 = geo_types::Coord<i16>;

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
