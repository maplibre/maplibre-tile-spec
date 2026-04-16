use mlt_core::{GeometryValues, TileLayer01};

/// All per-layer state owned by [`crate::tile::MltTile`].
///
/// Fully decoded at `decode_tile` time — no lazy loading.
pub(crate) struct DecodedLayer {
    pub(crate) tile: TileLayer01,

    /// MVT geometry types (0/1/2/3) — collapses single and multi variants.
    pub(crate) types_array: js_sys::Uint8Array,

    /// Original MLT geometry types — preserves the single vs multi distinction.
    pub(crate) mlt_types_array: js_sys::Uint8Array,

    pub(crate) geometry: GeometryValues,
}
