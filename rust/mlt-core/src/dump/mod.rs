//! Annotated binary dump of an MLT tile, for debugging the wire format.
//!
//! [`annotate_tile`] walks a tile buffer into a [`DumpTree`] of [`Region`]s.
//! [`render()`] formats that tree as an annotated hexdump.

mod decode;
mod model;
mod render;
mod walker;
mod walker01;
#[cfg(feature = "unstable-v2")]
mod walker02;

pub use crate::dump::decode::{DecodedBlob, decode_blob, decode_region};
pub use crate::dump::model::{
    BitField, BlobInfo, DecodeHint, DumpTree, Region, RegionKind, UNANNOTATED, filter_layer,
};
pub use crate::dump::render::{DataMode, RenderOpts, render};
pub use crate::dump::walker::annotate_tile;
