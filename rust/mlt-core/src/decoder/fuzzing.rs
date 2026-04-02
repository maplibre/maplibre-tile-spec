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

impl arbitrary::Arbitrary<'_> for EncodedLayer01 {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        // Bound name length to prevent OOM from unbounded string generation
        let name_len = u.int_in_range(0..=32u8)? as usize;
        let name: String = (0..name_len)
            .map(|_| u.arbitrary::<char>())
            .collect::<arbitrary::Result<_>>()?;
        let extent: u32 = u.arbitrary()?;
        let id: Option<EncodedId> = if u.arbitrary()? {
            Some(u.arbitrary()?)
        } else {
            None
        };
        let geometry = u.arbitrary()?;
        // Bound property count to prevent OOM from unbounded vector generation
        let prop_count = u.int_in_range(0..=4u8)? as usize;
        let properties: Vec<EncodedProperty> = (0..prop_count)
            .map(|_| u.arbitrary())
            .collect::<arbitrary::Result<_>>()?;

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

use arbitrary::Error::IncorrectFormat;
use geo_types::Point;

use crate::geojson::{Coord32, Geom32};
use crate::v01::{EncodedGeometry, GeometryValues};

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
        // Bound geometry count to prevent OOM from unbounded iteration
        let count = u.int_in_range(0..=32u16)? as usize;
        let mut decoded = Self::default();
        for _ in 0..count {
            let geo: ArbitraryGeometry = u.arbitrary()?;
            decoded.push_geom(&Geom32::from(geo));
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

use arbitrary::Error::IncorrectFormat;

use crate::v01::{EncodedId, IdEncoder, IdValues};

impl arbitrary::Arbitrary<'_> for IdValues {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        // Bound ID count to prevent OOM from unbounded vector generation
        let count = u.int_in_range(0..=64u8)? as usize;
        let values: Vec<Option<u64>> = (0..count)
            .map(|_| u.arbitrary())
            .collect::<arbitrary::Result<_>>()?;
        Ok(Self(values))
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

use arbitrary::Error::IncorrectFormat;
use arbitrary::Unstructured;

use crate::v01::{EncodedProperty, ScalarEncoder, StagedProperty, StagedSharedDict, StagedStrings};

impl<'a> arbitrary::Arbitrary<'a> for StagedSharedDict {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        // Bound item count and string sizes to prevent OOM
        let item_count = u.int_in_range(0..=8u8)? as usize;
        let items_raw: Vec<(String, Vec<Option<String>>)> = (0..item_count)
            .map(|_| -> arbitrary::Result<_> {
                let name = bounded_string(u, 32)?;
                let val_count = u.int_in_range(0..=16u8)? as usize;
                let values: Vec<Option<String>> = (0..val_count)
                    .map(|_| -> arbitrary::Result<_> {
                        if u.arbitrary()? {
                            Ok(Some(bounded_string(u, 64)?))
                        } else {
                            Ok(None)
                        }
                    })
                    .collect::<arbitrary::Result<_>>()?;
                Ok((name, values))
            })
            .collect::<arbitrary::Result<_>>()?;
        if items_raw.is_empty() {
            return Ok(Self {
                prefix: bounded_string(u, 32)?,
                data: String::new(),
                items: Vec::new(),
            });
        }
        let prefix = bounded_string(u, 32)?;
        Self::new(prefix, items_raw).map_err(|_| IncorrectFormat)
    }
}

impl arbitrary::Arbitrary<'_> for EncodedProperty {
    fn arbitrary(u: &mut Unstructured<'_>) -> arbitrary::Result<Self> {
        let decoded: StagedProperty = u.arbitrary()?;
        let encoder: ScalarEncoder = u.arbitrary()?;
        let prop: Option<Self> = Self::encode(&decoded, encoder).map_err(|_| IncorrectFormat)?;
        prop.ok_or(IncorrectFormat)
    }
}

impl arbitrary::Arbitrary<'_> for StagedProperty {
    fn arbitrary(u: &mut Unstructured<'_>) -> arbitrary::Result<Self> {
        // Bound value count to prevent OOM from unbounded vector generation
        let count = u.int_in_range(0..=64u8)? as usize;
        let values: Vec<Option<u32>> = (0..count)
            .map(|_| u.arbitrary())
            .collect::<arbitrary::Result<_>>()?;
        Ok(Self::u32("prop", values))
    }
}

impl arbitrary::Arbitrary<'_> for StagedStrings {
    fn arbitrary(u: &mut Unstructured<'_>) -> arbitrary::Result<Self> {
        let name = bounded_string(u, 32)?;
        // Bound string count and individual string lengths to prevent OOM
        let count = u.int_in_range(0..=16u8)? as usize;
        let values: Vec<Option<String>> = (0..count)
            .map(|_| -> arbitrary::Result<_> {
                if u.arbitrary()? {
                    Ok(Some(bounded_string(u, 64)?))
                } else {
                    Ok(None)
                }
            })
            .collect::<arbitrary::Result<_>>()?;
        Ok(Self::from_optional(name, values))
    }
}

/// Generate a string with bounded length to prevent OOM from unbounded string generation.
fn bounded_string(u: &mut Unstructured<'_>, max_len: u8) -> arbitrary::Result<String> {
    let len = u.int_in_range(0..=max_len)? as usize;
    (0..len)
        .map(|_| u.arbitrary::<char>())
        .collect::<arbitrary::Result<_>>()
}
