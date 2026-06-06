//! Encode a GeoJSON `FeatureCollection` into MLT bytes.
//!
//! Input is an RFC 7946 `FeatureCollection`.
//! Geometry is in tile-local coordinate space (no projection), mirroring `mapbox_vector_tile`'s default.
//! Coordinates must be integer-valued and 2D.

use mlt_core::encoder::EncoderConfig;
use mlt_core::geo_types::{
    Coord, Geometry, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
};
use mlt_core::{PropValue, TileFeature, TileLayer};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyBytes, PyDict, PyInt, PyList};
use pyo3_stub_gen::derive::gen_stub_pyfunction;

fn val_err(msg: impl Into<String>) -> PyErr {
    PyValueError::new_err(msg.into())
}

fn f64_to_tile_i32(f: f64) -> PyResult<i32> {
    if f.fract() != 0.0 {
        return Err(val_err(format!("non-integer coordinate: {f}")));
    }
    if f < f64::from(i32::MIN) || f > f64::from(i32::MAX) {
        return Err(val_err(format!("coordinate {f} out of i32 range")));
    }
    #[expect(clippy::cast_possible_truncation, reason = "range checked above")]
    Ok(f as i32)
}

fn coord_ordinate(obj: &Bound<'_, PyAny>) -> PyResult<i32> {
    if let Ok(i) = obj.extract::<i64>() {
        return i32::try_from(i).map_err(|_| val_err(format!("coordinate {i} out of i32 range")));
    }
    if let Ok(f) = obj.extract::<f64>() {
        return f64_to_tile_i32(f);
    }
    Err(val_err("coordinate ordinate must be a number"))
}

fn parse_position(obj: &Bound<'_, PyAny>) -> PyResult<Coord<i32>> {
    let len = obj
        .len()
        .map_err(|_| val_err("coordinate must be a sequence"))?;
    if len > 2 {
        return Err(val_err("3D coordinates are not supported"));
    }
    if len < 2 {
        return Err(val_err("coordinate must have x and y"));
    }
    Ok(Coord {
        x: coord_ordinate(&obj.get_item(0)?)?,
        y: coord_ordinate(&obj.get_item(1)?)?,
    })
}

fn parse_positions(obj: &Bound<'_, PyAny>) -> PyResult<Vec<Coord<i32>>> {
    let len = obj
        .len()
        .map_err(|_| val_err("expected a list of coordinates"))?;
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        out.push(parse_position(&obj.get_item(i)?)?);
    }
    Ok(out)
}

fn parse_line_string(obj: &Bound<'_, PyAny>) -> PyResult<LineString<i32>> {
    Ok(LineString(parse_positions(obj)?))
}

fn parse_polygon(obj: &Bound<'_, PyAny>) -> PyResult<Polygon<i32>> {
    let len = obj
        .len()
        .map_err(|_| val_err("polygon must be a list of rings"))?;
    let mut rings = Vec::with_capacity(len);
    for i in 0..len {
        rings.push(parse_line_string(&obj.get_item(i)?)?);
    }
    let mut it = rings.into_iter();
    let exterior = it.next().unwrap_or_else(|| LineString(Vec::new()));
    Ok(Polygon::new(exterior, it.collect()))
}

fn map_seq<T>(
    obj: &Bound<'_, PyAny>,
    f: impl Fn(&Bound<'_, PyAny>) -> PyResult<T>,
) -> PyResult<Vec<T>> {
    let len = obj
        .len()
        .map_err(|_| val_err("expected a coordinate sequence"))?;
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        out.push(f(&obj.get_item(i)?)?);
    }
    Ok(out)
}

