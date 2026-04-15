//! `GeoJSON` -like data to represent decoded MLT data with i32 coordinates

use std::collections::BTreeMap;
use std::str::FromStr;

use serde::ser::SerializeMap as _;
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};

use crate::decoder::{Layer, PropValueRef};
use crate::{Geom32, MltResult, ParsedLayer};

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
            let layer_name = parsed.name;
            let extent = parsed.extent;
            for feat in parsed.iter_features() {
                let feat = feat?;
                let mut properties = BTreeMap::new();
                for p in feat.iter_properties() {
                    properties.insert(p.name.to_string(), p.value.into());
                }
                properties.insert("_layer".into(), Value::String(layer_name.to_string()));
                properties.insert("_extent".into(), Value::Number(extent.into()));
                features.push(Feature {
                    geometry: feat.geometry,
                    id: feat.id,
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
    pub geometry: Geom32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    pub properties: BTreeMap<String, Value>,
    #[serde(rename = "type")]
    pub ty: String,
}

struct Geom32Wire<'a>(&'a Geom32);
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

/// Serialize/deserialize [`Geom32`] in `GeoJSON` wire format:
/// `{"type":"…","coordinates":…}` with `[x, y]` integer arrays.
mod geom_serde {
    use geo_types::{
        Geometry, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
    };
    use serde::de::Error as _;
    use serde::ser::{Error, SerializeMap as _};
    use serde::{Deserialize, Deserializer, Serializer};
    use serde_json::Value;

    use crate::Geom32;

    type Arr = [i32; 2];

    fn ls_arr(ls: &LineString<i32>) -> Vec<Arr> {
        ls.0.iter().copied().map(Into::into).collect()
    }

    fn poly_arr(poly: &Polygon<i32>) -> Vec<Vec<Arr>> {
        std::iter::once(poly.exterior())
            .chain(poly.interiors())
            .map(ls_arr)
            .collect()
    }

    fn arr_ls(v: Vec<Arr>) -> LineString<i32> {
        LineString::from(v)
    }

    fn arr_poly(rings: Vec<Vec<Arr>>) -> Polygon<i32> {
        let mut it = rings.into_iter();
        let ext = it.next().map_or_else(|| LineString(vec![]), arr_ls);
        Polygon::new(ext, it.map(arr_ls).collect())
    }

    pub fn serialize<S: Serializer>(g: &Geom32, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(Some(2))?;
        let (ty, coords): (&str, Value) = match g {
            Geometry::Point(p) => ("Point", serde_json::to_value(Arr::from(*p)).unwrap()),
            Geometry::LineString(ls) => ("LineString", serde_json::to_value(ls_arr(ls)).unwrap()),
            Geometry::Polygon(poly) => ("Polygon", serde_json::to_value(poly_arr(poly)).unwrap()),
            Geometry::MultiPoint(mp) => (
                "MultiPoint",
                serde_json::to_value(mp.0.iter().copied().map(Arr::from).collect::<Vec<_>>())
                    .unwrap(),
            ),
            Geometry::MultiLineString(mls) => (
                "MultiLineString",
                serde_json::to_value(mls.iter().map(ls_arr).collect::<Vec<_>>()).unwrap(),
            ),
            Geometry::MultiPolygon(mpoly) => (
                "MultiPolygon",
                serde_json::to_value(mpoly.iter().map(poly_arr).collect::<Vec<_>>()).unwrap(),
            ),
            _ => return Err(Error::custom("unsupported geometry variant")),
        };
        m.serialize_entry("type", ty)?;
        m.serialize_entry("coordinates", &coords)?;
        m.end()
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Geom32, D::Error> {
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
            "Point" => Geometry::Point(Point::from(parse::<Arr, _>(c)?)),
            "LineString" => Geometry::LineString(arr_ls(parse(c)?)),
            "Polygon" => Geometry::Polygon(arr_poly(parse(c)?)),
            "MultiPoint" => {
                let v: Vec<Arr> = parse(c)?;
                Geometry::MultiPoint(MultiPoint(v.into_iter().map(Point::from).collect()))
            }
            "MultiLineString" => {
                let v: Vec<Vec<Arr>> = parse(c)?;
                Geometry::MultiLineString(MultiLineString(v.into_iter().map(arr_ls).collect()))
            }
            "MultiPolygon" => {
                let v: Vec<Vec<Vec<Arr>>> = parse(c)?;
                Geometry::MultiPolygon(MultiPolygon(v.into_iter().map(arr_poly).collect()))
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
