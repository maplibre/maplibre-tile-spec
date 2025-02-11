use serde::Serialize;
use crate::types::geometries::coordinate::Coordinate;
use crate::types::geometries::GeometryType;

#[derive(PartialEq, Clone, Debug, Serialize)]
pub struct LineString {
    pub points: Vec<Coordinate>
}
impl LineString {
    pub fn geometry_type() -> GeometryType { GeometryType::Linestring }
}
