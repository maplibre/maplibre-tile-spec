//! `GeoJSON` -like data to represent decoded MLT data with i32 coordinates

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::MltError;
use crate::layer::Layer;
use crate::v01::{DecodedId, DecodedProperty, Geometry as MltGeometry, Id, Property};

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
    pub features: Vec<Feature>,
    #[serde(rename = "type")]
    pub ty: String,
}

impl FeatureCollection {
    /// Convert decoded layers to a `GeoJSON` [`FeatureCollection`]
    pub fn from_layers(layers: &[Layer<'_>]) -> Result<FeatureCollection, MltError> {
        let mut features = Vec::new();
        for layer in layers {
            let l = layer
                .as_layer01()
                .ok_or(MltError::NotDecoded("expected Tag01 layer"))?;
            let geom = match &l.geometry {
                MltGeometry::Decoded(g) => g,
                MltGeometry::Raw(_) => {
                    return Err(MltError::NotDecoded("geometry"));
                }
            };
            let ids = match &l.id {
                Id::Decoded(DecodedId(Some(v))) => Some(v.as_slice()),
                Id::Decoded(DecodedId(None)) | Id::None => None,
                Id::Raw(_) => return Err(MltError::NotDecoded("id")),
            };
            let props: Vec<&DecodedProperty> = l
                .properties
                .iter()
                .map(|p| match p {
                    Property::Decoded(d) => Ok(d),
                    Property::Raw(_) => Err(MltError::NotDecoded("property")),
                })
                .collect::<Result<_, _>>()?;

            for i in 0..geom.vector_types.len() {
                let id = ids.and_then(|v| v.get(i).copied().flatten()).unwrap_or(0);
                let geometry = geom.to_geojson(i)?;
                let mut properties = BTreeMap::new();
                for prop in &props {
                    if let Some(val) = prop.values.to_geojson(i) {
                        properties.insert(prop.name.clone(), val);
                    }
                }
                properties.insert("_layer".into(), Value::String(l.name.to_string()));
                properties.insert("_extent".into(), Value::Number(l.extent.into()));
                features.push(Feature {
                    geometry,
                    id,
                    properties,
                    ty: "Feature".into(),
                });
            }
        }
        Ok(FeatureCollection {
            features,
            ty: "FeatureCollection".into(),
        })
    }
}

/// `GeoJSON` [`Feature`]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Feature {
    #[serde(with = "geom_serde")]
    pub geometry: Geom32,
    pub id: u64,
    pub properties: BTreeMap<String, Value>,
    #[serde(rename = "type")]
    pub ty: String,
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
