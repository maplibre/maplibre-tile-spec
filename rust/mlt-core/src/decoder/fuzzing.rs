use arbitrary::{Arbitrary, Result, Unstructured};
use geo_types::{Coord, Geometry, Point};
use usize_cast::IntoUsize as _;

#[cfg(fuzzing)]
use crate::decoder::ColumnType;
use crate::decoder::GeometryValues;
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
            Bool | OptBool | I32 | OptI32 | U32 | OptU32 | I64 | OptI64 | U64 | OptU64 | F32
            | OptF32 | F64 | OptF64 | Str | OptStr | SharedDict => Self::Property,
            Geometry => Self::Geometry,
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Arbitrary)]
enum ArbitraryGeometry {
    Point((i32, i32)),
    // FIXME: Add LineString, Polygon, MultiPoint, MultiLineString, MultiPolygon, once supported upstream
}

impl From<ArbitraryGeometry> for Geometry<i32> {
    fn from(value: ArbitraryGeometry) -> Self {
        match value {
            ArbitraryGeometry::Point((x, y)) => Self::Point(Point(Coord::<i32> { x, y })), // FIXME: once fully working, add the rest
        }
    }
}

impl Arbitrary<'_> for GeometryValues {
    fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
        // Bound geometry count to prevent OOM from unbounded iteration
        let count = u.int_in_range(1..=32u16)?.into_usize();
        #[cfg(feature = "unstable-v2")]
        if u.ratio(1, 4)? {
            let mut decoded = Self::default();
            for _ in 0..count.min(8) {
                let n_verts = u.int_in_range(0..=8u8)?;
                let coords: Vec<Coord<i32>> = (0..n_verts)
                    .map(|_| {
                        Ok(Coord {
                            x: u.arbitrary()?,
                            y: u.arbitrary()?,
                        })
                    })
                    .collect::<Result<_>>()?;
                let n_tris = if n_verts == 0 {
                    0
                } else {
                    u.int_in_range(0..=8u8)?
                };
                let tris: Vec<[u32; 3]> = (0..n_tris)
                    .map(|_| {
                        let v = u32::from(n_verts - 1);
                        Ok([
                            u.int_in_range(0..=v)?,
                            u.int_in_range(0..=v)?,
                            u.int_in_range(0..=v)?,
                        ])
                    })
                    .collect::<Result<_>>()?;
                decoded.push_mesh(coords.into_iter(), &tris);
            }
            return Ok(decoded);
        }
        let mut decoded = Self::default();
        for _ in 0..count {
            let geo: ArbitraryGeometry = u.arbitrary()?;
            decoded.push_geom(&Geometry::<i32>::from(geo));
        }
        Ok(decoded)
    }
}
