use serde::{Serialize, Serializer};
use serde::ser::SerializeStruct;
use crate::types::geometries::CoordinateType;

#[derive(PartialEq, Clone, Debug)]
pub struct Coordinate {
    pub x: CoordinateType,
    pub y: CoordinateType,
}

impl Serialize for Coordinate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Coordinate", 2)?;
        state.serialize_field("x", &self.x)?;
        state.serialize_field("y", &self.y)?;
        state.end()
    }
}
