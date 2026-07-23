//! Annotated binary dump of an MLT tile, for debugging the wire format.
//!
//! [`annotate_tile`] walks a tile buffer into a [`DumpTree`] of [`Region`]s.
//! [`render`] formats that tree as an annotated hexdump.

mod model;
mod render;
mod walker01;

pub use model::{BitField, BlobInfo, DecodeHint, DumpTree, Region, RegionKind};
pub use render::{DataMode, RenderOpts, render};
pub use walker01::annotate_tile;
