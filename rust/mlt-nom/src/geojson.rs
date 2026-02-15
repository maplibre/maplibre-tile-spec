//! `GeoJSON` -like data to represent decoded MLT data with i32 coordinates

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::MltError;
use crate::layer::Layer;
use crate::v01::{DecodedId, DecodedProperty, Geometry as MltGeometry, Id, Property};

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
                .ok_or_else(|| MltError::DecodeError("expected Tag01 layer".into()))?;
            let geom = match &l.geometry {
                MltGeometry::Decoded(g) => g,
                MltGeometry::Raw(_) => {
                    return Err(MltError::DecodeError("geometry not decoded".into()));
                }
            };
            let ids = match &l.id {
                Id::Decoded(DecodedId(Some(v))) => Some(v.as_slice()),
                Id::Decoded(DecodedId(None)) | Id::None => None,
                Id::Raw(_) => return Err(MltError::DecodeError("id not decoded".into())),
            };
            let props: Vec<&DecodedProperty> = l
                .properties
                .iter()
                .map(|p| match p {
                    Property::Decoded(d) => Ok(d),
                    Property::Raw(_) => Err(MltError::DecodeError("property not decoded".into())),
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
                properties.insert("layer".into(), Value::String(l.name.to_string()));
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
    pub geometry: Geometry,
    pub id: u64,
    pub properties: BTreeMap<String, Value>,
    #[serde(rename = "type")]
    pub ty: String,
}

/// `GeoJSON` [`Geometry`] with i32 coordinates
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Geometry {
    Point {
        coordinates: [i32; 2],
        #[serde(default)]
        crs: Crs,
    },
    LineString {
        coordinates: Vec<[i32; 2]>,
        #[serde(default)]
        crs: Crs,
    },
    Polygon {
        coordinates: Vec<Vec<[i32; 2]>>,
        #[serde(default)]
        crs: Crs,
    },
    MultiPoint {
        coordinates: Vec<[i32; 2]>,
        #[serde(default)]
        crs: Crs,
    },
    MultiLineString {
        coordinates: Vec<Vec<[i32; 2]>>,
        #[serde(default)]
        crs: Crs,
    },
    MultiPolygon {
        coordinates: Vec<Vec<Vec<[i32; 2]>>>,
        #[serde(default)]
        crs: Crs,
    },
}

impl Geometry {
    #[must_use]
    pub fn point(coordinates: [i32; 2]) -> Self {
        Self::Point {
            coordinates,
            crs: Crs,
        }
    }

    #[must_use]
    pub fn line_string(coordinates: Vec<[i32; 2]>) -> Self {
        Self::LineString {
            coordinates,
            crs: Crs,
        }
    }

    #[must_use]
    pub fn polygon(coordinates: Vec<Vec<[i32; 2]>>) -> Self {
        Self::Polygon {
            coordinates,
            crs: Crs,
        }
    }

    #[must_use]
    pub fn multi_point(coordinates: Vec<[i32; 2]>) -> Self {
        Self::MultiPoint {
            coordinates,
            crs: Crs,
        }
    }

    #[must_use]
    pub fn multi_line_string(coordinates: Vec<Vec<[i32; 2]>>) -> Self {
        Self::MultiLineString {
            coordinates,
            crs: Crs,
        }
    }

    #[must_use]
    pub fn multi_polygon(coordinates: Vec<Vec<Vec<[i32; 2]>>>) -> Self {
        Self::MultiPolygon {
            coordinates,
            crs: Crs,
        }
    }
}

/// Constant CRS — serializes as `{"type":"name","properties":{"name":"EPSG:0"}}`,
/// ignores any value when deserializing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Crs;

impl Serialize for Crs {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Repr {
            #[serde(rename = "type")]
            ty: &'static str,
            properties: Props,
        }
        #[derive(Serialize)]
        struct Props {
            name: &'static str,
        }
        Repr {
            ty: "name",
            properties: Props { name: "EPSG:0" },
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Crs {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        serde::de::IgnoredAny::deserialize(deserializer)?;
        Ok(Self)
    }
}
