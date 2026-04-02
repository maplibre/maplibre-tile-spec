use arbitrary::Error::IncorrectFormat;
use geo_types::Point;

use crate::encoder::EncodedGeometry;
use crate::geojson::{Coord32, Geom32};
use crate::v01::GeometryValues;

#[derive(Debug, Clone, PartialEq, PartialOrd, arbitrary::Arbitrary)]
enum ArbitraryGeometry {
    Point((i32, i32)),
    // FIXME: Add LineString, Polygon, MultiPoint, MultiLineString, MultiPolygon, once supported upstream
}

impl From<ArbitraryGeometry> for Geom32 {
    fn from(value: ArbitraryGeometry) -> Self {
        match value {
            ArbitraryGeometry::Point((x, y)) => Self::Point(Point(Coord32 { x, y })),
            // FIXME: once fully working, add the rest
        }
    }
}

impl arbitrary::Arbitrary<'_> for GeometryValues {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let geoms = u.arbitrary_iter::<ArbitraryGeometry>()?;
        let mut decoded = Self::default();
        for geo in geoms {
            decoded.push_geom(&Geom32::from(geo?));
        }
        Ok(decoded)
    }
}

impl arbitrary::Arbitrary<'_> for EncodedGeometry {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let decoded = u.arbitrary()?;
        let enc = u.arbitrary()?;
        let geom = Self::encode(&decoded, enc).map_err(|_| IncorrectFormat)?;
        Ok(geom)
    }
}
