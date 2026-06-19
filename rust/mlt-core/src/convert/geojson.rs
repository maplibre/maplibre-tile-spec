//! `GeoJSON` -like data to represent decoded MLT data with i32 coordinates

use std::collections::BTreeMap;
use std::str::FromStr;

use serde::ser::SerializeMap as _;
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};

use crate::decoder::{Layer, PropValueRef};
use crate::{LendingIterator, MltResult, ParsedLayer};

/// `GeoJSON` [`FeatureCollection`]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeatureCollection {
    #[serde(rename = "type")]
    pub ty: String,
    pub features: Vec<Feature>,
}

impl FeatureCollection {
    /// Convert already-decoded layers to a `GeoJSON` [`FeatureCollection`], consuming them.
    /// Make sure to call `decode_all` on Layer before calling this (won't compile otherwise)
    pub fn from_layers<'a>(layers: impl IntoIterator<Item = ParsedLayer<'a>>) -> MltResult<Self> {
        let mut features = Vec::new();
        for layer in layers {
            let Layer::Tag01(parsed) = layer else {
                continue;
            };
            let layer_name = parsed.name();
            let extent = parsed.extent().get();
            let mut feat_iter = parsed.iter_features();
            while let Some(feat) = feat_iter.next() {
                let feat = feat?;
                let mut properties = BTreeMap::new();
                for p in feat.iter_properties() {
                    properties.insert(p.name().to_string(), p.value().into());
                }
                properties.insert("_layer".into(), Value::String(layer_name.to_string()));
                properties.insert("_extent".into(), Value::Number(extent.into()));
                features.push(Feature {
                    geometry: feat.geometry().clone(),
                    id: feat.id(),
                    properties,
                    ty: "Feature".into(),
                });
            }
        }
        Ok(Self {
            features,
            ty: "FeatureCollection".into(),
        })
    }

    pub fn equals(&self, other: &Self) -> Result<bool, serde_json::Error> {
        let self_val = normalize_tiny_floats(serde_json::to_value(self)?);
        let other_val = normalize_tiny_floats(serde_json::to_value(other)?);
        Ok(json_values_equal(&self_val, &other_val))
    }
}

impl FromStr for FeatureCollection {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

/// `GeoJSON` [`Feature`]
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Feature {
    #[serde(with = "geom_serde")]
    pub geometry: wkt::Wkt<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    #[serde(default)]
    pub properties: BTreeMap<String, Value>,
    #[serde(rename = "type")]
    pub ty: String,
}

struct Geom32Wire<'a>(&'a wkt::Wkt<i32>);
impl Serialize for Geom32Wire<'_> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        geom_serde::serialize(self.0, s)
    }
}

/// Serialize with the preferred order of the keys
impl Serialize for Feature {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let len = 3 + usize::from(self.id.is_some());
        let mut map = serializer.serialize_map(Some(len))?;
        map.serialize_entry("type", &self.ty)?;
        if let Some(id) = self.id {
            map.serialize_entry("id", &id)?;
        }
        map.serialize_entry("properties", &self.properties)?;
        map.serialize_entry("geometry", &Geom32Wire(&self.geometry))?;
        map.end()
    }
}

/// Serialize/deserialize [`wkt::Wkt<i32>`] in `GeoJSON` wire format:
/// `{"type":"…","coordinates":…}`.
///
/// Each coordinate is a `[x, y]` array, or `[x, y, z]` when the geometry carries a Z (3D).
/// The container [`Dimension`] is inferred from the first coordinate on deserialize.
mod geom_serde {
    use serde::de::Error as _;
    use serde::ser::{Error, SerializeMap as _};
    use serde::{Deserialize, Deserializer, Serializer};
    use serde_json::Value;
    use wkt::Wkt;
    use wkt::types::{
        Coord, Dimension, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
    };

    /// A coordinate as a 2- or 3-element array (`[x, y]` or `[x, y, z]`).
    type Arr = Vec<i32>;

    fn coord_arr(c: &Coord<i32>) -> Arr {
        match c.z {
            Some(z) => vec![c.x, c.y, z],
            None => vec![c.x, c.y],
        }
    }

    fn point_arr(p: &Point<i32>) -> Option<Arr> {
        p.coord().map(coord_arr)
    }

    fn ls_arr(ls: &LineString<i32>) -> Vec<Arr> {
        ls.coords().iter().map(coord_arr).collect()
    }

    fn poly_arr(poly: &Polygon<i32>) -> Vec<Vec<Arr>> {
        poly.rings().iter().map(ls_arr).collect()
    }

    fn arr_coord(a: &Arr) -> Coord<i32> {
        Coord {
            x: a[0],
            y: a[1],
            z: a.get(2).copied(),
            m: None,
        }
    }

