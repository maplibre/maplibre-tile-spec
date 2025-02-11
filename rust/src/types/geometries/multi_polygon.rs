use serde::Serialize;
use crate::types::geometries::GeometryType;
use crate::types::geometries::polygon::Polygon;

#[derive(PartialEq, Clone, Debug, Serialize)]
pub struct MultiPolygon {
    pub polygons: Vec<Polygon>,
}
impl MultiPolygon {
    pub fn geometry_type() -> GeometryType { GeometryType::Polygon }
}
