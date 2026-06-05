#![doc = include_str!("../README.md")]

#[cfg(not(any(feature = "reader", feature = "writer")))]
compile_error!("fast-mvt requires at least one of the `reader` or `writer` features to be enabled");

mod error;
pub use error::{MvtError, MvtResult};

mod geom;
#[cfg(feature = "reader")]
mod geom_reader;
#[cfg(feature = "writer")]
mod geom_writer;

#[rustfmt::skip]
#[allow(
    clippy::derivable_impls,
    clippy::needless_else,
    clippy::pedantic,
    clippy::upper_case_acronyms,
    clippy::use_self,
    dead_code,
    unused_qualifications,
)]
mod generated;

pub mod proto;

#[cfg(feature = "reader")]
mod reader;
#[cfg(feature = "reader")]
pub use reader::{MvtFeatureRef, MvtLayerRef, MvtPropertyIter, MvtReaderRef, MvtValueRef};

mod types;
pub use types::{
    DEFAULT_EXTENT, MvtCoord, MvtExtent, MvtFeature, MvtGeometry, MvtLayer, MvtLineString,
    MvtMultiLineString, MvtMultiPoint, MvtMultiPolygon, MvtPoint, MvtPolygon, MvtTile, MvtValue,
};

#[cfg(feature = "writer")]
mod writer;

#[cfg(feature = "writer")]
pub use writer::{encode, encode_to_vec};