fn parse_geojson_geometry(geom: &Bound<'_, PyDict>) -> PyResult<Geometry<i32>> {
    let ty: String = geom
        .get_item("type")?
        .ok_or_else(|| val_err("geometry missing 'type'"))?
        .extract()?;
    let coords = geom
        .get_item("coordinates")?
        .ok_or_else(|| val_err("geometry missing 'coordinates'"))?;
    Ok(match ty.as_str() {
        "Point" => Geometry::Point(Point(parse_position(&coords)?)),
        "MultiPoint" => Geometry::MultiPoint(MultiPoint(
            parse_positions(&coords)?.into_iter().map(Point).collect(),
        )),
        "LineString" => Geometry::LineString(parse_line_string(&coords)?),
        "MultiLineString" => {
            Geometry::MultiLineString(MultiLineString(map_seq(&coords, parse_line_string)?))
        }
        "Polygon" => Geometry::Polygon(parse_polygon(&coords)?),
        "MultiPolygon" => Geometry::MultiPolygon(MultiPolygon(map_seq(&coords, parse_polygon)?)),
        other => return Err(val_err(format!("unsupported geometry type: {other}"))),
    })
}

fn validate_non_empty(g: &Geometry<i32>) -> PyResult<()> {
    let non_empty = match g {
        Geometry::Point(_) => true,
        Geometry::MultiPoint(mp) => !mp.0.is_empty(),
        Geometry::LineString(l) => !l.0.is_empty(),
        Geometry::MultiLineString(ml) => !ml.0.is_empty() && ml.0.iter().all(|l| !l.0.is_empty()),
        Geometry::Polygon(p) => !p.exterior().0.is_empty(),
        Geometry::MultiPolygon(mp) => {
            !mp.0.is_empty() && mp.0.iter().all(|p| !p.exterior().0.is_empty())
        }
        _ => false,
    };
    if non_empty {
        Ok(())
    } else {
        Err(val_err("empty geometry is not supported"))
    }
}

fn parse_geometry(obj: &Bound<'_, PyAny>) -> PyResult<Geometry<i32>> {
    let dict = obj
        .cast::<PyDict>()
        .map_err(|_| val_err("geometry must be a GeoJSON dict"))?;
    let geometry = parse_geojson_geometry(dict)?;
    validate_non_empty(&geometry)?;
    Ok(geometry)
}

enum PyScalar {
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
    Str(String),
}

impl PyScalar {
    fn stringify(&self) -> String {
        match self {
            Self::Bool(b) => b.to_string(),
            Self::I64(i) => i.to_string(),
            Self::U64(u) => u.to_string(),
            Self::F64(f) => f.to_string(),
            Self::Str(s) => s.clone(),
        }
    }
}

/// `Ok(None)` is a Python `None` (typed null).
/// Nested or unsupported values error.
fn extract_scalar(key: &str, obj: &Bound<'_, PyAny>) -> PyResult<Option<PyScalar>> {
    if obj.is_none() {
        return Ok(None);
    }
    // bool is a subclass of int in Python, so it must be checked first.
    if let Ok(b) = obj.cast::<PyBool>() {
        return Ok(Some(PyScalar::Bool(b.is_true())));
    }
    if obj.cast::<PyInt>().is_ok() {
        if let Ok(i) = obj.extract::<i64>() {
            return Ok(Some(PyScalar::I64(i)));
        }
        if let Ok(u) = obj.extract::<u64>() {
            return Ok(Some(PyScalar::U64(u)));
        }
        return Err(val_err(format!(
            "property '{key}' integer is out of u64 range"
        )));
    }
    if let Ok(f) = obj.extract::<f64>() {
        return Ok(Some(PyScalar::F64(f)));
    }
    if let Ok(s) = obj.extract::<String>() {
        return Ok(Some(PyScalar::Str(s)));
    }
    Err(val_err(format!(
        "property '{key}' has unsupported type '{}'; MLT columns must be bool/int/float/str",
        obj.get_type().name()?
    )))
}

/// Per-column type, with `mlt_core`'s MVT-importer widening rules.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ColKind {
    Unknown,
    Bool,
    I64,
    U64,
    F64,
    Str,
}

