use crate::MltError;
use crate::optimizer::{AutomaticOptimisation, ManualOptimisation, ProfileOptimisation};
use crate::v01::{
    GeometryEncoder, GeometryProfile, IdEncoder, IdProfile, OwnedLayer01, PropertyEncoder,
    PropertyProfile,
};

impl ManualOptimisation for OwnedLayer01 {
    type UsedEncoder = Tag01Encoder;

    fn manual_optimisation(&mut self, encoder: Self::UsedEncoder) -> Result<(), MltError> {
        if let (Some(id_enc), Some(id)) = (encoder.id, &mut self.id) {
            id.manual_optimisation(id_enc)?;
        }
        self.properties.manual_optimisation(encoder.properties)?;
        self.geometry.manual_optimisation(encoder.geometry)?;
        Ok(())
    }
}

impl ProfileOptimisation for OwnedLayer01 {
    type UsedEncoder = Tag01Encoder;
    type Profile = Tag01Profile;

    fn profile_driven_optimisation(
        &mut self,
        profile: &Self::Profile,
    ) -> Result<Self::UsedEncoder, MltError> {
        let id = match &mut self.id {
            Some(id) => id.profile_driven_optimisation(&profile.id)?,
            None => None,
        };
        let properties = self
            .properties
            .profile_driven_optimisation(&profile.properties)?;
        let geometry = self
            .geometry
            .profile_driven_optimisation(&profile.geometry)?;

        Ok(Tag01Encoder {
            id,
            properties,
            geometry,
        })
    }
}

impl AutomaticOptimisation for OwnedLayer01 {
    type UsedEncoder = Tag01Encoder;

    fn automatic_encoding_optimisation(&mut self) -> Result<Self::UsedEncoder, MltError> {
        let id = match &mut self.id {
            Some(id) => id.automatic_encoding_optimisation()?,
            None => None,
        };
        let properties = self.properties.automatic_encoding_optimisation()?;
        let geometry = self.geometry.automatic_encoding_optimisation()?;
        Ok(Tag01Encoder {
            id,
            properties,
            geometry,
        })
    }
}

/// Fully-specified encoder configuration for a v01 layer, produced by any of
/// the three optimization paths (manual, automatic, or profile-driven).
#[derive(Debug, Clone)]
pub struct Tag01Encoder {
    pub id: Option<IdEncoder>,
    pub properties: Vec<PropertyEncoder>,
    pub geometry: GeometryEncoder,
}

/// Profile for a v01 layer, built by running automatic optimization over a
/// representative sample of tiles and capturing the chosen encoders.
#[derive(Debug, Clone)]
pub struct Tag01Profile {
    pub id: IdProfile,
    pub properties: PropertyProfile,
    pub geometry: GeometryProfile,
}
