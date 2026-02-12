//! `GeoJSON` -like data to represent decoded MLT data with i32 coordinates

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::MltError;
use crate::layer::Layer;
use crate::v01::{
    DecodedGeometry, DecodedId, DecodedProperty, Geometry as MltGeometry, GeometryType, Id,
    PropValue, Property,
};

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
                let geometry = build_geometry(geom, i);
                let mut properties = HashMap::new();
                for prop in &props {
                    if let Some(val) = prop_to_val(&prop.values, i) {
                        properties.insert(prop.name.clone(), val);
                    }
                }
                properties.insert("layer".into(), PropVal::Str(l.name.to_string()));
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
    pub properties: HashMap<String, PropVal>,
    #[serde(rename = "type")]
    pub ty: String,
}

/// `GeoJSON` [`Geometry`] with i32 coordinates
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Geometry {
    Point {
        coordinates: [i32; 2],
        crs: Crs,
    },
    LineString {
        coordinates: Vec<[i32; 2]>,
        crs: Crs,
    },
    Polygon {
        coordinates: Vec<Vec<[i32; 2]>>,
        crs: Crs,
    },
    MultiPolygon {
        coordinates: Vec<Vec<Vec<[i32; 2]>>>,
        crs: Crs,
    },
}

impl Geometry {
    #[must_use]
    pub fn point(coordinates: [i32; 2]) -> Self {
        Self::Point {
            coordinates,
            crs: Crs::default(),
        }
    }

    #[must_use]
    pub fn line_string(coordinates: Vec<[i32; 2]>) -> Self {
        Self::LineString {
            coordinates,
            crs: Crs::default(),
        }
    }

    #[must_use]
    pub fn polygon(coordinates: Vec<Vec<[i32; 2]>>) -> Self {
        Self::Polygon {
            coordinates,
            crs: Crs::default(),
        }
    }

    #[must_use]
    pub fn multi_polygon(coordinates: Vec<Vec<Vec<[i32; 2]>>>) -> Self {
        Self::MultiPolygon {
            coordinates,
            crs: Crs::default(),
        }
    }
}

/// Coordinate Reference System
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Crs {
    #[serde(rename = "type")]
    pub ty: String,
    pub properties: CrsProperties,
}

/// CRS properties
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrsProperties {
    pub name: String,
}

impl Default for Crs {
    fn default() -> Self {
        Self {
            ty: "name".into(),
            properties: CrsProperties {
                name: "EPSG:0".into(),
            },
        }
    }
}

/// A single JSON-compatible property value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PropVal {
    Bool(bool),
    Int(i64),
    Float(f32),
    Str(String),
}

/// Build a `GeoJSON` geometry for a single feature at index `i`.
/// Polygon and `MultiPolygon` rings are closed per `GeoJSON` spec (MLT omits the closing vertex).
fn build_geometry(geom: &DecodedGeometry, i: usize) -> Geometry {
    let verts = geom.vertices.as_deref().unwrap_or(&[]);
    let geom_type = geom.vector_types[i];
    let go = geom.geometry_offsets.as_deref();
    let po = geom.part_offsets.as_deref();
    let ro = geom.ring_offsets.as_deref();

    let v = |idx: usize| [verts[idx * 2], verts[idx * 2 + 1]];
    let line = |start: usize, end: usize| (start..end).map(&v).collect();
    let closed_ring = |start: usize, end: usize| {
        let mut coords: Vec<[i32; 2]> = (start..end).map(&v).collect();
        coords.push(v(start));
        coords
    };

    match geom_type {
        GeometryType::Point => {
            let pt = match (go, po, ro) {
                (Some(go), Some(po), Some(ro)) => v(ro[po[go[i] as usize] as usize] as usize),
                (None, Some(po), Some(ro)) => v(ro[po[i] as usize] as usize),
                (None, Some(po), None) => v(po[i] as usize),
                (None, None, None) => v(i),
                _ => unreachable!(),
            };
            Geometry::point(pt)
        }
        GeometryType::LineString => {
            let coords = match (po, ro) {
                (Some(po), Some(ro)) => {
                    let ri = po[i] as usize;
                    line(ro[ri] as usize, ro[ri + 1] as usize)
                }
                (Some(po), None) => line(po[i] as usize, po[i + 1] as usize),
                _ => unreachable!(),
            };
            Geometry::line_string(coords)
        }
        GeometryType::Polygon => {
            let (rs, re) = if let Some(go) = go {
                let pi = go[i] as usize;
                (po.unwrap()[pi] as usize, po.unwrap()[pi + 1] as usize)
            } else {
                (po.unwrap()[i] as usize, po.unwrap()[i + 1] as usize)
            };
            let ro = ro.unwrap();
            Geometry::polygon(
                (rs..re)
                    .map(|r| closed_ring(ro[r] as usize, ro[r + 1] as usize))
                    .collect(),
            )
        }
        GeometryType::MultiPolygon => {
            let go = go.unwrap();
            let po = po.unwrap();
            let ro = ro.unwrap();
            let (ps, pe) = (go[i] as usize, go[i + 1] as usize);
            Geometry::multi_polygon(
                (ps..pe)
                    .map(|p| {
                        let (rs, re) = (po[p] as usize, po[p + 1] as usize);
                        (rs..re)
                            .map(|r| closed_ring(ro[r] as usize, ro[r + 1] as usize))
                            .collect()
                    })
                    .collect(),
            )
        }
        t => todo!("geometry type {t:?}"),
    }
}

/// Convert a decoded property value at index `i` to a [`PropVal`]
#[allow(clippy::cast_possible_truncation)] // f64 stored as f32 in wire format
#[allow(clippy::cast_possible_wrap)]
fn prop_to_val(values: &PropValue, i: usize) -> Option<PropVal> {
    match values {
        PropValue::Bool(v) => v[i].map(PropVal::Bool),
        PropValue::I8(v) => v[i].map(|n| PropVal::Int(i64::from(n))),
        PropValue::U8(v) => v[i].map(|n| PropVal::Int(i64::from(n))),
        PropValue::I32(v) => v[i].map(|n| PropVal::Int(i64::from(n))),
        PropValue::U32(v) => v[i].map(|n| PropVal::Int(i64::from(n))),
        PropValue::I64(v) => v[i].map(PropVal::Int),
        PropValue::U64(v) => v[i].map(|n| PropVal::Int(n as i64)),
        PropValue::F32(v) => v[i].map(PropVal::Float),
        PropValue::F64(v) => v[i].map(|f| PropVal::Float(f as f32)),
        PropValue::Str(v) => v[i].as_ref().map(|s| PropVal::Str(s.clone())),
        PropValue::Struct => None,
    }
}
