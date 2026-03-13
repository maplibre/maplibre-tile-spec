use std::cell::RefCell;

use js_sys::Uint8Array;
use mlt_core::EncDec;
use mlt_core::v01::{EncodedGeometry, ParsedGeometry, ParsedProperty};

use crate::geometry::LayerGeometry;
use crate::ids::IdState;
use crate::properties::PropCache;

/// All per-layer state owned by [`crate::tile::MltTile`].
///
/// Geometry types and the `types_array` are decoded eagerly in `decode_tile`.
/// Everything else is decoded lazily on first access and then cached.
pub(crate) struct DecodedLayer {
    pub(crate) name: String,
    pub(crate) extent: u32,

    /// Pre-built `Uint8Array` — one MVT type byte per feature (0/1/2/3).
    /// Built once in `decode_tile`; `layer_types` just clones the handle.
    pub(crate) types_array: Uint8Array,

    /// Encoded or decoded geometry column.
    pub(crate) geometry: RefCell<EncDec<EncodedGeometry, ParsedGeometry>>,

    /// Cached geometry typed arrays — built once on first `layer_geometry` call.
    pub(crate) geometry_cache: RefCell<Option<LayerGeometry>>,

    /// Encode/decode state of the feature-ID column.
    pub(crate) ids: RefCell<IdState>,

    /// Property columns, fully decoded.
    pub(crate) props: RefCell<Vec<ParsedProperty<'static>>>,

    /// Cached bulk property arrays — built once on first `layer_properties` call.
    pub(crate) prop_cache: RefCell<Option<PropCache>>,
}
