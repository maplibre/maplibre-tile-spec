use crate::MltError;

/// Trait for types that can be converted back to raw data
pub trait FromDecoded<'a>: Sized {
    type Input: 'a;
    type EncodingStrategy;
    fn from_decoded(input: &Self::Input, config: Self::EncodingStrategy) -> Result<Self, MltError>;
}

/// Trait for enums that can be in either decoded or raw form
pub trait Encodable<'a>: Sized {
    type DecodedType;
    type RawType: FromDecoded<'a, Input = Self::DecodedType>;

    /// Check if the data is still in decoded form
    fn is_decoded(&self) -> bool;
    /// Create a new instance from raw data
    fn new_raw(decoded: Self::RawType) -> Self;
    /// Temporarily replace self with a default value to take ownership of the decoded data
    fn take_decoded(&mut self) -> Option<Self::DecodedType>;
    /// Borrow the raw data if available
    fn borrow_raw(&self) -> Option<&Self::RawType>;

    fn encode_with(
        &mut self,
        config: <Self::RawType as FromDecoded<'a>>::EncodingStrategy,
    ) -> Result<&Self, MltError>
    where
        Self::RawType: FromDecoded<'a>,
    {
        if self.is_decoded() {
            // Temporarily replace self with a default value to take ownership of the decoded data
            let Some(decoded) = self.take_decoded() else {
                return Err(MltError::NotRaw("raw data"))?;
            };
            let res = Self::RawType::from_decoded(&decoded, config)?;
            *self = Self::new_raw(res);
        }
        Ok(self)
    }
}

/// Macro to implement the Encodable trait for enum types with Decoded and Raw variants
/// This macro is internal to the crate and not exposed to external users
macro_rules! impl_encodable {
    ($enum_type:ty, $decoded_type:ty, $raw_type:ty) => {
        impl<'a> $crate::Encodable<'a> for $enum_type {
            type DecodedType = $decoded_type;
            type RawType = $raw_type;

            fn is_decoded(&self) -> bool {
                matches!(self, Self::Decoded(_))
            }

            fn new_raw(decoded: Self::RawType) -> Self {
                Self::Raw(decoded)
            }

            fn take_decoded(&mut self) -> Option<Self::DecodedType> {
                if let Self::Decoded(decoded) =
                    std::mem::replace(self, Self::Raw(Self::RawType::default()))
                {
                    Some(decoded)
                } else {
                    None
                }
            }

            fn borrow_raw(&self) -> Option<&Self::RawType> {
                if let Self::Raw(raw) = self {
                    Some(raw)
                } else {
                    None
                }
            }
        }
    };
}

// Make the macro available within this module
pub(crate) use impl_encodable;