impl ColKind {
    fn of(s: &PyScalar) -> Self {
        match s {
            PyScalar::Bool(_) => Self::Bool,
            PyScalar::I64(_) => Self::I64,
            PyScalar::U64(_) => Self::U64,
            PyScalar::F64(_) => Self::F64,
            PyScalar::Str(_) => Self::Str,
        }
    }

    fn merge(self, other: Self) -> Self {
        if self == Self::Unknown {
            return other;
        }
        if other == Self::Unknown || self == other {
            return self;
        }
        match (self, other) {
            (Self::I64, Self::U64) | (Self::U64, Self::I64) => Self::I64,
            _ => Self::Str,
        }
    }

    fn typed_null(self) -> PropValue {
        match self {
            Self::Unknown | Self::Str => PropValue::Str(None),
            Self::Bool => PropValue::Bool(None),
            Self::I64 => PropValue::I64(None),
            Self::U64 => PropValue::U64(None),
            Self::F64 => PropValue::F64(None),
        }
    }

    fn convert(self, s: PyScalar) -> PropValue {
        match (self, s) {
            (Self::Bool, PyScalar::Bool(b)) => PropValue::Bool(Some(b)),
            (Self::I64, PyScalar::I64(i)) => PropValue::I64(Some(i)),
            (Self::I64, PyScalar::U64(u)) if i64::try_from(u).is_ok() =>
            {
                #[expect(clippy::cast_possible_wrap, reason = "checked above")]
                PropValue::I64(Some(u as i64))
            }
            (Self::U64, PyScalar::U64(u)) => PropValue::U64(Some(u)),
            (Self::F64, PyScalar::F64(f)) => PropValue::F64(Some(f)),
            (Self::Str, PyScalar::Str(s)) => PropValue::Str(Some(s)),
            (_, s) => PropValue::Str(Some(s.stringify())),
        }
    }
}

struct RawFeature {
    id: Option<u64>,
    geometry: Geometry<i32>,
    props: Vec<(String, Option<PyScalar>)>,
}

fn parse_id(feat: &Bound<'_, PyDict>) -> PyResult<Option<u64>> {
    let Some(v) = feat.get_item("id")? else {
        return Ok(None);
    };
    if v.is_none() {
        return Ok(None);
    }
    if v.cast::<PyBool>().is_ok() {
        return Err(val_err("feature 'id' must be an integer, not bool"));
    }
    if v.cast::<PyInt>().is_ok() {
        return v
            .extract::<u64>()
            .map(Some)
            .map_err(|_| val_err("feature 'id' must be a non-negative integer <= u64::MAX"));
    }
    Err(val_err("feature 'id' must be a non-negative integer"))
}

fn parse_raw_feature(feat: &Bound<'_, PyAny>) -> PyResult<RawFeature> {
    let feat = feat
        .cast::<PyDict>()
        .map_err(|_| val_err("feature must be a GeoJSON Feature object"))?;
    let ty: Option<String> = feat.get_item("type")?.and_then(|t| t.extract().ok());
    if ty.as_deref() != Some("Feature") {
        return Err(val_err(
            "feature must be a GeoJSON Feature (\"type\": \"Feature\")",
        ));
    }
    let id = parse_id(feat)?;
    let geom_obj = feat
        .get_item("geometry")?
        .ok_or_else(|| val_err("feature missing 'geometry'"))?;
    if geom_obj.is_none() {
        return Err(val_err("feature 'geometry' must not be null"));
    }
    let geometry = parse_geometry(&geom_obj)?;

    let mut props = Vec::new();
    if let Some(p) = feat.get_item("properties")? {
        let p = p
            .cast::<PyDict>()
            .map_err(|_| val_err("'properties' must be a dict"))?;
        for (k, v) in p.iter() {
            let key: String = k
                .extract()
                .map_err(|_| val_err("property keys must be strings"))?;
            let scalar = extract_scalar(&key, &v)?;
            props.push((key, scalar));
        }
    }

    Ok(RawFeature {
        id,
        geometry,
        props,
    })
}

