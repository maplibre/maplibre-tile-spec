//! Convert MVT data to [`FeatureCollection`]

use std::collections::BTreeMap;

use geo_types::{
    Coord, Geometry as Geom, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
};
use mvt_reader::Reader;
use mvt_reader::feature::Value as MvtValue;
use serde_json::{Number, Value};

use crate::MltError;
use crate::geojson::{Coord32, Feature, FeatureCollection, Geom32};

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
                id: mvt_feat.id,
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

fn coord(c: impl AsRef<Coord<f32>>) -> Coord32 {
    let c = c.as_ref();
    #[expect(clippy::cast_possible_truncation)]
    Coord {
        x: c.x.round() as i32,
        y: c.y.round() as i32,
    }
}

fn convert_geometry(geom: &Geom<f32>) -> Result<Geom32, MltError> {
    Ok(match geom {
        Geom::Point(v) => Geom32::Point(Point(coord(v))),
        Geom::MultiPoint(v) => {
            Geom32::MultiPoint(MultiPoint(v.iter().map(|p| Point(coord(p))).collect()))
        }
        Geom::LineString(v) => Geom32::LineString(LineString(v.coords().map(coord).collect())),
        Geom::MultiLineString(v) => Geom32::MultiLineString(MultiLineString(
            v.iter()
                .map(|ls| LineString(ls.coords().map(coord).collect()))
                .collect(),
        )),
        Geom::Polygon(v) => Geom32::Polygon(convert_polygon(v)),
        Geom::MultiPolygon(v) => {
            Geom32::MultiPolygon(MultiPolygon(v.iter().map(convert_polygon).collect()))
        }
        Geom::GeometryCollection(v) => {
            return if v.len() == 1 {
                convert_geometry(&v[0])
            } else {
                Err(MltError::BadMvtGeometry(
                    "multiple geometries in a collection are not supported",
                ))
            };
        }
        Geom::Line(_) => Err(MltError::BadMvtGeometry("Unsupported Line geo type"))?,
        Geom::Rect(_) => Err(MltError::BadMvtGeometry("Unsupported Rect geo type"))?,
        Geom::Triangle(_) => Err(MltError::BadMvtGeometry("Unsupported Triangle geo type"))?,
    })
}

fn convert_polygon(poly: &Polygon<f32>) -> Polygon<i32> {
    let exterior = LineString(poly.exterior().coords().map(coord).collect());
    let interiors = poly
        .interiors()
        .iter()
        .map(|r| LineString(r.coords().map(coord).collect()))
        .collect();
    Polygon::new(exterior, interiors)
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
