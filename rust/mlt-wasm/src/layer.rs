use std::cell::RefCell;

use js_sys::Uint8Array;
use mlt_core::v01::{ParsedGeometry, ParsedProperty};

use crate::geometry::LayerGeometry;
use crate::ids::IdState;
use crate::properties::PropCache;

/// All per-layer state owned by [`crate::tile::MltTile`].
///
/// Geometry types and the `types_array` are decoded eagerly in `decode_tile`.
/// Full geometry is decoded eagerly and stored as `Some(ParsedGeometry)`.
/// Properties are decoded eagerly as `Vec<ParsedProperty<'static>>` — the input bytes
/// have been dropped so all string data is promoted to owned via `into_static()`.
pub(crate) struct DecodedLayer {
    pub(crate) name: String,
    pub(crate) extent: u32,

    /// Pre-built `Uint8Array` — one MVT type byte per feature (0/1/2/3).
    /// Built once in `decode_tile`; `layer_types` just clones the handle.
    pub(crate) types_array: Uint8Array,

    /// Fully decoded geometry column (`None` only if not yet decoded — always `Some` after init).
    pub(crate) geometry: RefCell<Option<ParsedGeometry>>,

    /// Cached geometry typed arrays — built once on first `layer_geometry` call.
    pub(crate) geometry_cache: RefCell<Option<LayerGeometry>>,

    /// Encode/decode state of the feature-ID column.
    pub(crate) ids: RefCell<IdState>,

    /// Decoded property columns.  The `'static` bound is required because the
    /// original input bytes are not kept alive; all borrowed string slices are
    /// promoted to owned `String` via `ParsedProperty::into_static()`.
    pub(crate) props: RefCell<Vec<ParsedProperty<'static>>>,

    /// Cached bulk property arrays — built once on first `layer_properties` call.
    pub(crate) prop_cache: RefCell<Option<PropCache>>,
}