    /// Infer the [`Dimension`] of a coordinate sequence from its first coordinate.
    fn dim_of(coords: &[Coord<i32>]) -> Dimension {
        coords.first().map_or(Dimension::XY, Coord::dimension)
    }

    fn arr_ls(v: Vec<Arr>) -> LineString<i32> {
        let coords: Vec<Coord<i32>> = v.into_iter().map(|a| arr_coord(&a)).collect();
        let dim = dim_of(&coords);
        LineString::new(coords, dim)
    }

    fn arr_poly(rings: Vec<Vec<Arr>>) -> Polygon<i32> {
        let rings: Vec<LineString<i32>> = rings.into_iter().map(arr_ls).collect();
        let dim = rings.first().map_or(Dimension::XY, LineString::dimension);
        Polygon::new(rings, dim)
    }

    pub fn serialize<S: Serializer>(g: &Wkt<i32>, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(Some(2))?;
        let (ty, coords): (&str, Value) = match g {
            Wkt::Point(p) => (
                "Point",
                serde_json::to_value(point_arr(p).unwrap_or_default()).unwrap(),
            ),
            Wkt::LineString(ls) => ("LineString", serde_json::to_value(ls_arr(ls)).unwrap()),
            Wkt::Polygon(poly) => ("Polygon", serde_json::to_value(poly_arr(poly)).unwrap()),
            Wkt::MultiPoint(mp) => (
                "MultiPoint",
                serde_json::to_value(mp.points().iter().filter_map(point_arr).collect::<Vec<_>>())
                    .unwrap(),
            ),
            Wkt::MultiLineString(mls) => (
                "MultiLineString",
                serde_json::to_value(mls.line_strings().iter().map(ls_arr).collect::<Vec<_>>())
                    .unwrap(),
            ),
            Wkt::MultiPolygon(mpoly) => (
                "MultiPolygon",
                serde_json::to_value(mpoly.polygons().iter().map(poly_arr).collect::<Vec<_>>())
                    .unwrap(),
            ),
            Wkt::GeometryCollection(_) => {
                return Err(Error::custom("unsupported geometry variant"));
            }
        };
        m.serialize_entry("type", ty)?;
        m.serialize_entry("coordinates", &coords)?;
        m.end()
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Wkt<i32>, D::Error> {
        fn parse<T: serde::de::DeserializeOwned, E: serde::de::Error>(v: Value) -> Result<T, E> {
            serde_json::from_value(v).map_err(E::custom)
        }

        #[derive(Deserialize)]
        struct Wire {
            #[serde(rename = "type")]
            ty: String,
            coordinates: Value,
        }

        let Wire { ty, coordinates: c } = Wire::deserialize(d)?;
        Ok(match ty.as_str() {
            "Point" => Wkt::Point(Point::from_coord(arr_coord(&parse::<Arr, _>(c)?))),
            "LineString" => Wkt::LineString(arr_ls(parse(c)?)),
            "Polygon" => Wkt::Polygon(arr_poly(parse(c)?)),
            "MultiPoint" => {
                let v: Vec<Arr> = parse(c)?;
                let points: Vec<Point<i32>> =
                    v.iter().map(|a| Point::from_coord(arr_coord(a))).collect();
                let dim = points.first().map_or(Dimension::XY, Point::dimension);
                Wkt::MultiPoint(MultiPoint::new(points, dim))
            }
            "MultiLineString" => {
                let v: Vec<Vec<Arr>> = parse(c)?;
                let lines: Vec<LineString<i32>> = v.into_iter().map(arr_ls).collect();
                let dim = lines.first().map_or(Dimension::XY, LineString::dimension);
                Wkt::MultiLineString(MultiLineString::new(lines, dim))
            }
            "MultiPolygon" => {
                let v: Vec<Vec<Vec<Arr>>> = parse(c)?;
                let polys: Vec<Polygon<i32>> = v.into_iter().map(arr_poly).collect();
                let dim = polys.first().map_or(Dimension::XY, Polygon::dimension);
                Wkt::MultiPolygon(MultiPolygon::new(polys, dim))
            }
            _ => {
                return Err(D::Error::unknown_variant(
                    &ty,
                    &[
                        "Point",
                        "LineString",
                        "Polygon",
                        "MultiPoint",
                        "MultiLineString",
                        "MultiPolygon",
                    ],
                ));
            }
        })
    }
}

/// Convert f32 to `GeoJSON` value: finite as number, non-finite as string per issue #978.
#[must_use]
pub fn f32_to_json(f: f32) -> Value {
    if f.is_nan() {
        Value::String("f32::NAN".to_owned())
    } else if f == f32::INFINITY {
        Value::String("f32::INFINITY".to_owned())
    } else if f == f32::NEG_INFINITY {
        Value::String("f32::NEG_INFINITY".to_owned())
    } else {
        Number::from_f64(f64::from(f)).expect("finite f32").into()
    }
}

/// Convert f64 to `GeoJSON` value: finite as number, non-finite as string per issue #978.
#[must_use]
pub fn f64_to_json(f: f64) -> Value {
    if f.is_nan() {
        Value::String("f64::NAN".to_owned())
    } else if f == f64::INFINITY {
        Value::String("f64::INFINITY".to_owned())
    } else if f == f64::NEG_INFINITY {
        Value::String("f64::NEG_INFINITY".to_owned())
    } else {
        Number::from_f64(f).expect("finite f64").into()
    }
}

impl From<PropValueRef<'_>> for Value {
    fn from(v: PropValueRef<'_>) -> Self {
        match v {
            PropValueRef::Bool(v) => Self::Bool(v),
            PropValueRef::I8(v) => Self::from(v),
            PropValueRef::U8(v) => Self::from(v),
            PropValueRef::I32(v) => Self::from(v),
            PropValueRef::U32(v) => Self::from(v),
            PropValueRef::I64(v) => Self::from(v),
            PropValueRef::U64(v) => Self::from(v),
            PropValueRef::F32(v) => f32_to_json(v),
            PropValueRef::F64(v) => f64_to_json(v),
            PropValueRef::Str(s) => Self::String(s.to_string()),
        }
    }
}

