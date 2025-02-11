use serde::{Serialize, Serializer};
use serde::ser::SerializeSeq;
use crate::types::geometries::GeometryType;
use crate::types::geometries::linestring::LineString;

#[derive(PartialEq, Clone, Debug)]
pub struct MultiLineString {
    pub strings: Vec<LineString>
}
impl MultiLineString {
    pub fn geometry_type() -> GeometryType { GeometryType::Linestring }
}

impl Serialize for MultiLineString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&self.strings)?;
        seq.end()
    }
}