fn infer_columns(features: &[RawFeature]) -> (Vec<String>, Vec<ColKind>) {
    let mut names: Vec<String> = Vec::new();
    let mut index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut kinds: Vec<ColKind> = Vec::new();

    for f in features {
        for (key, val) in &f.props {
            let idx = *index.entry(key.clone()).or_insert_with(|| {
                names.push(key.clone());
                kinds.push(ColKind::Unknown);
                names.len() - 1
            });
            if let Some(s) = val {
                kinds[idx] = kinds[idx].merge(ColKind::of(s));
            }
        }
    }
    for k in &mut kinds {
        if *k == ColKind::Unknown {
            *k = ColKind::Str;
        }
    }
    (names, kinds)
}

fn build_tile_features(
    raw: Vec<RawFeature>,
    names: &[String],
    kinds: &[ColKind],
) -> Vec<TileFeature> {
    let index: std::collections::HashMap<&str, usize> = names
        .iter()
        .enumerate()
        .map(|(i, n)| (n.as_str(), i))
        .collect();

    raw.into_iter()
        .map(|f| {
            let mut properties: Vec<PropValue> = kinds.iter().map(|k| k.typed_null()).collect();
            for (key, val) in f.props {
                if let (Some(s), Some(&idx)) = (val, index.get(key.as_str())) {
                    properties[idx] = kinds[idx].convert(s);
                }
            }
            TileFeature {
                id: f.id,
                geometry: f.geometry,
                properties,
            }
        })
        .collect()
}

/// Encode a GeoJSON `FeatureCollection` into MLT bytes.
///
/// `geojson` is an RFC 7946 `FeatureCollection`.
/// `name` and `extent` set the MLT layer metadata, since a `FeatureCollection` has no slot for them.
/// Geometry is in tile-local coordinate space (no projection).
/// See the module docs.
#[gen_stub_pyfunction]
#[pyfunction]
#[pyo3(signature = (geojson, name, extent=4096))]
pub fn encode(
    py: Python<'_>,
    #[gen_stub(override_type(type_repr = "typing.Mapping[builtins.str, builtins.object]"))]
    geojson: &Bound<'_, PyAny>,
    name: String,
    extent: u32,
) -> PyResult<Py<PyBytes>> {
    if name.is_empty() {
        return Err(val_err("'name' must be non-empty"));
    }

    let fc = geojson
        .cast::<PyDict>()
        .map_err(|_| val_err("input must be a GeoJSON FeatureCollection"))?;
    let ty: Option<String> = fc.get_item("type")?.and_then(|t| t.extract().ok());
    if ty.as_deref() != Some("FeatureCollection") {
        return Err(val_err(
            "input must be a GeoJSON FeatureCollection (\"type\": \"FeatureCollection\")",
        ));
    }

    let features_obj = fc
        .get_item("features")?
        .ok_or_else(|| val_err("FeatureCollection missing 'features'"))?;
    let features_list = features_obj
        .cast::<PyList>()
        .map_err(|_| val_err("'features' must be a list"))?;

    let mut raw = Vec::with_capacity(features_list.len());
    for feat in features_list.iter() {
        raw.push(parse_raw_feature(&feat)?);
    }
    if raw.is_empty() {
        return Err(val_err("FeatureCollection has no features"));
    }

    let (property_names, kinds) = infer_columns(&raw);
    let features = build_tile_features(raw, &property_names, &kinds);

    let tile = TileLayer {
        name,
        extent,
        property_names,
        features,
    };
    let bytes = tile
        .encode(EncoderConfig::default())
        .map_err(|e| val_err(format!("MLT encode error: {e}")))?;
    Ok(PyBytes::new(py, &bytes).unbind())
}
