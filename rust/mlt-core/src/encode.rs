/// Trait for column types that can exist in either decoded or encoded form.
///
/// This trait is the public half of the encode/decode duality.  It provides
/// read-only access to the encoded representation ([`borrow_encoded`]) and
/// low-level plumbing used by the optimisation traits.  Encoding itself is
/// performed through the [`ManualOptimisation`], [`AutomaticOptimisation`],
/// and [`ProfileOptimisation`] traits, which are the intended entry points
/// for callers.
///
/// [`borrow_encoded`]: Encodable::borrow_encoded
/// [`ManualOptimisation`]: crate::optimizer::ManualOptimisation
/// [`AutomaticOptimisation`]: crate::optimizer::AutomaticOptimisation
/// [`ProfileOptimisation`]: crate::optimizer::ProfileOptimisation
pub trait Encodable: Sized {
    type DecodedType;
    type EncodedType;

    /// Returns `true` if the data is in decoded form.
    fn is_decoded(&self) -> bool;
    /// Wrap an already-encoded value in this type.
    fn new_encoded(encoded: Self::EncodedType) -> Self;
    /// Borrow the encoded data, or `None` if the data is still decoded.
    fn borrow_encoded(&self) -> Option<&Self::EncodedType>;
}
