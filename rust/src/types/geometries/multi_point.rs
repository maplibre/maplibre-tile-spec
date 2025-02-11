use serde::Serialize;
use crate::types::geometries::point::Point;

#[derive(PartialEq, Clone, Debug, Serialize)]
pub struct MultiPoint {
    pub points: Vec<Point>
}
