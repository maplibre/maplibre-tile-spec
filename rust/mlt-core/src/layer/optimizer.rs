use crate::optimizer::AutomaticOptimisation;
use crate::v01::Tag01Encoder;
use crate::{MltError, OwnedLayer};

impl AutomaticOptimisation for OwnedLayer {
    type UsedEncoder = LayerEncoder;

    fn automatic_encoding_optimisation(&mut self) -> Result<Self::UsedEncoder, MltError> {
        match self {
            OwnedLayer::Tag01(t) => Ok(LayerEncoder::Tag01(t.automatic_encoding_optimisation()?)),
            OwnedLayer::Unknown(_) => Ok(LayerEncoder::Unknown),
        }
    }
}

#[derive(Debug, Clone)]
pub enum LayerEncoder {
    Tag01(Tag01Encoder),
    Unknown,
}
