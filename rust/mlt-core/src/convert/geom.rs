//! Conversions between geometry representations.
//!
//! MLT accepts any geometry that implements [`geo_traits::GeometryTrait`] as encoder input
//! (so both [`geo_types`] and [`wkt`] values work), and produces [`wkt::Wkt`] as decoder output
//! (the dimension-aware type that can carry an optional Z coordinate).
//!
//! [`to_wkt`] normalizes any geo-traits geometry into a [`wkt::Wkt`], preserving coordinate
//! dimensionality. [`geo_traits::to_geo::ToGeoGeometry::to_geometry`] handles the reverse
//! (`wkt::Wkt` -> `geo_types::Geometry`, dropping any Z) for the 2D MVT boundary.

use geo_traits::{
    CoordTrait, Dimensions, GeometryCollectionTrait, GeometryTrait, GeometryType, LineStringTrait,
    LineTrait, MultiLineStringTrait, MultiPointTrait, MultiPolygonTrait, PointTrait, PolygonTrait,
    RectTrait, TriangleTrait,
};
use wkt::Wkt;
use wkt::types::{
    Coord, Dimension, GeometryCollection, LineString, MultiLineString, MultiPoint, MultiPolygon,
    Point, Polygon,
};

/// Map a geo-traits [`Dimensions`] to the wkt [`Dimension`] enum.
fn wkt_dim(dim: Dimensions) -> Dimension {
    match dim {
        Dimensions::Xyz | Dimensions::Unknown(3) => Dimension::XYZ,
        Dimensions::Xym => Dimension::XYM,
        Dimensions::Xyzm | Dimensions::Unknown(4..) => Dimension::XYZM,
        Dimensions::Xy | Dimensions::Unknown(_) => Dimension::XY,
    }
}

/// Build a wkt [`Coord`] from any [`CoordTrait`], preserving Z / M according to its dimension.
fn coord_of<C: CoordTrait<T = i32>>(c: &C) -> Coord<i32> {
    let (z, m) = match c.dim() {
        Dimensions::Xyz | Dimensions::Unknown(3) => (c.nth(2), None),
        Dimensions::Xym => (None, c.nth(2)),
        Dimensions::Xyzm | Dimensions::Unknown(4..) => (c.nth(2), c.nth(3)),
        Dimensions::Xy | Dimensions::Unknown(_) => (None, None),
    };
    Coord {
        x: c.x(),
        y: c.y(),
        z,
        m,
    }
}

fn point_of<P: PointTrait<T = i32>>(p: &P) -> Point<i32> {
    let dim = wkt_dim(p.dim());
    Point::new(p.coord().map(|c| coord_of(&c)), dim)
}

fn line_string_of<L: LineStringTrait<T = i32>>(ls: &L) -> LineString<i32> {
    let dim = wkt_dim(ls.dim());
    let coords = ls.coords().map(|c| coord_of(&c)).collect();
    LineString::new(coords, dim)
}

fn polygon_of<P: PolygonTrait<T = i32>>(poly: &P) -> Polygon<i32> {
    let dim = wkt_dim(poly.dim());
    let rings = poly
        .exterior()
        .into_iter()
        .chain(poly.interiors())
        .map(|r| line_string_of(&r))
        .collect();
    Polygon::new(rings, dim)
}

/// Normalize any [`geo_traits::GeometryTrait`] geometry into a [`wkt::Wkt`], preserving
/// coordinate dimensionality. `Rect` and `Triangle` are converted to polygons and `Line`
/// to a line string, mirroring the encoder's handling of those geo-types-only variants.
pub(crate) fn to_wkt(geom: &impl GeometryTrait<T = i32>) -> Wkt<i32> {
    match geom.as_type() {
        GeometryType::Point(p) => Wkt::Point(point_of(p)),
        GeometryType::LineString(ls) => Wkt::LineString(line_string_of(ls)),
        GeometryType::Polygon(poly) => Wkt::Polygon(polygon_of(poly)),
        GeometryType::MultiPoint(mp) => {
            let dim = wkt_dim(mp.dim());
            let points = mp.points().map(|p| point_of(&p)).collect();
            Wkt::MultiPoint(MultiPoint::new(points, dim))
        }
        GeometryType::MultiLineString(mls) => {
            let dim = wkt_dim(mls.dim());
            let lines = mls.line_strings().map(|ls| line_string_of(&ls)).collect();
            Wkt::MultiLineString(MultiLineString::new(lines, dim))
        }
        GeometryType::MultiPolygon(mp) => {
            let dim = wkt_dim(mp.dim());
            let polys = mp.polygons().map(|p| polygon_of(&p)).collect();
            Wkt::MultiPolygon(MultiPolygon::new(polys, dim))
        }
        GeometryType::GeometryCollection(gc) => {
            // MLT itself never produces a GeometryCollection, but a caller may pass one as
            // encoder input. Flatten the contained geometries into a wkt GeometryCollection.
            let dim = wkt_dim(gc.dim());
            let geoms = gc.geometries().map(|g| to_wkt(&g)).collect();
            Wkt::GeometryCollection(GeometryCollection::new(geoms, dim))
        }
        GeometryType::Rect(r) => Wkt::Polygon(rect_to_polygon(r)),
        GeometryType::Triangle(t) => Wkt::Polygon(triangle_to_polygon(t)),
        GeometryType::Line(l) => {
            let dim = wkt_dim(l.dim());
            let coords = vec![coord_of(&l.start()), coord_of(&l.end())];
            Wkt::LineString(LineString::new(coords, dim))
        }
    }
}

fn rect_to_polygon(r: &impl RectTrait<T = i32>) -> Polygon<i32> {
    // A rect is inherently 2D; its corners are synthesized from min/max.
    let (min, max) = (r.min(), r.max());
    let (x0, y0, x1, y1) = (min.x(), min.y(), max.x(), max.y());
    let xy = |x, y| Coord {
        x,
        y,
        z: None,
        m: None,
    };
    let coords = vec![xy(x0, y0), xy(x1, y0), xy(x1, y1), xy(x0, y1), xy(x0, y0)];
    Polygon::new(vec![LineString::new(coords, Dimension::XY)], Dimension::XY)
}

fn triangle_to_polygon(t: &impl TriangleTrait<T = i32>) -> Polygon<i32> {
    let dim = wkt_dim(t.dim());
    let coords = vec![
        coord_of(&t.first()),
        coord_of(&t.second()),
        coord_of(&t.third()),
        coord_of(&t.first()),
    ];
    Polygon::new(vec![LineString::new(coords, dim)], dim)
}
