//! Convert MVT data to [`FeatureCollection`]

use std::collections::BTreeMap;

use geo_types as gt;
use geo_types::Coord;
use mvt_reader::Reader;
use mvt_reader::feature::Value as MvtValue;
use serde_json::{Number, Value};

use crate::MltError;
use crate::geojson::{Coordinate, Feature, FeatureCollection, Geometry};

/// Parse MVT binary data and convert to a [`FeatureCollection`].
pub fn mvt_to_feature_collection(data: Vec<u8>) -> Result<FeatureCollection, MltError> {
    let reader = Reader::new(data).map_err(|e| MltError::MvtParse(e.to_string()))?;
    let layers = reader
        .get_layer_metadata()
        .map_err(|e| MltError::MvtParse(e.to_string()))?;
    let mut features = Vec::new();

    for layer in &layers {
        let mvt_features = reader
            .get_features(layer.layer_index)
            .map_err(|e| MltError::MvtParse(e.to_string()))?;
        for mvt_feat in mvt_features {
            let geometry = convert_geometry(&mvt_feat.geometry)?;
            let id = mvt_feat.id.unwrap_or(0);
            let mut properties = mvt_feat
                .properties
                .as_ref()
                .map(|p| {
                    p.iter()
                        .map(|(k, v)| (k.clone(), convert_value(v)))
                        .collect::<BTreeMap<_, _>>()
                })
                .unwrap_or_default();
            properties.insert("_layer".into(), Value::String(layer.name.clone()));
            properties.insert("_extent".into(), Value::Number(layer.extent.into()));
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

fn coord(coord: impl AsRef<Coord<f32>>) -> Coordinate {
    let c = coord.as_ref();
    #[expect(clippy::cast_possible_truncation)]
    [c.x.round() as i32, c.y.round() as i32]
}

fn convert_geometry(geom: &gt::Geometry<f32>) -> Result<Geometry, MltError> {
    Ok(match geom {
        gt::Geometry::Point(v) => Geometry::Point(coord(v)),
        gt::Geometry::MultiPoint(v) => Geometry::MultiPoint(v.iter().map(coord).collect()),
        gt::Geometry::LineString(v) => Geometry::LineString(v.coords().map(coord).collect()),
        gt::Geometry::MultiLineString(v) => Geometry::MultiLineString(
            v.iter()
                .map(|vv| vv.coords().map(coord).collect())
                .collect(),
        ),
        gt::Geometry::Polygon(v) => Geometry::Polygon(convert_polygon(v)),
        gt::Geometry::MultiPolygon(v) => {
            Geometry::MultiPolygon(v.iter().map(convert_polygon).collect())
        }
        gt::Geometry::GeometryCollection(v) => {
            return if v.len() == 1 {
                convert_geometry(&v[0])
            } else {
                Err(MltError::BadMvtGeometry(
                    "multiple geometries in a collection are not supported",
                ))
            };
        }
        gt::Geometry::Line(_) => Err(MltError::BadMvtGeometry("Unsupported Line geo type"))?,
        gt::Geometry::Rect(_) => Err(MltError::BadMvtGeometry("Unsupported Rect geo type"))?,
        gt::Geometry::Triangle(_) => {
            Err(MltError::BadMvtGeometry("Unsupported Triangle geo type"))?
        }
    })
}

fn convert_polygon(poly: &gt::Polygon<f32>) -> Vec<Vec<Coordinate>> {
    let mut rings = Vec::with_capacity(1 + poly.interiors().len());
    rings.push(poly.exterior().coords().map(coord).collect());
    for interior in poly.interiors() {
        rings.push(interior.coords().map(coord).collect());
    }
    rings
}

fn convert_value(val: &MvtValue) -> Value {
    match val {
        MvtValue::String(s) => Value::String(s.clone()),
        MvtValue::Float(f) => Number::from_f64(f64::from(*f)).map_or(Value::Null, Value::Number),
        MvtValue::Double(f) => Number::from_f64(*f).map_or(Value::Null, Value::Number),
        MvtValue::Int(i) | MvtValue::SInt(i) => Value::Number((*i).into()),
        MvtValue::UInt(u) => Value::Number((*u).into()),
        MvtValue::Bool(b) => Value::Bool(*b),
        MvtValue::Null => Value::Null,
    }
}
