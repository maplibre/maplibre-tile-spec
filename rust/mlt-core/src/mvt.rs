//! Convert MVT data to [`FeatureCollection`]

use std::collections::BTreeMap;

use mvt_reader::Reader;
use serde_json::Value;

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

fn convert_geometry(geom: &geo_types::Geometry<f32>) -> Geometry {
    match geom {
        geo_types::Geometry::Point(p) => Geometry::Point(coord(p.x(), p.y())),
        geo_types::Geometry::MultiPoint(mp) => {
            Geometry::MultiPoint(mp.iter().map(|p| coord(p.x(), p.y())).collect())
        }
        geo_types::Geometry::LineString(ls) => {
            Geometry::LineString(ls.coords().map(|c| coord(c.x, c.y)).collect())
        }
        geo_types::Geometry::MultiLineString(mls) => Geometry::MultiLineString(
            mls.iter()
                .map(|ls| ls.coords().map(|c| coord(c.x, c.y)).collect())
                .collect(),
        ),
        geo_types::Geometry::Polygon(poly) => Geometry::Polygon(convert_polygon(poly)),
        geo_types::Geometry::MultiPolygon(mp) => {
            Geometry::MultiPolygon(mp.iter().map(convert_polygon).collect())
        }
        geo_types::Geometry::GeometryCollection(gc) => {
            if gc.len() == 1 {
                convert_geometry(&gc[0])
            } else {
                Geometry::Point([0, 0])
            }
        }
        _ => Geometry::Point([0, 0]),
    }
}

fn convert_polygon(poly: &geo_types::Polygon<f32>) -> Vec<Vec<Coordinate>> {
    let mut rings = Vec::with_capacity(1 + poly.interiors().len());
    rings.push(poly.exterior().coords().map(|c| coord(c.x, c.y)).collect());
    for interior in poly.interiors() {
        rings.push(interior.coords().map(|c| coord(c.x, c.y)).collect());
    }
    rings
}

fn convert_value(val: &mvt_reader::feature::Value) -> Value {
    match val {
        mvt_reader::feature::Value::String(s) => Value::String(s.clone()),
        mvt_reader::feature::Value::Float(f) => {
            serde_json::Number::from_f64(f64::from(*f)).map_or(Value::Null, Value::Number)
        }
        mvt_reader::feature::Value::Double(f) => {
            serde_json::Number::from_f64(*f).map_or(Value::Null, Value::Number)
        }
        mvt_reader::feature::Value::Int(i) | mvt_reader::feature::Value::SInt(i) => {
            Value::Number((*i).into())
        }
        mvt_reader::feature::Value::UInt(u) => Value::Number((*u).into()),
        mvt_reader::feature::Value::Bool(b) => Value::Bool(*b),
        mvt_reader::feature::Value::Null => Value::Null,
    }
}
