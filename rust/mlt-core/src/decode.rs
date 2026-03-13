use crate::MltError;

/// Decoding counterpart to [`TryFrom`], used as a trait bound on `Decodable::DecodedType`.
///
/// Mirrors the structure of [`TryFrom`] but is defined in this crate, which allows
/// implementing it for foreign types like `Option<DecodedId>` without hitting the
/// orphan rule that would block `impl TryFrom<Option<EncodedId<'_>>> for Option<DecodedId>`.
pub trait Decode<Input>: Sized {
    fn decode(input: Input) -> Result<Self, MltError>;
}

/// Decoding counterpart to [`TryInto`]: consume `self` and decode into `Output`.
///
/// A blanket impl is provided: any type that implements [`Decode<I>`] for `Self` as the input
/// type gets `DecodeInto<Self>` implemented for `I`.
pub trait DecodeInto<Output>: Sized {
    fn decode_into(self) -> Result<Output, MltError>;
}

impl<I: Sized, O: Decode<I>> DecodeInto<O> for I {
    fn decode_into(self) -> Result<O, MltError> {
        O::decode(self)
    }
}

/// Trait for types that can be in either encoded or decoded form.
pub(crate) trait Decodable<'a>: Sized {
    type EncodedType;
    type DecodedType: Decode<Self::EncodedType>;

    /// Check if the data is still in encoded form
    fn is_encoded(&self) -> bool;
    /// Create a new instance from decoded data
    fn new_decoded(decoded: Self::DecodedType) -> Self;
    /// Temporarily replace self with a default value to take ownership of the raw data
    fn take_encoded(&mut self) -> Option<Self::EncodedType>;
    /// Borrow the decoded data mutably if available
    fn borrow_decoded_mut(&mut self) -> Option<&mut Self::DecodedType>;

    fn materialize(&mut self) -> Result<&mut Self::DecodedType, MltError> {
        if self.is_encoded() {
            // Temporarily replace self with a default value to take ownership of the raw data
            let Some(enc) = self.take_encoded() else {
                return Err(MltError::NotDecoded("decoded data"))?;
            };
            let res: Self::DecodedType = enc.decode_into()?;
            *self = Self::new_decoded(res);
        }
        self.borrow_decoded_mut()
            .ok_or(MltError::NotDecoded("decoded data"))
    }
}
