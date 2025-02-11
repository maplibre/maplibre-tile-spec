use crate::types::geometries::coordinate::Coordinate;
use crate::types::geometries::linearring::LinearRing;
use crate::types::geometries::linestring::LineString;
use crate::types::geometries::multi_linestring::MultiLineString;
use crate::types::geometries::multi_point::MultiPoint;
use crate::types::geometries::multi_polygon::MultiPolygon;
use crate::types::geometries::point::Point;
use crate::types::geometries::polygon::Polygon;

pub struct GeometryFactory;

impl GeometryFactory {
    pub fn create_point(coordinate: Coordinate) -> Point {
        Point {
            coordinate
        }
    }

    pub fn create_multi_point(points: Vec<Point>) -> MultiPoint {
        MultiPoint {
            points
        }
    }

    pub fn create_line_string(points: Vec<Coordinate>) -> LineString {
        LineString {
            points
        }
    }
    
    pub fn create_linear_ring(points: Vec<Coordinate>) -> LinearRing {
        LinearRing {
            points
        }
    }

    pub fn create_polygon(shell: LinearRing, rings: Vec<LinearRing>) -> Polygon {
        Polygon {
            shell,
            holes: rings,
        }
    }

    pub fn create_multi_polygon(polygons: Vec<Polygon>) -> MultiPolygon {
        MultiPolygon {
            polygons
        }
    }

    pub fn create_multi_line_string(strings: Vec<LineString>) -> MultiLineString {
        MultiLineString {
            strings
        }
    }
}
