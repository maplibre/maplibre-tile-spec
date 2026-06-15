//! Encode a GeoJSON `FeatureCollection` into MLT bytes.
//!
//! Input is an RFC 7946 `FeatureCollection`.
//! Geometry is in tile-local coordinate space (no projection), mirroring `mapbox_vector_tile`'s default.
//! Coordinates must be integer-valued and 2D.
//!
//! The Python mapping is deserialized once into [`mlt_core::geojson::FeatureCollection`].
//! That type parses coordinates as `[i32; 2]` arrays, so non-integer or 3D coordinates,
//! null geometry, and out-of-range / non-integer feature ids are all rejected during
//! deserialization. Emptiness and non-scalar property values are checked here.

use std::collections::HashMap;

use mlt_core::geo_types::Geometry;
use mlt_core::geojson::FeatureCollection;
use mlt_core::{PropValue, TileFeature, TileLayer};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3_stub_gen::derive::gen_stub_pyfunction;
use serde_json::Value;

use super::shared::{encoder_config, val_err};

/// Encode a GeoJSON `FeatureCollection` into MLT bytes.
///
/// `geojson` is an RFC 7946 `FeatureCollection`.
/// `name` and `extent` set the MLT layer metadata, since a `FeatureCollection` has no slot for them.
/// Geometry is in tile-local coordinate space (no projection).
///
/// `tessellate` generates triangulation data for polygons and multi-polygons.
/// `sort` chooses which feature ordering(s) the encoder trials: `all` tries all orderings, `auto` tries a subset with a good speed-size tradeoff, a named curve (`morton`/`hilbert`/`id`) tries just that one, and `none` keeps the input order.
/// `shared_dict` allows grouping strings into shared dictionaries.
/// `fsst` allows FSST string compression.
/// `fastpfor` allows FastPFOR integer compression.
/// See the module docs.
#[gen_stub_pyfunction]
#[pyfunction]
#[pyo3(signature = (geojson, name, extent=4096, *, tessellate=false, sort="auto", shared_dict=true, fsst=true, fastpfor=true))]
#[expect(
    clippy::too_many_arguments,
    reason = "argument list mirrors the intentional Python keyword-argument API"
)]
pub fn encode_geojson(
    py: Python<'_>,
    #[gen_stub(override_type(type_repr = "typing.Mapping[builtins.str, builtins.object]"))]
    geojson: &Bound<'_, PyAny>,
    name: String,
    extent: u32,
    tessellate: bool,
    #[gen_stub(override_type(
        type_repr = "typing.Literal['all', 'auto', 'morton', 'hilbert', 'id', 'none']"
    ))]
    sort: &str,
    shared_dict: bool,
    fsst: bool,
    fastpfor: bool,
) -> PyResult<Py<PyBytes>> {
    if name.is_empty() {
        return Err(val_err("'name' must be non-empty"));
    }

    let fc: FeatureCollection = pythonize::depythonize(geojson)
        .map_err(|e| val_err(format!("input must be a GeoJSON FeatureCollection: {e}")))?;
    if fc.ty != "FeatureCollection" {
        return Err(val_err(
            "input must be a GeoJSON FeatureCollection (\"type\": \"FeatureCollection\")",
        ));
    }
    if fc.features.is_empty() {
        return Err(val_err("FeatureCollection has no features"));
    }

    let tile = build_layer(fc, name, extent)?;
    let cfg = encoder_config(tessellate, sort, shared_dict, fsst, fastpfor)?;
    // Release the GIL for the pure-Rust encode. The steps above read Python input and keep it.
    let bytes = py
        .detach(|| tile.encode(cfg))
        .map_err(|e| val_err(format!("MLT encode error: {e}")))?;
    Ok(PyBytes::new(py, &bytes).unbind())
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

/// Stringify a scalar JSON value (without the quoting `Value::to_string` adds to strings).
fn stringify(v: &Value) -> String {
    match v {
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
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
    /// Classify a scalar property value. `Ok(None)` is a JSON null (typed null);
    /// nested arrays/objects are rejected.
    fn of(key: &str, v: &Value) -> PyResult<Option<Self>> {
        Ok(match v {
            Value::Null => None,
            Value::Bool(_) => Some(Self::Bool),
            Value::Number(n) => Some(if n.is_i64() {
                Self::I64
            } else if n.is_u64() {
                Self::U64
            } else {
                Self::F64
            }),
            Value::String(_) => Some(Self::Str),
            Value::Array(_) | Value::Object(_) => {
                return Err(val_err(format!(
                    "property '{key}' has an unsupported nested value; MLT columns must be bool/int/float/str"
                )));
            }
        })
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

    fn convert(self, v: Value) -> PropValue {
        match (self, &v) {
            (Self::Bool, Value::Bool(b)) => PropValue::Bool(Some(*b)),
            (Self::I64, Value::Number(n)) if n.is_i64() => PropValue::I64(n.as_i64()),
            (Self::U64, Value::Number(n)) if n.is_u64() => PropValue::U64(n.as_u64()),
            (Self::F64, Value::Number(n)) => PropValue::F64(n.as_f64()),
            (Self::Str, Value::String(s)) => PropValue::Str(Some(s.clone())),
            _ => PropValue::Str(Some(stringify(&v))),
        }
    }
}

/// Validate geometries, reject nested property values, and infer one type per column.
/// Column order follows first appearance across features.
fn build_layer(fc: FeatureCollection, name: String, extent: u32) -> PyResult<TileLayer> {
    let mut names: Vec<String> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut kinds: Vec<ColKind> = Vec::new();

    for feat in &fc.features {
        if feat.ty != "Feature" {
            return Err(val_err(
                "feature must be a GeoJSON Feature (\"type\": \"Feature\")",
            ));
        }
        validate_non_empty(&feat.geometry)?;
        for (key, val) in &feat.properties {
            let kind = ColKind::of(key, val)?;
            let idx = *index.entry(key.clone()).or_insert_with(|| {
                names.push(key.clone());
                kinds.push(ColKind::Unknown);
                names.len() - 1
            });
            if let Some(k) = kind {
                kinds[idx] = kinds[idx].merge(k);
            }
        }
    }
    for k in &mut kinds {
        if *k == ColKind::Unknown {
            *k = ColKind::Str;
        }
    }

    let features = fc
        .features
        .into_iter()
        .map(|feat| {
            let mut properties: Vec<PropValue> = kinds.iter().map(|k| k.typed_null()).collect();
            for (key, val) in feat.properties {
                if val.is_null() {
                    continue;
                }
                if let Some(&idx) = index.get(&key) {
                    properties[idx] = kinds[idx].convert(val);
                }
            }
            TileFeature {
                id: feat.id,
                geometry: feat.geometry,
                properties,
            }
        })
        .collect();

    Ok(TileLayer {
        name,
        extent,
        property_names: names,
        features,
    })
}
