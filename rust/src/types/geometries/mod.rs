pub mod coordinate;
pub mod linestring;
pub mod point;
pub mod multi_point;
pub mod linearring;
pub mod polygon;
pub mod multi_linestring;
pub mod multi_polygon;
pub mod factory;

use core::fmt::Debug;
use serde::{Deserialize, Serialize, Serializer};
use wasm_bindgen::prelude::wasm_bindgen;
use crate::types::geometries::linearring::LinearRing;
use crate::types::geometries::linestring::LineString;
use crate::types::geometries::multi_linestring::MultiLineString;
use crate::types::geometries::multi_point::MultiPoint;
use crate::types::geometries::multi_polygon::MultiPolygon;
use crate::types::geometries::point::Point;
use crate::types::geometries::polygon::Polygon;

pub type CoordinateType = f64;


#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
#[repr(i32)]
pub enum GeometryType {
    Point = 0,
    Linestring = 1,
    Polygon = 2,
    MultiPoint = 3,
    MultiLineString = 4,
    MultiPolygon = 5,
}





#[derive(PartialEq, Clone, Debug)]
pub enum Geometry {
    Point(Point),
    MultiPoint(MultiPoint),
    LineString(LineString),
    MultiLineString(MultiLineString),
    LinearRing(LinearRing),
    Polygon(Polygon),
    MultiPolygon(MultiPolygon),
}

impl Serialize for Geometry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Geometry::Point(p) => p.serialize(serializer),
            Geometry::MultiPoint(mp) => mp.serialize(serializer),
            Geometry::LineString(ls) => ls.serialize(serializer),
            Geometry::MultiLineString(mls) => mls.serialize(serializer),
            Geometry::LinearRing(lr) => lr.serialize(serializer),
            Geometry::Polygon(p) => p.serialize(serializer),
            Geometry::MultiPolygon(mp) => mp.serialize(serializer),
        }
    }
}


impl From<Point> for Geometry {
    fn from(x: Point) -> Self {
        Self::Point(x)
    }
}
impl From<MultiPoint> for Geometry {
    fn from(x: MultiPoint) -> Self {
        Self::MultiPoint(x)
    }
}
impl From<LineString> for Geometry {
    fn from(x: LineString) -> Self {
        Self::LineString(x)
    }
}
impl From<MultiLineString> for Geometry {
    fn from(x: MultiLineString) -> Self {
        Self::MultiLineString(x)
    }
}
impl From<LinearRing> for Geometry {
    fn from(x: LinearRing) -> Self {
        Self::LinearRing(x)
    }
}
impl From<Polygon> for Geometry {
    fn from(x: Polygon) -> Self {
        Self::Polygon(x)
    }
}
impl From<MultiPolygon> for Geometry {
    fn from(x: MultiPolygon) -> Self {
        Self::MultiPolygon(x)
    }
}
