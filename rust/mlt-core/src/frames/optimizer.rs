use crate::frames::{EncodedLayer, LayerEncoder, LayerProfile};
use crate::{MltError, StagedLayer};

impl StagedLayer {
    /// Encode using a specific [`LayerEncoder`], consuming `self` and producing [`EncodedLayer`].
    pub fn encode(self, encoder: LayerEncoder) -> Result<EncodedLayer, MltError> {
        match (self, encoder) {
            (Self::Tag01(t), LayerEncoder::Tag01(e)) => Ok(EncodedLayer::Tag01(t.encode(e)?)),
            (Self::Unknown(u), LayerEncoder::Unknown) => Ok(EncodedLayer::Unknown(u)),
            _ => Err(MltError::BadEncoderDataCombination),
        }
    }

    /// Profile-driven encode, consuming `self` and producing `(EncodedLayer, LayerEncoder)`.
    pub fn encode_with_profile(
        self,
        profile: &LayerProfile,
    ) -> Result<(EncodedLayer, LayerEncoder), MltError> {
        match (self, profile) {
            (Self::Tag01(t), LayerProfile::Tag01(p)) => {
                let (encoded, enc) = t.encode_with_profile(p)?;
                Ok((EncodedLayer::Tag01(encoded), LayerEncoder::Tag01(enc)))
            }
            (Self::Unknown(u), LayerProfile::Unknown) => {
                Ok((EncodedLayer::Unknown(u), LayerEncoder::Unknown))
            }
            _ => Err(MltError::BadProfileDataCombination),
        }
    }

    /// Automatically select the best encoders, consuming `self` and producing
    /// `(EncodedLayer, LayerEncoder)`.
    pub fn encode_auto(self) -> Result<(EncodedLayer, LayerEncoder), MltError> {
        match self {
            Self::Tag01(t) => {
                let (encoded, enc) = t.encode_auto()?;
                Ok((EncodedLayer::Tag01(encoded), LayerEncoder::Tag01(enc)))
            }
            Self::Unknown(u) => Ok((EncodedLayer::Unknown(u), LayerEncoder::Unknown)),
        }
    }
}