/// Replace tiny float values (e.g. `1e-40`) with `0.0` to handle codec precision issues.
fn normalize_tiny_floats(value: Value) -> Value {
    match value {
        Value::Number(ref n) => {
            let eps = f64::from(f32::EPSILON);
            if let Some(f) = n.as_f64()
                && f.is_finite()
                && f.abs() < eps
            {
                Value::from(0.0)
            } else {
                value
            }
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(normalize_tiny_floats).collect()),
        Value::Object(obj) => Value::Object(
            obj.into_iter()
                .map(|(k, v)| (k, normalize_tiny_floats(v)))
                .collect(),
        ),
        v => v,
    }
}

/// Compare two JSON values for equality. Numbers are compared with float tolerance so that
/// f32 round-trip (e.g. 3.14 vs 3.140000104904175) and Java minimal decimal (e.g. 3.4028235e+38)
/// match the Rust decoder output.
fn json_values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(na), Value::Number(nb)) if na.is_f64() && nb.is_f64() => {
            let na = na.as_f64().expect("f64");
            let nb = nb.as_f64().expect("f64");
            assert!(
                !na.is_nan() && !nb.is_nan(),
                "unexpected non-finite numbers"
            );
            let abs_diff = (na - nb).abs();
            let max_abs = na.abs().max(nb.abs()).max(1.0);
            abs_diff <= f64::from(f32::EPSILON) * max_abs * 2.0
        }
        (Value::Array(aa), Value::Array(ab)) => {
            aa.len() == ab.len()
                && aa
                    .iter()
                    .zip(ab.iter())
                    .all(|(x, y)| json_values_equal(x, y))
        }
        (Value::Object(ao), Value::Object(bo)) => {
            ao.len() == bo.len()
                && ao
                    .iter()
                    .all(|(k, v)| bo.get(k).is_some_and(|w| json_values_equal(v, w)))
        }
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use wkt::Wkt;
    use wkt::types::{Coord, Dimension, LineString, Point};

    use super::*;

    fn feature(geometry: Wkt<i32>) -> Feature {
        Feature {
            geometry,
            id: None,
            properties: BTreeMap::new(),
            ty: "Feature".into(),
        }
    }

    #[test]
    fn linestring_3d_serde_roundtrip() {
        let geom = Wkt::LineString(LineString::new(
            vec![
                Coord {
                    x: 1,
                    y: 2,
                    z: Some(3),
                    m: None,
                },
                Coord {
                    x: 4,
                    y: 5,
                    z: Some(6),
                    m: None,
                },
            ],
            Dimension::XYZ,
        ));
        let json = serde_json::to_string(&feature(geom.clone())).unwrap();
        assert!(json.contains("[1,2,3]"), "expected 3D coords in {json}");
        let back: Feature = serde_json::from_str(&json).unwrap();
        assert_eq!(back.geometry, geom);
    }

    #[test]
    fn point_2d_serde_has_no_z() {
        let geom = Wkt::Point(Point::from_coord(Coord {
            x: 1,
            y: 2,
            z: None,
            m: None,
        }));
        let json = serde_json::to_string(&feature(geom.clone())).unwrap();
        assert!(json.contains("[1,2]"), "expected 2D coords in {json}");
        let back: Feature = serde_json::from_str(&json).unwrap();
        assert_eq!(back.geometry, geom);
    }
}
