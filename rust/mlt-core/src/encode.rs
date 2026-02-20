use crate::MltError;

/// Trait for types that can be converted back to encoded data
pub trait FromDecoded<'a>: Sized {
    type Input: 'a;
    type EncodingStrategy;
    fn from_decoded(input: &Self::Input, config: Self::EncodingStrategy) -> Result<Self, MltError>;
}

/// Trait for enums that can be in either decoded or encoded form
pub trait Encodable<'a>: Sized {
    type DecodedType;
    type EncodedType: FromDecoded<'a, Input = Self::DecodedType>;

    /// Check if the data is still in decoded form
    fn is_decoded(&self) -> bool;
    /// Create a new instance from encoded data
    fn new_encoded(decoded: Self::EncodedType) -> Self;
    /// Temporarily replace self with a default value to take ownership of the decoded data
    fn take_decoded(&mut self) -> Option<Self::DecodedType>;
    /// Borrow the encoded data if available
    fn borrow_encoded(&self) -> Option<&Self::EncodedType>;

    fn encode_with(
        &mut self,
        config: <Self::EncodedType as FromDecoded<'a>>::EncodingStrategy,
    ) -> Result<&Self, MltError>
    where
        Self::EncodedType: FromDecoded<'a>,
    {
        if self.is_decoded() {
            // Temporarily replace self with a default value to take ownership of the decoded data
            let Some(decoded) = self.take_decoded() else {
                return Err(MltError::NotEncoded("decoded data"))?;
            };
            let res = Self::EncodedType::from_decoded(&decoded, config)?;
            *self = Self::new_encoded(res);
        }
        Ok(self)
    }
}

/// Macro to implement the Encodable trait for enum types with Decoded and Encoded variants
/// This macro is internal to the crate and not exposed to external users
macro_rules! impl_encodable {
    ($enum_type:ty, $decoded_type:ty, $encoded_type:ty) => {
        impl<'a> $crate::Encodable<'a> for $enum_type {
            type DecodedType = $decoded_type;
            type EncodedType = $encoded_type;

            fn is_decoded(&self) -> bool {
                matches!(self, Self::Decoded(_))
            }

            fn new_encoded(decoded: Self::EncodedType) -> Self {
                Self::Encoded(decoded)
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

// Make the macro available within this module
pub(crate) use impl_encodable;
