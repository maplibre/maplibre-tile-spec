//! Convert MVT data to [`FeatureCollection`] or to [`TileLayer01`]

use std::collections::{BTreeMap, HashMap};

use geo_types::{
    Coord, Geometry as Geom, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
};
use mvt_reader::Reader;
use mvt_reader::feature::{Feature as MvtFeature, Value as MvtValue};
use serde_json::{Number, Value};

use crate::decoder::{PropValue, TileFeature, TileLayer01};
use crate::geojson::{Feature, FeatureCollection};
use crate::{Coord32, Geom32, MltError, MltResult};

// ── Common MVT parsing ────────────────────────────────────────────────────────

/// Parsed representation of a single MVT layer: metadata plus raw features.
struct MvtLayer {
    name: String,
    extent: u32,
    features: Vec<MvtFeature<f32>>,
}

/// Parse MVT bytes into a list of layers, each holding its raw features.
///
/// This is the single place where the `mvt_reader` API is called; both
/// [`mvt_to_feature_collection`] and [`mvt_to_tile_layers`] build on top of it.
fn read_mvt_layers(data: Vec<u8>) -> MltResult<Vec<MvtLayer>> {
    let reader = Reader::new(data).map_err(|e| MltError::MvtParse(e.to_string()))?;
    let metas = reader
        .get_layer_metadata()
        .map_err(|e| MltError::MvtParse(e.to_string()))?;
    metas
        .iter()
        .map(|meta| {
            let features = reader
                .get_features(meta.layer_index)
                .map_err(|e| MltError::MvtParse(e.to_string()))?;
            Ok(MvtLayer {
                name: meta.name.clone(),
                extent: meta.extent,
                features,
            })
        })
        .collect()
}

