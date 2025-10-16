use crate::MltError;

pub trait Decodable<'a>: Sized {
    type RawType;
    type DecodedType: Parsable<'a, Input = Self::RawType>;

    /// Check if the data is still in raw form
    fn is_raw(&self) -> bool;
    /// Create a new instance from decoded data
    fn new_decoded(raw: Self::DecodedType) -> Self;
    /// Temporarily replace self with a default value to take ownership of the raw data
    fn take_raw(&mut self) -> Option<Self::RawType>;
    /// Borrow the decoded data if available
    fn borrow_decoded(&self) -> Option<&Self::DecodedType>;

    fn decode(&mut self) -> Result<&Self::DecodedType, MltError> {
        if self.is_raw() {
            // Temporarily replace self with a default value to take ownership of the raw data
            let raw = self.take_raw().expect("Expected raw data");
            *self = Self::new_decoded(Self::DecodedType::parse(raw)?);
        }
        Ok(self.borrow_decoded().expect("Expected decoded data"))
    }
}

pub trait Parsable<'a>: Sized {
    type Input: 'a;
    fn parse(input: Self::Input) -> Result<Self, MltError>;
}

/// Macro to implement the Decodable trait for enum types with Raw and Decoded variants
/// This macro is internal to the crate and not exposed to external users
macro_rules! impl_decodable {
    ($enum_type:ty, $raw_type:ty, $decoded_type:ty) => {
        impl<'a> $crate::v0x01::Decodable<'a> for $enum_type {
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
