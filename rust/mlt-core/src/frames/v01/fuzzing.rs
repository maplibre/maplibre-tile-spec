use arbitrary::Error::IncorrectFormat;

use crate::encoder::{EncodedId, IdEncoder};
#[allow(
    unused_imports,
    clippy::wildcard_imports,
    reason = "not worth for fuzzing"
)]
use crate::v01::*;

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
        use crate::frames::v01::model::ColumnType::*;
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

#[cfg(all(not(test), feature = "arbitrary"))]
impl arbitrary::Arbitrary<'_> for EncodedLayer01 {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let name: String = u.arbitrary()?;
        let extent: u32 = u.arbitrary()?;
        let id: Option<EncodedId> = if u.arbitrary()? {
            Some(u.arbitrary()?)
        } else {
            None
        };
        let geometry = u.arbitrary()?;
        let properties: Vec<EncodedProperty> = u.arbitrary()?;

        #[cfg(fuzzing)]
        let layer_order = {
            // Build a valid layer_order and Fisher-Yates shuffle it.
            let mut layer_order: Vec<LayerOrdering> = Vec::new();
            if id.is_some() {
                layer_order.push(LayerOrdering::Id);
            }
            layer_order.push(LayerOrdering::Geometry);
            for _ in &properties {
                layer_order.push(LayerOrdering::Property);
            }
            let n = layer_order.len();
            for i in (1..n).rev() {
                let j: usize = u.int_in_range(0..=i)?;
                layer_order.swap(i, j);
            }
            layer_order
        };

        Ok(Self {
            name,
            extent,
            id,
            geometry,
            properties,
            #[cfg(fuzzing)]
            layer_order,
        })
    }
}

use geo_types::Point;

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

impl arbitrary::Arbitrary<'_> for EncodedId {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let parsed: IdValues = u.arbitrary()?;
        let encoder: IdEncoder = u.arbitrary()?;
        let owned_id = Self::encode(&parsed, encoder).map_err(|_| IncorrectFormat)?;
        Ok(owned_id)
    }
}

impl<'a> arbitrary::Arbitrary<'a> for StagedSharedDict {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let items_raw: Vec<(String, Vec<Option<String>>)> = u.arbitrary()?;
        if items_raw.is_empty() {
            return Ok(Self {
                prefix: u.arbitrary()?,
                data: String::new(),
                items: Vec::new(),
            });
        }
        let prefix: String = u.arbitrary()?;
        Self::new(prefix, items_raw).map_err(|_| IncorrectFormat)
    }
}

impl arbitrary::Arbitrary<'_> for EncodedProperty {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let decoded: StagedProperty = u.arbitrary()?;
        let encoder: ScalarEncoder = u.arbitrary()?;
        let prop: Option<Self> = Self::encode(&decoded, encoder).map_err(|_| IncorrectFormat)?;
        prop.ok_or(IncorrectFormat)
    }
}

impl arbitrary::Arbitrary<'_> for StagedProperty {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let values: Vec<Option<u32>> = u.arbitrary()?;
        Ok(Self::u32("prop", values))
    }
}

impl arbitrary::Arbitrary<'_> for StagedStrings {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        Ok(Self::from_optional(
            u.arbitrary::<String>()?,
            u.arbitrary::<Vec<Option<String>>>()?,
        ))
    }
}
