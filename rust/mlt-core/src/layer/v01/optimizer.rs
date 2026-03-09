use crate::MltError;
use crate::optimizer::AutomaticOptimisation;
use crate::v01::{GeometryEncoder, IdEncoder, OwnedLayer01, PropertyEncoder};

impl AutomaticOptimisation for OwnedLayer01 {
    type UsedEncoder = Tag01Encoder;

    fn automatic_encoding_optimisation(&mut self) -> Result<Self::UsedEncoder, MltError> {
        let id = self.id.automatic_encoding_optimisation()?;
        let properties = self.properties.automatic_encoding_optimisation()?;
        let geometry = self.geometry.automatic_encoding_optimisation()?;
        Ok(Tag01Encoder {
            id,
            properties,
            geometry,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Tag01Encoder {
    pub id: Option<IdEncoder>,
    pub properties: Vec<PropertyEncoder>,
    pub geometry: GeometryEncoder,
}
