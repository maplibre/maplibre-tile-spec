//! `GeoJSON` -like data to represent decoded MLT data with i32 coordinates

use std::collections::BTreeMap;
use std::str::FromStr;

use geo_types::Geometry;
use serde::ser::SerializeMap as _;
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};

use crate::decoder::PropValueRef;
use crate::{LendingIterator, MltResult, ParsedLayer};
#[cfg(feature = "unstable-v2")]
use crate::{
    ParsedLayer02,
    tile::{MValue, NestedValue},
};

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
            // Read the v2-only columns first: the rest of the loop needs only the
            // shared ones, which is all `parsed` keeps.
            #[cfg(feature = "unstable-v2")]
            let mut m_values = match &layer {
                ParsedLayer::Tag01(_) | ParsedLayer::Unknown(_) => Vec::new().into_iter(),
                ParsedLayer::Tag02(l) => m_value_properties(l)?.into_iter(),
            };
            #[cfg(feature = "unstable-v2")]
            let mut nested = match &layer {
                ParsedLayer::Tag01(_) | ParsedLayer::Unknown(_) => Vec::new().into_iter(),
                ParsedLayer::Tag02(l) => nested_properties(l)?.into_iter(),
            };
            let parsed = match layer {
                ParsedLayer::Tag01(l) => l,
                #[cfg(feature = "unstable-v2")]
                ParsedLayer::Tag02(l) => l.into_layer(),
                ParsedLayer::Unknown(_) => continue,
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
                #[cfg(feature = "unstable-v2")]
                properties.extend(m_values.next().unwrap_or_default());
                #[cfg(feature = "unstable-v2")]
                properties.extend(nested.next().unwrap_or_default());
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
    pub geometry: Geometry<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    #[serde(default)]
    pub properties: BTreeMap<String, Value>,
    #[serde(rename = "type")]
    pub ty: String,
}

struct Geom32Wire<'a>(&'a Geometry<i32>);
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

/// Serialize/deserialize [`Geometry<i32>`](geo_types::Geometry) in `GeoJSON` wire format:
/// `{"type":"…","coordinates":…}` with `[x, y]` integer arrays.
mod geom_serde {
    use geo_types::{
        Geometry, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
    };
    use serde::de::Error as _;
    use serde::ser::{Error, SerializeMap as _};
    use serde::{Deserialize, Deserializer, Serializer};
    use serde_json::Value;

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