/// Parse MVT binary data and convert to a [`FeatureCollection`].
pub fn mvt_to_feature_collection(data: Vec<u8>) -> MltResult<FeatureCollection> {
    let mut features = Vec::new();

    for layer in read_mvt_layers(data)? {
        for feat in layer.features {
            let geometry = convert_geometry(&feat.geometry)?;
            let mut properties = feat
                .properties
                .map(|p| {
                    p.into_iter()
                        .map(|(k, v)| (k, convert_value(&v)))
                        .collect::<BTreeMap<_, _>>()
                })
                .unwrap_or_default();
            properties.insert("_layer".into(), Value::String(layer.name.clone()));
            properties.insert("_extent".into(), Value::Number(layer.extent.into()));
            features.push(Feature {
                geometry,
                id: feat.id,
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

/// Parse MVT binary data and convert each layer to a row-oriented [`TileLayer01`].
///
/// Each MVT layer becomes one [`TileLayer01`].  Property column types are inferred
/// from all features in the layer: the first non-null value seen for each column
/// determines its type, with `I64`+`U64` widened to `I64` and `F32`+`F64` widened
/// to `F64`; all other type conflicts fall back to `Str`.
pub fn mvt_to_tile_layers(data: Vec<u8>) -> MltResult<Vec<TileLayer01>> {
    read_mvt_layers(data)?
        .into_iter()
        .map(mvt_layer_to_tile)
        .collect()
}

fn mvt_layer_to_tile(layer: MvtLayer) -> MltResult<TileLayer01> {
    // First pass: collect property names (insertion-ordered) and infer column types.
    let mut col_names: Vec<String> = Vec::new();
    let mut col_index: HashMap<String, usize> = HashMap::new();
    let mut col_types: Vec<InferredType> = Vec::new();

    for feat in &layer.features {
        let Some(props) = &feat.properties else {
            continue;
        };
        for (key, val) in props {
            let idx = *col_index.entry(key.clone()).or_insert_with(|| {
                let i = col_names.len();
                col_names.push(key.clone());
                col_types.push(InferredType::Unknown);
                i
            });
            col_types[idx] = col_types[idx].merge(InferredType::from_mvt(val));
        }
    }

    // Columns that were only ever null fall back to Str.
    for t in &mut col_types {
        if *t == InferredType::Unknown {
            *t = InferredType::Str;
        }
    }

    // Second pass: build TileFeature objects.
    let mut tile_features = Vec::with_capacity(layer.features.len());
    for feat in layer.features {
        let geometry = convert_geometry(&feat.geometry)?;
        // Start every slot with a typed null; fill in present values below.
        let mut properties: Vec<PropValue> = col_types.iter().map(|t| t.typed_null()).collect();
        if let Some(props) = feat.properties {
            for (key, val) in props {
                if let Some(&idx) = col_index.get(&key)
                    && !matches!(val, MvtValue::Null)
                {
                    properties[idx] = col_types[idx].convert(val);
                }
            }
        }
        tile_features.push(TileFeature {
            id: feat.id,
            geometry,
            properties,
        });
    }

    Ok(TileLayer01 {
        name: layer.name,
        extent: layer.extent,
        property_names: col_names,
        features: tile_features,
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

fn convert_geometry(geom: &Geom<f32>) -> MltResult<Geom32> {
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

/// Column type inferred from MVT property values across all features in a layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InferredType {
    Unknown,
    Bool,
    I64,
    U64,
    F32,
    F64,
    Str,
}

impl InferredType {
    fn from_mvt(val: &MvtValue) -> Self {
        match val {
            MvtValue::Bool(_) => Self::Bool,
            MvtValue::Int(_) | MvtValue::SInt(_) => Self::I64,
            MvtValue::UInt(_) => Self::U64,
            MvtValue::Float(_) => Self::F32,
            MvtValue::Double(_) => Self::F64,
            MvtValue::String(_) => Self::Str,
            MvtValue::Null => Self::Unknown,
        }
    }

    /// Merge with another type, widening when necessary.
    fn merge(self, other: Self) -> Self {
        if self == Self::Unknown {
            return other;
        }
        if other == Self::Unknown || self == other {
            return self;
        }
        if matches!(
            (self, other),
            (Self::I64, Self::U64) | (Self::U64, Self::I64)
        ) {
            return Self::I64;
        }
        if matches!(
            (self, other),
            (Self::F32, Self::F64) | (Self::F64, Self::F32)
        ) {
            return Self::F64;
        }
        Self::Str
    }

    fn typed_null(self) -> PropValue {
        match self {
            Self::Unknown | Self::Str => PropValue::Str(None),
            Self::Bool => PropValue::Bool(None),
            Self::I64 => PropValue::I64(None),
            Self::U64 => PropValue::U64(None),
            Self::F32 => PropValue::F32(None),
            Self::F64 => PropValue::F64(None),
        }
    }

    /// Convert an owned [`MvtValue`] into a [`PropValue`] matching this column type.
    fn convert(self, val: MvtValue) -> PropValue {
        match (self, val) {
            (_, MvtValue::Null) => self.typed_null(),
            (Self::Bool, MvtValue::Bool(b)) => PropValue::Bool(Some(b)),
            (Self::I64, MvtValue::Int(i) | MvtValue::SInt(i)) => PropValue::I64(Some(i)),
            (Self::I64, MvtValue::UInt(u)) if i64::try_from(u).is_ok() => {
                // Value must be within 0..i64::MAX
                #[expect(clippy::cast_possible_wrap, reason = "checked above")]
                PropValue::I64(Some(u as i64))
            }
            (Self::U64, MvtValue::UInt(u)) => PropValue::U64(Some(u)),
            (Self::F32, MvtValue::Float(f)) => PropValue::F32(Some(f)),
            (Self::F64, MvtValue::Double(f)) => PropValue::F64(Some(f)),
            (Self::F64, MvtValue::Float(f)) => PropValue::F64(Some(f64::from(f))),
            (_, MvtValue::String(s)) => PropValue::Str(Some(s)),
            // Type conflict at runtime: fall back to a debug string.
            (_, v) => PropValue::Str(Some(format!("{v:?}"))),
        }
    }
}
