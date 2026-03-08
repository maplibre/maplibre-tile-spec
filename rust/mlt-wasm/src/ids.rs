use js_sys::Float64Array;
use mlt_core::v01::OwnedEncodedId;

/// Tracks the decode state of a layer's feature-ID column.
pub(crate) enum IdState {
    /// Layer has no ID column.
    Absent,
    /// Raw encoded bytes, not yet decoded.
    Encoded(OwnedEncodedId),
    /// Decoded and converted to a JS-owned typed array ready to return.
    /// One `f64` per feature; absent IDs are `NaN`.
    Ready(Float64Array),
}
