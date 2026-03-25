//! `GeoJSON` -like data to represent decoded MLT data with i32 coordinates

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::frames::Layer;
use crate::{MltResult, ParsedLayer};

/// `GeoJSON` geometry with `i32` tile coordinates
pub type Geom32 = geo_types::Geometry<i32>;

/// A single `i32` coordinate (x, y)
pub type Coord32 = geo_types::Coord<i32>;

/// `GeoJSON` geometry with `i16` tile coordinates
pub type Geom16 = geo_types::Geometry<i16>;

/// A single `i16` coordinate (x, y)
pub type Coord16 = geo_types::Coord<i16>;

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
                for col in feat.iter_properties() {
                    properties.insert(col.name.to_string(), col.value.into());
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
        use serde::ser::SerializeMap as _;
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
    use serde::{Deserialize, Deserializer, Serializer};
    use serde_json::Value;

    use crate::geojson::Geom32;

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
        use serde::ser::{Error, SerializeMap as _};

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
        use serde::de::Error as _;

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
