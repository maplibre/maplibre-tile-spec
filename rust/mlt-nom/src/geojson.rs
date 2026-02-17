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
    pub geometry: Geometry,
    pub id: u64,
    pub properties: BTreeMap<String, Value>,
    #[serde(rename = "type")]
    pub ty: String,
}

/// `[lat, lon]` or `[east, north]`
pub type Coordinate = [i32; 2];

/// `GeoJSON` [`Geometry`] with i32 coordinates
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "coordinates")]
pub enum Geometry {
    Point(Coordinate),
    LineString(Vec<Coordinate>),
    Polygon(Vec<Vec<Coordinate>>),
    MultiPoint(Vec<Coordinate>),
    MultiLineString(Vec<Vec<Coordinate>>),
    MultiPolygon(Vec<Vec<Vec<Coordinate>>>),
}

impl Geometry {
    #[must_use]
    pub fn point(coordinates: Coordinate) -> Self {
        Self::Point(coordinates)
    }

    #[must_use]
    pub fn line_string(coordinates: Vec<Coordinate>) -> Self {
        Self::LineString(coordinates)
    }

    #[must_use]
    pub fn polygon(coordinates: Vec<Vec<Coordinate>>) -> Self {
        Self::Polygon(coordinates)
    }

    #[must_use]
    pub fn multi_point(coordinates: Vec<Coordinate>) -> Self {
        Self::MultiPoint(coordinates)
    }

    #[must_use]
    pub fn multi_line_string(coordinates: Vec<Vec<Coordinate>>) -> Self {
        Self::MultiLineString(coordinates)
    }

    #[must_use]
    pub fn multi_polygon(coordinates: Vec<Vec<Vec<Coordinate>>>) -> Self {
        Self::MultiPolygon(coordinates)
    }
}