    pub fn serialize<S: Serializer>(g: &Geometry<i32>, s: S) -> Result<S::Ok, S::Error> {
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
            Geometry::Line(_)
            | Geometry::Rect(_)
            | Geometry::Triangle(_)
            | Geometry::GeometryCollection(_) => {
                return Err(Error::custom("unsupported geometry variant"));
            }
        };
        m.serialize_entry("type", ty)?;
        m.serialize_entry("coordinates", &coords)?;
        m.end()
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Geometry<i32>, D::Error> {
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

/// Every feature's m-values as `m:<name>` properties, in feature order.
///
/// A feature with no values for a column has no property for it, the way a null
/// property is left out.
#[cfg(feature = "unstable-v2")]
fn m_value_properties(layer: &ParsedLayer02<'_>) -> MltResult<Vec<Vec<(String, Value)>>> {
    let geometry = layer.layer().geometry_values();
    let mut features = vec![Vec::new(); geometry.feature_count()];
    for column in layer.m_values() {
        let key = format!("m:{}", column.name());
        for (index, span) in column.spans(geometry).enumerate() {
            let values = column.values().row(column.name(), span?)?;
            if let Some(value) = m_value_to_json(&values) {
                features[index].push((key.clone(), value));
            }
        }
    }
    Ok(features)
}

/// Every feature's nested columns as JSON, one entry per column under its own name.
#[cfg(feature = "unstable-v2")]
fn nested_properties(layer: &ParsedLayer02<'_>) -> MltResult<Vec<Vec<(String, Value)>>> {
    let mut features = vec![Vec::new(); layer.layer().geometry_values().feature_count()];
    for column in layer.nested() {
        for (index, feature) in features.iter_mut().enumerate() {
            feature.push((
                column.name().to_string(),
                nested_to_json(&column.row(index)?),
            ));
        }
    }
    Ok(features)
}

/// One nested value as the JSON it stands for: an object, an array, a scalar or null.
#[cfg(feature = "unstable-v2")]
fn nested_to_json(value: &NestedValue) -> Value {
    match value {
        NestedValue::Leaf(leaf) => prop_to_json(leaf),
        NestedValue::List(None) | NestedValue::Map(None) => Value::Null,
        NestedValue::List(Some(items)) => Value::Array(items.iter().map(nested_to_json).collect()),
        NestedValue::Map(Some(entries)) => Value::Object(
            entries
                .iter()
                .map(|(key, value)| (key.clone(), nested_to_json(value)))
                .collect(),
        ),
    }
}

/// One scalar as JSON, a null value included.
#[cfg(feature = "unstable-v2")]
fn prop_to_json(value: &crate::PropValue) -> Value {
    use crate::PropValue as P;
    match value {
        P::Bool(v) => v.map_or(Value::Null, Value::Bool),
        P::I8(v) => v.map_or(Value::Null, Value::from),
        P::U8(v) => v.map_or(Value::Null, Value::from),
        P::I32(v) => v.map_or(Value::Null, Value::from),
        P::U32(v) => v.map_or(Value::Null, Value::from),
        P::I64(v) => v.map_or(Value::Null, Value::from),
        P::U64(v) => v.map_or(Value::Null, Value::from),
        P::F32(v) => v.map_or(Value::Null, f32_to_json),
        P::F64(v) => v.map_or(Value::Null, f64_to_json),
        P::Str(v) => v.clone().map_or(Value::Null, Value::String),
    }
}

/// One feature's m-values as a JSON array, or [`None`] when it carries none.
#[cfg(feature = "unstable-v2")]
fn m_value_to_json(values: &MValue) -> Option<Value> {
    /// A present run, mapped value by value.
    macro_rules! array {
        ($values:expr, $to_json:expr) => {
            Value::Array($values.iter().copied().map($to_json).collect())
        };
    }
    Some(match values {
        MValue::Bool(v) => array!(v.as_ref()?, Value::Bool),
        MValue::I8(v) => array!(v.as_ref()?, Value::from),
        MValue::U8(v) => array!(v.as_ref()?, Value::from),
        MValue::I32(v) => array!(v.as_ref()?, Value::from),
        MValue::U32(v) => array!(v.as_ref()?, Value::from),
        MValue::I64(v) => array!(v.as_ref()?, Value::from),
        MValue::U64(v) => array!(v.as_ref()?, Value::from),
        MValue::F32(v) => array!(v.as_ref()?, f32_to_json),
        MValue::F64(v) => array!(v.as_ref()?, f64_to_json),
        MValue::Str(v) => Value::Array(
            v.as_ref()?
                .iter()
                .map(|s| Value::String(s.clone()))
                .collect(),
        ),
    })
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
        Value::Null | Value::Bool(_) | Value::String(_) => value,
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
    use geo_types::{
        Coord, GeometryCollection, Line, LineString, MultiLineString, MultiPoint, MultiPolygon,
        Point, Polygon, Rect, Triangle,
    };
    use insta::assert_snapshot;
    use rstest::rstest;

    use super::*;

    fn ring(pts: &[(i32, i32)]) -> LineString<i32> {
        LineString::from(pts.iter().map(|&(x, y)| Coord { x, y }).collect::<Vec<_>>())
    }

    fn square_with_hole() -> Polygon<i32> {
        Polygon::new(
            ring(&[(0, 0), (8, 0), (8, 8), (0, 8), (0, 0)]),
            vec![ring(&[(2, 2), (4, 2), (4, 4), (2, 2)])],
        )
    }

    fn feature(geometry: Geometry<i32>) -> Feature {
        Feature {
            geometry,
            id: None,
            properties: BTreeMap::new(),
            ty: "Feature".into(),
        }
    }

    fn supported_geometries() -> Vec<Geometry<i32>> {
        vec![
            Geometry::Point(Point::new(1, -2)),
            Geometry::LineString(ring(&[(0, 0), (1, 1)])),
            Geometry::LineString(LineString(vec![])),
            Geometry::Polygon(square_with_hole()),
            Geometry::Polygon(Polygon::new(LineString(vec![]), vec![])),
            Geometry::MultiPoint(MultiPoint(vec![Point::new(1, 2), Point::new(3, 4)])),
            Geometry::MultiPoint(MultiPoint(vec![])),
            Geometry::MultiLineString(MultiLineString(vec![ring(&[(0, 0), (1, 1)])])),
            Geometry::MultiPolygon(MultiPolygon(vec![square_with_hole()])),
        ]
    }

    #[test]
    fn supported_geometries_round_trip_through_geojson() {
        let mut rendered = Vec::new();
        for geometry in supported_geometries() {
            let json = serde_json::to_string(&feature(geometry.clone())).expect("serialize");
            let back: Feature = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back.geometry, geometry);
            rendered.push(json);
        }

        assert_snapshot!(rendered.join("\n"), @r#"
        {"type":"Feature","properties":{},"geometry":{"type":"Point","coordinates":[1,-2]}}
        {"type":"Feature","properties":{},"geometry":{"type":"LineString","coordinates":[[0,0],[1,1]]}}
        {"type":"Feature","properties":{},"geometry":{"type":"LineString","coordinates":[]}}
        {"type":"Feature","properties":{},"geometry":{"type":"Polygon","coordinates":[[[0,0],[8,0],[8,8],[0,8],[0,0]],[[2,2],[4,2],[4,4],[2,2]]]}}
        {"type":"Feature","properties":{},"geometry":{"type":"Polygon","coordinates":[[]]}}
        {"type":"Feature","properties":{},"geometry":{"type":"MultiPoint","coordinates":[[1,2],[3,4]]}}
        {"type":"Feature","properties":{},"geometry":{"type":"MultiPoint","coordinates":[]}}
        {"type":"Feature","properties":{},"geometry":{"type":"MultiLineString","coordinates":[[[0,0],[1,1]]]}}
        {"type":"Feature","properties":{},"geometry":{"type":"MultiPolygon","coordinates":[[[[0,0],[8,0],[8,8],[0,8],[0,0]],[[2,2],[4,2],[4,4],[2,2]]]]}}
        "#);
    }

    #[rstest]
    #[case::line(Geometry::Line(Line::new(Coord { x: 0, y: 0 }, Coord { x: 1, y: 1 })))]
    #[case::rect(Geometry::Rect(Rect::new(Coord { x: 0, y: 0 }, Coord { x: 1, y: 1 })))]
    #[case::triangle(Geometry::Triangle(Triangle::new(
        Coord { x: 0, y: 0 },
        Coord { x: 1, y: 0 },
        Coord { x: 0, y: 1 },
    )))]
    #[case::collection(Geometry::GeometryCollection(GeometryCollection(vec![])))]
    fn geometries_outside_geojson_fail_to_serialize(#[case] geometry: Geometry<i32>) {
        let err = serde_json::to_string(&feature(geometry)).expect_err("must not serialize");
        assert_eq!(err.to_string(), "unsupported geometry variant");
    }

    #[test]
    fn unknown_geometry_type_fails_to_deserialize() {
        let err = serde_json::from_str::<Feature>(
            r#"{"type":"Feature","properties":{},"geometry":{"type":"Circle","coordinates":[0,0]}}"#,
        )
        .expect_err("must not deserialize");
        assert_snapshot!(
            err,
            @"unknown variant `Circle`, expected one of `Point`, `LineString`, `Polygon`, `MultiPoint`, `MultiLineString`, `MultiPolygon` at line 1 column 83"
        );
    }

    #[test]
    fn a_feature_collection_round_trips_through_its_text_form() {
        let collection = FeatureCollection {
            ty: "FeatureCollection".into(),
            features: vec![Feature {
                geometry: Geometry::Point(Point::new(7, 9)),
                id: Some(42),
                properties: BTreeMap::from([
                    ("name".into(), Value::String("ß".into())),
                    ("rank".into(), Value::from(3)),
                ]),
                ty: "Feature".into(),
            }],
        };

        let text = serde_json::to_string(&collection).expect("serialize");
        assert_snapshot!(
            text,
            @r#"{"type":"FeatureCollection","features":[{"type":"Feature","id":42,"properties":{"name":"ß","rank":3},"geometry":{"type":"Point","coordinates":[7,9]}}]}"#
        );
        assert_eq!(
            FeatureCollection::from_str(&text).expect("parse"),
            collection
        );
    }

    #[test]
    fn floats_outside_the_json_number_range_become_names() {
        let values = [
            f32_to_json(1.5),
            f32_to_json(f32::NAN),
            f32_to_json(f32::INFINITY),
            f32_to_json(f32::NEG_INFINITY),
            f64_to_json(1.5),
            f64_to_json(f64::NAN),
            f64_to_json(f64::INFINITY),
            f64_to_json(f64::NEG_INFINITY),
        ];
        assert_snapshot!(
            Value::Array(values.to_vec()),
            @r#"[1.5,"f32::NAN","f32::INFINITY","f32::NEG_INFINITY",1.5,"f64::NAN","f64::INFINITY","f64::NEG_INFINITY"]"#
        );
    }

    #[test]
    fn every_property_kind_becomes_json() {
        let values = [
            PropValueRef::Bool(true),
            PropValueRef::I8(-8),
            PropValueRef::U8(8),
            PropValueRef::I32(i32::MIN),
            PropValueRef::U32(u32::MAX),
            PropValueRef::I64(i64::MIN),
            PropValueRef::U64(u64::MAX),
            PropValueRef::F32(0.5),
            PropValueRef::F64(f64::NAN),
            PropValueRef::Str("text"),
        ];
        let json = Value::Array(values.into_iter().map(Value::from).collect());
        assert_snapshot!(
            json,
            @r#"[true,-8,8,-2147483648,4294967295,-9223372036854775808,18446744073709551615,0.5,"f64::NAN","text"]"#
        );
    }

    fn collection_with(value: &str) -> FeatureCollection {
        FeatureCollection::from_str(&format!(
            r#"{{"type":"FeatureCollection","features":[{{"type":"Feature","properties":{{"a":{value}}},"geometry":{{"type":"Point","coordinates":[0,0]}}}}]}}"#
        ))
        .expect("parse")
    }

    #[rstest]
    #[case::identical("3.14", "3.14", true)]
    #[case::f32_round_trip("3.14", "3.140000104904175", true)]
    #[case::denormal_against_zero("1e-41", "0.0", true)]
    #[case::denormal_against_integer_zero("1e-41", "0", true)]
    #[case::largest_f32("3.4028235e38", "3.4028234663852886e38", true)]
    #[case::beyond_f32_tolerance("1.0", "1.001", false)]
    #[case::different_integers("1", "2", false)]
    #[case::nested_arrays("[1.0, [2.0]]", "[1.0, [2.0]]", true)]
    #[case::shorter_array("[1.0, 2.0]", "[1.0]", false)]
    #[case::nan_name(r#""f64::NAN""#, r#""f64::NAN""#, true)]
    #[case::number_against_string("1.0", r#""1.0""#, false)]
    fn equals_compares_with_float_tolerance(
        #[case] left: &str,
        #[case] right: &str,
        #[case] expected: bool,
    ) {
        let (left, right) = (collection_with(left), collection_with(right));
        assert_eq!(left.equals(&right).expect("compare"), expected);
        assert_eq!(right.equals(&left).expect("compare"), expected);
    }

    #[test]
    fn equals_is_false_for_a_missing_property() {
        let with_extra = FeatureCollection::from_str(
            r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{"a":1.0,"b":2.0},"geometry":{"type":"Point","coordinates":[0,0]}}]}"#,
        )
        .expect("parse");
        assert!(!collection_with("1.0").equals(&with_extra).expect("compare"));
        assert!(!with_extra.equals(&collection_with("1.0")).expect("compare"));
    }
}
