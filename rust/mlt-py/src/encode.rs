//! Encode Python GeoJSON-like input into MLT bytes.
//!
//! Input geometry is in tile-local coordinate space (no projection), mirroring
//! `mapbox_vector_tile`'s default. Coordinates must be integer-valued and 2D.

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

fn coord_f64_to_i32(c: Coord<f64>) -> PyResult<Coord<i32>> {
    Ok(Coord {
        x: f64_to_tile_i32(c.x)?,
        y: f64_to_tile_i32(c.y)?,
    })
}

fn line_f64_to_i32(l: &LineString<f64>) -> PyResult<LineString<i32>> {
    Ok(LineString(
        l.0.iter()
            .map(|c| coord_f64_to_i32(*c))
            .collect::<PyResult<_>>()?,
    ))
}

fn poly_f64_to_i32(p: &Polygon<f64>) -> PyResult<Polygon<i32>> {
    let exterior = line_f64_to_i32(p.exterior())?;
    let interiors = p
        .interiors()
        .iter()
        .map(line_f64_to_i32)
        .collect::<PyResult<_>>()?;
    Ok(Polygon::new(exterior, interiors))
}

fn geom_f64_to_i32(g: Geometry<f64>) -> PyResult<Geometry<i32>> {
    Ok(match g {
        Geometry::Point(p) => Geometry::Point(Point(coord_f64_to_i32(p.0)?)),
        Geometry::MultiPoint(mp) => Geometry::MultiPoint(MultiPoint(
            mp.0.iter()
                .map(|p| Ok(Point(coord_f64_to_i32(p.0)?)))
                .collect::<PyResult<_>>()?,
        )),
        Geometry::LineString(l) => Geometry::LineString(line_f64_to_i32(&l)?),
        Geometry::MultiLineString(ml) => Geometry::MultiLineString(MultiLineString(
            ml.0.iter().map(line_f64_to_i32).collect::<PyResult<_>>()?,
        )),
        Geometry::Polygon(p) => Geometry::Polygon(poly_f64_to_i32(&p)?),
        Geometry::MultiPolygon(mp) => Geometry::MultiPolygon(MultiPolygon(
            mp.0.iter().map(poly_f64_to_i32).collect::<PyResult<_>>()?,
        )),
        _ => return Err(val_err("unsupported geometry type")),
    })
}

fn geom_trait_to_tile(g: &impl geo_traits::GeometryTrait<T = f64>) -> PyResult<Geometry<i32>> {
    use geo_traits::to_geo::ToGeoGeometry as _;
    use geo_traits::{Dimensions, GeometryType, MultiPointTrait as _, PointTrait as _};
    if !matches!(g.dim(), Dimensions::Xy) {
        return Err(val_err(
            "3D/measured geometry is not supported; geometry must be 2D",
        ));
    }
    // `to_geometry` panics on an empty point or a MultiPoint containing one.
    let has_empty_point = match g.as_type() {
        GeometryType::Point(p) => p.coord().is_none(),
        GeometryType::MultiPoint(mp) => mp.points().any(|p| p.coord().is_none()),
        _ => false,
    };
    if has_empty_point {
        return Err(val_err("empty geometry is not supported"));
    }
    geom_f64_to_i32(g.to_geometry())
}

fn parse_wkt_geometry(s: &str) -> PyResult<Geometry<i32>> {
    use std::str::FromStr as _;
    let w = wkt::Wkt::<f64>::from_str(s).map_err(|e| val_err(format!("invalid WKT: {e}")))?;
    geom_trait_to_tile(&w)
}

fn parse_wkb_geometry(bytes: &[u8]) -> PyResult<Geometry<i32>> {
    let wkb = wkb::reader::read_wkb(bytes).map_err(|e| val_err(format!("invalid WKB: {e}")))?;
    geom_trait_to_tile(&wkb)
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
    let geometry = if let Ok(d) = obj.cast::<PyDict>() {
        parse_geojson_geometry(d)?
    } else if let Ok(b) = obj.cast::<PyBytes>() {
        parse_wkb_geometry(b.as_bytes())?
    } else if let Ok(s) = obj.extract::<String>() {
        parse_wkt_geometry(&s)?
    } else {
        return Err(val_err(
            "geometry must be a GeoJSON dict, WKT string, or WKB bytes",
        ));
    };
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

/// `Ok(None)` is a Python `None` (typed null); nested/unsupported values error.
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
        .map_err(|_| val_err("feature must be a dict"))?;
    let id = parse_id(feat)?;
    let geometry = parse_geometry(
        &feat
            .get_item("geometry")?
            .ok_or_else(|| val_err("feature missing 'geometry'"))?,
    )?;

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

fn options_dict<'py>(options: Option<&Bound<'py, PyAny>>) -> PyResult<Option<Bound<'py, PyDict>>> {
    match options {
        None => Ok(None),
        Some(o) if o.is_none() => Ok(None),
        Some(o) => Ok(Some(
            o.cast::<PyDict>()
                .map_err(|_| val_err("options must be a dict"))?
                .clone(),
        )),
    }
}

fn opt_item<'py>(
    opts: Option<&Bound<'py, PyDict>>,
    key: &str,
) -> PyResult<Option<Bound<'py, PyAny>>> {
    match opts {
        Some(d) => d.get_item(key),
        None => Ok(None),
    }
}

