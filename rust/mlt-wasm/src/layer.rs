use mlt_core::v01::ParsedGeometry;
use mlt_core::v01::tile::TileLayer01;

/// All per-layer state owned by [`crate::tile::MltTile`].
///
/// Fully decoded at `decode_tile` time — no lazy loading.
pub(crate) struct DecodedLayer {
    pub(crate) tile: TileLayer01,

    /// Pre-built `Uint8Array` — one MVT type byte per feature (0/1/2/3).
    pub(crate) types_array: js_sys::Uint8Array,

    /// Decoded geometry in columnar offset form, kept for the WASM geometry
    /// typed-array accessors which need the raw offset arrays.
    pub(crate) geometry: ParsedGeometry,
}
