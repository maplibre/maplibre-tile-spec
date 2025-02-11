use serde::{Serialize, Serializer};
use serde::ser::SerializeSeq;
use crate::types::geometries::coordinate::Coordinate;

#[derive(PartialEq, Clone, Debug)]
pub struct Point {
    pub coordinate: Coordinate,
}

impl Serialize for Point {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&self.coordinate.x)?;
        seq.serialize_element(&self.coordinate.y)?;
        seq.end()
    }
}
