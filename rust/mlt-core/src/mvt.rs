//! Convert MVT data to [`FeatureCollection`]

use std::collections::BTreeMap;

use geo_types as gt;
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
            let geometry = convert_geometry(&mvt_feat.geometry);
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

fn coord(x: f32, y: f32) -> Coordinate {
    #[expect(clippy::cast_possible_truncation)]
    [x.round() as i32, y.round() as i32]
}

fn convert_geometry(geom: &gt::Geometry<f32>) -> Geometry {
    match geom {
        gt::Geometry::Point(p) => Geometry::Point(coord(p.x(), p.y())),
        gt::Geometry::MultiPoint(mp) => {
            Geometry::MultiPoint(mp.iter().map(|p| coord(p.x(), p.y())).collect())
        }
        gt::Geometry::LineString(ls) => {
            Geometry::LineString(ls.coords().map(|c| coord(c.x, c.y)).collect())
        }
        gt::Geometry::MultiLineString(mls) => Geometry::MultiLineString(
            mls.iter()
                .map(|ls| ls.coords().map(|c| coord(c.x, c.y)).collect())
                .collect(),
        ),
        gt::Geometry::Polygon(poly) => Geometry::Polygon(convert_polygon(poly)),
        gt::Geometry::MultiPolygon(mp) => {
            Geometry::MultiPolygon(mp.iter().map(convert_polygon).collect())
        }
        gt::Geometry::GeometryCollection(gc) => {
            if gc.len() == 1 {
                convert_geometry(&gc[0])
            } else {
                Geometry::Point([0, 0])
            }
        }
        _ => Geometry::Point([0, 0]),
    }
}

fn convert_polygon(poly: &gt::Polygon<f32>) -> Vec<Vec<Coordinate>> {
    let mut rings = Vec::with_capacity(1 + poly.interiors().len());
    rings.push(poly.exterior().coords().map(|c| coord(c.x, c.y)).collect());
    for interior in poly.interiors() {
        rings.push(interior.coords().map(|c| coord(c.x, c.y)).collect());
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
