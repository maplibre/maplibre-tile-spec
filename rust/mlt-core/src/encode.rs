use crate::MltError;

/// Trait for types that can be created from decoded data.
///
/// This is a crate-internal implementation trait.  External code should use
/// the higher-level [`ManualOptimisation`], [`AutomaticOptimisation`], or
/// [`ProfileOptimisation`] APIs instead.
///
/// [`ManualOptimisation`]: crate::optimizer::ManualOptimisation
/// [`AutomaticOptimisation`]: crate::optimizer::AutomaticOptimisation
/// [`ProfileOptimisation`]: crate::optimizer::ProfileOptimisation
pub(crate) trait FromDecoded<'a>: Sized {
    type Input: 'a;
    type Encoder;
    fn from_decoded(decoded: &Self::Input, encoder: Self::Encoder) -> Result<Self, MltError>;
}

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
    /// Temporarily replace `self` with a sentinel so decoded data can be
    /// taken by value.
    fn take_decoded(&mut self) -> Option<Self::DecodedType>;
    /// Borrow the encoded data, or `None` if the data is still decoded.
    fn borrow_encoded(&self) -> Option<&Self::EncodedType>;
}

/// Macro to implement the [`Encodable`] trait for enum types with `Decoded`
/// and `Encoded` variants.
///
/// This macro is internal to the crate and not exposed to external users.
macro_rules! impl_encodable {
    ($enum_type:ty, $decoded_type:ty, $encoded_type:ty) => {
        impl $crate::Encodable for $enum_type {
            type DecodedType = $decoded_type;
            type EncodedType = $encoded_type;

            fn is_decoded(&self) -> bool {
                matches!(self, Self::Decoded(_))
            }

            fn new_encoded(encoded: Self::EncodedType) -> Self {
                Self::Encoded(encoded)
            }

            fn take_decoded(&mut self) -> Option<Self::DecodedType> {
                if let Self::Decoded(decoded) =
                    std::mem::replace(self, Self::Encoded(Self::EncodedType::default()))
                {
                    Some(decoded)
                } else {
                    None
                }
            }

            fn borrow_encoded(&self) -> Option<&Self::EncodedType> {
                if let Self::Encoded(enc) = self {
                    Some(enc)
                } else {
                    None
                }
            }
        }
    };
}

// Make the macro available within the crate.
pub(crate) use impl_encodable;
