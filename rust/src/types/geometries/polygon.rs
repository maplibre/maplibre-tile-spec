use serde::Serialize;
use crate::types::geometries::GeometryType;
use crate::types::geometries::linearring::LinearRing;

#[derive(PartialEq, Clone, Debug, Serialize)]
pub struct Polygon {
    pub(crate) shell: LinearRing,
    pub(crate) holes: Vec<LinearRing>,
}
impl Polygon {
    pub fn geometry_type() -> GeometryType { GeometryType::Polygon }
}
