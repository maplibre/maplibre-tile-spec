use crate::MltError;

/// Trait for types that can be constructed from encoded data
pub(crate) trait FromEncoded<'a>: Sized {
    type Input: 'a;
    fn from_encoded(input: Self::Input) -> Result<Self, MltError>;
}

/// Trait for enums that can be in either encoded or decoded form
pub(crate) trait Decodable<'a>: Sized {
    type EncodedType;
    type DecodedType: FromEncoded<'a, Input = Self::EncodedType>;

    /// Check if the data is still in encoded form
    fn is_encoded(&self) -> bool;
    /// Create a new instance from decoded data
    fn new_decoded(decoded: Self::DecodedType) -> Self;
    /// Temporarily replace self with a default value to take ownership of the raw data
    fn take_encoded(&mut self) -> Option<Self::EncodedType>;
    /// Borrow the decoded data if available
    fn borrow_decoded(&self) -> Option<&Self::DecodedType>;
    /// Borrow the decoded data mutably if available
    fn borrow_decoded_mut(&mut self) -> Option<&mut Self::DecodedType>;

    fn materialize(&mut self) -> Result<&mut Self::DecodedType, MltError> {
        if self.is_encoded() {
            // Temporarily replace self with a default value to take ownership of the raw data
            let Some(enc) = self.take_encoded() else {
                return Err(MltError::NotDecoded("decoded data"))?;
            };
            let res = Self::DecodedType::from_encoded(enc)?;
            *self = Self::new_decoded(res);
        }
        self.borrow_decoded_mut()
            .ok_or(MltError::NotDecoded("decoded data"))
    }
}

/// Macro to implement the Decodable trait for enum types with Encoded and Decoded variants
/// This macro is internal to the crate and not exposed to external users
macro_rules! impl_decodable {
    ($enum_type:ty, $encoded_type:ty, $decoded_type:ty) => {
        impl<'a> $crate::Decodable<'a> for $enum_type {
            type EncodedType = $encoded_type;
            type DecodedType = $decoded_type;

            fn is_encoded(&self) -> bool {
                matches!(self, Self::Encoded(_))
            }

            fn new_decoded(decoded: Self::DecodedType) -> Self {
                Self::Decoded(decoded)
            }

            fn take_encoded(&mut self) -> Option<Self::EncodedType> {
                if let Self::Encoded(enc) =
                    std::mem::replace(self, Self::Decoded(Self::DecodedType::default()))
                {
                    Some(enc)
                } else {
                    None
                }
            }

            fn borrow_decoded(&self) -> Option<&Self::DecodedType> {
                if let Self::Decoded(decoded) = self {
                    Some(decoded)
                } else {
                    None
                }
            }

            fn borrow_decoded_mut(&mut self) -> Option<&mut Self::DecodedType> {
                if let Self::Decoded(decoded) = self {
                    Some(decoded)
                } else {
                    None
                }
            }
        }
    };
}

// Make the macro available within this module
pub(crate) use impl_decodable;