/// A layer dict's own `name`/`extent` win; a `FeatureCollection` takes both from
/// options (`name` required), otherwise falling back to the 4096 default.
fn resolve_name_extent(
    dict: &Bound<'_, PyDict>,
    opts: Option<&Bound<'_, PyDict>>,
    is_fc: bool,
) -> PyResult<(String, u32)> {
    let opt_name: Option<String> = opt_item(opts, "name")?.map(|n| n.extract()).transpose()?;
    let opt_extent: Option<u32> = opt_item(opts, "extent")?.map(|e| e.extract()).transpose()?;

    let name = if is_fc {
        opt_name.ok_or_else(|| val_err("FeatureCollection input requires 'name' in options"))?
    } else {
        match dict.get_item("name")? {
            Some(n) => n.extract()?,
            None => opt_name.ok_or_else(|| val_err("layer missing 'name'"))?,
        }
    };
    if name.is_empty() {
        return Err(val_err("layer 'name' must be non-empty"));
    }

    let extent = if is_fc {
        opt_extent.unwrap_or(4096)
    } else {
        match dict.get_item("extent")? {
            Some(e) => e.extract()?,
            None => opt_extent.unwrap_or(4096),
        }
    };

    Ok((name, extent))
}

fn opt_bool(opts: Option<&Bound<'_, PyDict>>, key: &str) -> PyResult<Option<bool>> {
    match opt_item(opts, key)? {
        Some(v) => {
            Ok(Some(v.extract().map_err(|_| {
                val_err(format!("option '{key}' must be a bool"))
            })?))
        }
        None => Ok(None),
    }
}

fn build_config(opts: Option<&Bound<'_, PyDict>>) -> PyResult<EncoderConfig> {
    let mut cfg = EncoderConfig::default();

    if let Some(s) = opt_item(opts, "sort")? {
        let s: String = s
            .extract()
            .map_err(|_| val_err("option 'sort' must be a string"))?;
        let (morton, hilbert, id) = match s.as_str() {
            "auto" => (true, true, true),
            "morton" => (true, false, false),
            "hilbert" => (false, true, false),
            "id" => (false, false, true),
            "none" => (false, false, false),
            other => {
                return Err(val_err(format!(
                    "invalid sort '{other}'; expected one of auto/morton/hilbert/id/none"
                )));
            }
        };
        cfg.try_spatial_morton_sort = morton;
        cfg.try_spatial_hilbert_sort = hilbert;
        cfg.try_id_sort = id;
    }

    if let Some(v) = opt_bool(opts, "tessellate")? {
        cfg.tessellate = v;
    }
    if let Some(v) = opt_bool(opts, "allow_fsst")? {
        cfg.allow_fsst = v;
    }
    if let Some(v) = opt_bool(opts, "allow_fpf")? {
        cfg.allow_fpf = v;
    }
    if let Some(v) = opt_bool(opts, "allow_shared_dict")? {
        cfg.allow_shared_dict = v;
    }

    Ok(cfg)
}

/// Encode a single layer into MLT bytes.
///
/// `layer` is either a GeoJSON `FeatureCollection` (its layer `name`/`extent`
/// come from `options`) or a layer dict `{name, extent?, features}`. Geometry
/// is in tile-local coordinate space (no projection); see the module docs.
#[gen_stub_pyfunction]
#[pyfunction]
#[pyo3(signature = (layer, options=None))]
pub fn encode(
    py: Python<'_>,
    #[gen_stub(override_type(type_repr = "typing.Mapping[builtins.str, builtins.object]"))]
    layer: &Bound<'_, PyAny>,
    #[gen_stub(override_type(
        type_repr = "typing.Optional[typing.Mapping[builtins.str, builtins.object]]"
    ))]
    options: Option<&Bound<'_, PyAny>>,
) -> PyResult<Py<PyBytes>> {
    let dict = layer
        .cast::<PyDict>()
        .map_err(|_| val_err("layer must be a FeatureCollection or layer dict"))?;
    let opts = options_dict(options)?;

    let is_fc = matches!(
        dict.get_item("type")?.and_then(|t| t.extract::<String>().ok()),
        Some(t) if t == "FeatureCollection"
    );
    let (name, extent) = resolve_name_extent(dict, opts.as_ref(), is_fc)?;

    let features_obj = dict
        .get_item("features")?
        .ok_or_else(|| val_err("layer missing 'features'"))?;
    let features_list = features_obj
        .cast::<PyList>()
        .map_err(|_| val_err("'features' must be a list"))?;

    let mut raw = Vec::with_capacity(features_list.len());
    for feat in features_list.iter() {
        raw.push(parse_raw_feature(&feat)?);
    }
    if raw.is_empty() {
        return Err(val_err("layer has no features"));
    }

    let (property_names, kinds) = infer_columns(&raw);
    let features = build_tile_features(raw, &property_names, &kinds);

    let tile = TileLayer {
        name,
        extent,
        property_names,
        features,
    };
    let cfg = build_config(opts.as_ref())?;
    let bytes = tile
        .encode(cfg)
        .map_err(|e| val_err(format!("MLT encode error: {e}")))?;
    Ok(PyBytes::new(py, &bytes).unbind())
}
