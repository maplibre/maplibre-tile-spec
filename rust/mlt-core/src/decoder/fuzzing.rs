use arbitrary::{Arbitrary, Result, Unstructured};
use geo_types::Point;

#[cfg(fuzzing)]
use crate::decoder::ColumnType;
use crate::decoder::GeometryValues;
use crate::geojson::{Coord32, Geom32};
#[allow(
    unused_imports,
    clippy::wildcard_imports,
    reason = "not worth for fuzzing"
)]
use crate::*;

#[cfg(fuzzing)]
/// To make sure we serialize out in the same order as the original file, we need to store the order in which we parsed the columns
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum LayerOrdering {
    Id,
    Geometry,
    Property,
}

#[cfg(fuzzing)]
impl From<ColumnType> for LayerOrdering {
    fn from(typ: ColumnType) -> Self {
        use ColumnType::*;
        match typ {
            OptId | Id | LongId | OptLongId => Self::Id,
            Bool | OptBool | I8 | OptI8 | U8 | OptU8 | I32 | OptI32 | U32 | OptU32 | I64
            | OptI64 | U64 | OptU64 | F32 | OptF32 | F64 | OptF64 | Str | OptStr | SharedDict => {
                Self::Property
            }
            Geometry => Self::Geometry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Arbitrary)]
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

impl Arbitrary<'_> for GeometryValues {
    fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
        // Bound geometry count to prevent OOM from unbounded iteration
        let count = u.int_in_range(1..=32u16)? as usize;
        let mut decoded = Self::default();
        for _ in 0..count {
            let geo: ArbitraryGeometry = u.arbitrary()?;
            decoded.push_geom(&Geom32::from(geo));
        }
        Ok(decoded)
    }
}

impl Arbitrary<'_> for IdValues {
    fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
        // Bound ID count to prevent OOM from unbounded vector generation
        let count = u.int_in_range(0..=64u8)? as usize;
        let values: Vec<Option<u64>> = (0..count).map(|_| u.arbitrary()).collect::<Result<_>>()?;
        Ok(Self(values))
    }
}
