//! Annotated binary dump of an MLT tile.
//!
//! [`annotate_tile`] walks a tile buffer and produces a [`DumpTree`] of
//! [`Region`]s describing every metadata byte (and the bit-fields of packed
//! bytes), plus opaque data-payload blobs. [`render`] formats the tree as an
//! annotated hexdump. Intended for debugging the wire format; normal tile
//! consumers do not need this module.

mod model;
mod render;
mod walker01;

pub use model::{BitField, BlobInfo, DecodeHint, DumpTree, Region, RegionKind};
pub use render::{DataMode, RenderOpts, render};
pub use walker01::annotate_tile;
