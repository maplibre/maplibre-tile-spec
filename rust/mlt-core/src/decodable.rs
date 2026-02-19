use crate::MltError;

/// Trait for enums that can be in either raw or decoded form
pub trait Decodable<'a>: Sized {
    type RawType;
    type DecodedType: FromRaw<'a, Input = Self::RawType>;

    /// Check if the data is still in raw form
    fn is_raw(&self) -> bool;
    /// Create a new instance from decoded data
    fn new_decoded(raw: Self::DecodedType) -> Self;
    /// Temporarily replace self with a default value to take ownership of the raw data
    fn take_raw(&mut self) -> Option<Self::RawType>;
    /// Borrow the decoded data if available
    fn borrow_decoded(&self) -> Option<&Self::DecodedType>;

    fn materialize(&mut self) -> Result<&Self, MltError> {
        if self.is_raw() {
            // Temporarily replace self with a default value to take ownership of the raw data
            let Some(raw) = self.take_raw() else {
                return Err(MltError::NotDecoded("expected raw data"))?;
            };
            let res = Self::DecodedType::from_raw(raw)?;
            *self = Self::new_decoded(res);
        }
        Ok(self)
    }
}

/// Trait for types that can be constructed from raw data
pub trait FromRaw<'a>: Sized {
    type Input: 'a;
    fn from_raw(input: Self::Input) -> Result<Self, MltError>;
}

/// Macro to implement the Decodable trait for enum types with Raw and Decoded variants
/// This macro is internal to the crate and not exposed to external users
macro_rules! impl_decodable {
    ($enum_type:ty, $raw_type:ty, $decoded_type:ty) => {
        impl<'a> $crate::Decodable<'a> for $enum_type {
            type RawType = $raw_type;
            type DecodedType = $decoded_type;

            fn is_raw(&self) -> bool {
                matches!(self, Self::Raw(_))
            }

            fn new_decoded(raw: Self::DecodedType) -> Self {
                Self::Decoded(raw)
            }

            fn take_raw(&mut self) -> Option<Self::RawType> {
                if let Self::Raw(raw) =
                    std::mem::replace(self, Self::Decoded(Self::DecodedType::default()))
                {
                    Some(raw)
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
        }
    };
}

// Make the macro available within this module
pub(crate) use impl_decodable;
