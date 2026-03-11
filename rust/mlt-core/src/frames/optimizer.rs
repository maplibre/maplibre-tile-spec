use crate::frames::{LayerEncoder, LayerProfile};
use crate::optimizer::{AutomaticOptimisation, ManualOptimisation, ProfileOptimisation};
use crate::{MltError, OwnedLayer};

impl ManualOptimisation for OwnedLayer {
    type UsedEncoder = LayerEncoder;

    fn manual_optimisation(&mut self, encoder: Self::UsedEncoder) -> Result<(), MltError> {
        use LayerEncoder as E;
        use OwnedLayer as L;
        match (self, encoder) {
            (L::Tag01(t), E::Tag01(e)) => Ok(t.manual_optimisation(e)?),
            (L::Unknown(_), E::Unknown) => Ok(()),
            (L::Tag01(_) | L::Unknown(_), _) => Err(MltError::BadEncoderDataCombination),
        }
    }
}

impl ProfileOptimisation for OwnedLayer {
    type UsedEncoder = LayerEncoder;
    type Profile = LayerProfile;

    fn profile_driven_optimisation(
        &mut self,
        profile: &Self::Profile,
    ) -> Result<Self::UsedEncoder, MltError> {
        use LayerEncoder as E;
        use LayerProfile as P;
        use OwnedLayer as L;
        match (self, profile) {
            (L::Tag01(t), P::Tag01(p)) => Ok(E::Tag01(t.profile_driven_optimisation(p)?)),
            (L::Unknown(_), P::Unknown) => Ok(E::Unknown),
            (L::Tag01(_) | L::Unknown(_), _) => Err(MltError::BadProfileDataCombination),
        }
    }
}

impl AutomaticOptimisation for OwnedLayer {
    type UsedEncoder = LayerEncoder;
    fn automatic_encoding_optimisation(&mut self) -> Result<Self::UsedEncoder, MltError> {
        match self {
            OwnedLayer::Tag01(t) => Ok(LayerEncoder::Tag01(t.automatic_encoding_optimisation()?)),
            OwnedLayer::Unknown(_) => Ok(LayerEncoder::Unknown),
        }
    }
}
