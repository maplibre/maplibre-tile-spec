mod feature;
mod tile_transform;

use std::iter::once;
use std::ops::Deref;

use geo_types::{LineString, Polygon};
use mlt_core::geojson::{FeatureCollection, Geom32};
use mlt_core::v01::{
    DecodedGeometry, DecodedId, DecodedProperty, Geometry as MltGeometry, Id, PropValue, Property,
};
use mlt_core::{MltError, parse_layers};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use pyo3_stub_gen::define_stub_info_gatherer;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};
use tile_transform::TileTransform;

use crate::feature::MltFeature;

fn mlt_err(e: MltError) -> PyErr {
    PyValueError::new_err(format!("MLT decode error: {e}"))
}

/// A decoded MLT layer containing features.
#[gen_stub_pyclass]
#[pyclass]
struct MltLayer {
    #[pyo3(get)]
    name: String,
    #[pyo3(get)]
    extent: u32,
    #[pyo3(get)]
    features: Vec<Py<MltFeature>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl MltLayer {
    fn __repr__(&self) -> String {
        format!(
            "MltLayer(name={:?}, extent={}, features=<{} features>)",
            self.name,
            self.extent,
            self.features.len()
        )
    }
}

fn push_coord_raw(buf: &mut Vec<u8>, coord: [i32; 2]) {
    buf.extend_from_slice(&f64::from(coord[0]).to_le_bytes());
    buf.extend_from_slice(&f64::from(coord[1]).to_le_bytes());
}

fn push_coord_xform(buf: &mut Vec<u8>, coord: [i32; 2], xf: TileTransform) {
    let [x, y] = xf.apply(coord);
    buf.extend_from_slice(&x.to_le_bytes());
    buf.extend_from_slice(&y.to_le_bytes());
}

fn push_coord(buf: &mut Vec<u8>, coord: [i32; 2], xf: Option<TileTransform>) {
    match xf {
        Some(xf) => push_coord_xform(buf, coord, xf),
        None => push_coord_raw(buf, coord),
    }
}

fn push_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn push_rings(
    buf: &mut Vec<u8>,
    rings: impl IntoIterator<Item = impl Deref<Target = LineString<i32>>>,
    xf: Option<TileTransform>,
) {
    for ring in rings {
        push_u32(buf, ring.0.len() as u32);
        for c in &ring.0 {
            push_coord(buf, (*c).into(), xf);
        }
    }
}

fn push_linestring(
    buf: &mut Vec<u8>,
    line: impl Deref<Target = LineString<i32>>,
    xf: Option<TileTransform>,
) {
    buf.push(0x01);
    push_u32(buf, 2);
    push_rings(buf, once(line), xf);
}

fn push_polygon(buf: &mut Vec<u8>, poly: &Polygon<i32>, xf: Option<TileTransform>) {
    buf.push(0x01);
    push_u32(buf, 3);
    push_u32(buf, (poly.interiors().len() + 1) as u32);
    push_rings(buf, once(poly.exterior()).chain(poly.interiors()), xf);
}

fn geom_to_wkb(
    geom: &DecodedGeometry,
    index: usize,
    xf: Option<TileTransform>,
) -> Result<Vec<u8>, MltError> {
    let gj = geom.to_geojson(index)?;
    let mut buf = Vec::with_capacity(128);

    match gj {
        Geom32::Point(c) => {
            buf.push(0x01);
            push_u32(&mut buf, 1);
            push_coord(&mut buf, c.into(), xf);
        }
        Geom32::LineString(coords) => push_linestring(&mut buf, &coords, xf),
        Geom32::Polygon(poly) => push_polygon(&mut buf, &poly, xf),
        Geom32::MultiPoint(coords) => {
            buf.push(0x01);
            push_u32(&mut buf, 4);
            push_u32(&mut buf, coords.0.len() as u32);
            for c in &coords.0 {
                buf.push(0x01);
                push_u32(&mut buf, 1);
                push_coord(&mut buf, (*c).into(), xf);
            }
        }
        Geom32::MultiLineString(lines) => {
            buf.push(0x01);
            push_u32(&mut buf, 5);
            push_u32(&mut buf, lines.0.len() as u32);
            for line in &lines.0 {
                push_linestring(&mut buf, line, xf);
            }
        }
        Geom32::MultiPolygon(polygons) => {
            buf.push(0x01);
            push_u32(&mut buf, 6);
            push_u32(&mut buf, polygons.0.len() as u32);
            for polygon in &polygons.0 {
                push_polygon(&mut buf, polygon, xf);
            }
        }
        _ => return Err(MltError::NotImplemented("unsupported geometry type")),
    }

    Ok(buf)
}

fn prop_value_to_py(py: Python<'_>, pv: &PropValue, i: usize) -> Py<PyAny> {
    match pv {
        PropValue::Bool(v) => match v[i] {
            Some(b) => b.into_pyobject(py).unwrap().to_owned().into_any().unbind(),
            None => py.None(),
        },
        PropValue::I8(v) => match v[i] {
            Some(n) => n.into_pyobject(py).unwrap().into_any().unbind(),
            None => py.None(),
        },
        PropValue::U8(v) => match v[i] {
            Some(n) => n.into_pyobject(py).unwrap().into_any().unbind(),
            None => py.None(),
        },
        PropValue::I32(v) => match v[i] {
            Some(n) => n.into_pyobject(py).unwrap().into_any().unbind(),
            None => py.None(),
        },
        PropValue::U32(v) => match v[i] {
            Some(n) => n.into_pyobject(py).unwrap().into_any().unbind(),
            None => py.None(),
        },
        PropValue::I64(v) => match v[i] {
            Some(n) => n.into_pyobject(py).unwrap().into_any().unbind(),
            None => py.None(),
        },
        PropValue::U64(v) => match v[i] {
            Some(n) => n.into_pyobject(py).unwrap().into_any().unbind(),
            None => py.None(),
        },
        PropValue::F32(v) => match v[i] {
            Some(n) => n.into_pyobject(py).unwrap().into_any().unbind(),
            None => py.None(),
        },
        PropValue::F64(v) => match v[i] {
            Some(n) => n.into_pyobject(py).unwrap().into_any().unbind(),
            None => py.None(),
        },
        PropValue::Str(v) => match &v[i] {
            Some(s) => s.into_pyobject(py).unwrap().into_any().unbind(),
            None => py.None(),
        },
        PropValue::Struct => py.None(),
    }
}

fn build_features(
    py: Python<'_>,
    geom: &DecodedGeometry,
    ids: Option<&[Option<u64>]>,
    props: &[&DecodedProperty],
    xf: Option<TileTransform>,
) -> PyResult<Vec<Py<MltFeature>>> {
    let count = geom.vector_types.len();
    let mut features = Vec::with_capacity(count);

    for i in 0..count {
        let id = ids.and_then(|v| v.get(i).copied().flatten());
        let gt = geom.vector_types[i];

        let wkb_bytes = geom_to_wkb(geom, i, xf).map_err(mlt_err)?;
        let wkb = PyBytes::new(py, &wkb_bytes).unbind();

        let prop_dict = PyDict::new(py);
        for p in props {
            prop_dict.set_item(&p.name, prop_value_to_py(py, &p.values, i))?;
        }

        let feat = Py::new(
            py,
            MltFeature::new(id, format!("{gt}"), wkb, prop_dict.unbind()),
        )?;
        features.push(feat);
    }

    Ok(features)
}

/// Decode an MLT binary blob into a list of `MltLayer` objects.
///
/// If `z`, `x`, `y` are provided, tile-local coordinates are transformed
/// to EPSG:3857 (Web Mercator) meters. Without them, raw tile coordinates
/// are preserved.
///
/// `tms`: when True (the default), treat `y` as TMS convention (y=0 at south,
/// used by OpenMapTiles / MBTiles). Set to False for XYZ / slippy-map tiles
/// (y=0 at north, e.g. OSM raster tiles).
#[gen_stub_pyfunction]
#[pyfunction]
#[pyo3(signature = (data, z=None, x=None, y=None, tms=true))]
fn decode_mlt(
    py: Python<'_>,
    #[gen_stub(override_type(type_repr = "bytes"))] data: &[u8],
    z: Option<u32>,
    x: Option<u32>,
    y: Option<u32>,
    tms: bool,
) -> PyResult<Vec<MltLayer>> {
    let mut layers = parse_layers(data).map_err(mlt_err)?;
    let mut result = Vec::with_capacity(layers.len());
    for layer in &mut layers {
        layer.decode_all().map_err(mlt_err)?;

        let layer = layer
            .as_layer01()
            .ok_or_else(|| PyValueError::new_err("unsupported layer tag (expected 0x01)"))?;

        let xf = match (z, x, y) {
            (Some(z), Some(x), Some(y)) => {
                Some(TileTransform::from_zxy(z, x, y, layer.extent, tms)?)
            }
            _ => None,
        };

        let geom = match &layer.geometry {
            MltGeometry::Decoded(g) => g,
            _ => return Err(PyValueError::new_err("geometry not decoded")),
        };

        let ids = match &layer.id {
            Id::Decoded(DecodedId(v)) => v.as_deref(),
            Id::None => None,
            _ => return Err(PyValueError::new_err("id not decoded")),
        };

        let props: Vec<&DecodedProperty> = layer
            .properties
            .iter()
            .map(|p| match p {
                Property::Decoded(d) => Ok(d),
                _ => Err(PyValueError::new_err("property not decoded")),
            })
            .collect::<PyResult<_>>()?;

        result.push(MltLayer {
            name: layer.name.to_string(),
            extent: layer.extent,
            features: build_features(py, geom, ids, &props, xf)?,
        });
    }

    Ok(result)
}

/// Decode an MLT binary blob and return GeoJSON as a string.
#[gen_stub_pyfunction]
#[pyfunction]
fn decode_mlt_to_geojson(
    #[gen_stub(override_type(type_repr = "bytes"))] data: &[u8],
) -> PyResult<String> {
    let mut layers = parse_layers(data).map_err(mlt_err)?;
    for layer in &mut layers {
        layer.decode_all().map_err(mlt_err)?;
    }
    let fc = FeatureCollection::from_layers(&layers).map_err(mlt_err)?;
    serde_json::to_string(&fc).map_err(|e| PyValueError::new_err(format!("JSON error: {e}")))
}

/// Return a list of layer names without fully decoding.
#[gen_stub_pyfunction]
#[pyfunction]
fn list_layers(
    #[gen_stub(override_type(type_repr = "bytes"))] data: &[u8],
) -> PyResult<Vec<String>> {
    let layers = parse_layers(data).map_err(mlt_err)?;
    Ok(layers
        .iter()
        .filter_map(|l| l.as_layer01().map(|l| l.name.to_string()))
        .collect())
}

#[pymodule]
fn maplibre_tiles(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(decode_mlt, m)?)?;
    m.add_function(wrap_pyfunction!(decode_mlt_to_geojson, m)?)?;
    m.add_function(wrap_pyfunction!(list_layers, m)?)?;
    m.add_class::<MltLayer>()?;
    m.add_class::<MltFeature>()?;
    Ok(())
}

define_stub_info_gatherer!(stub_info);

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;
    use std::fs;

    use super::*;

    #[test]
    fn tile_transform_rejects_zoom_above_30() {
        let result = TileTransform::from_zxy(31, 0, 0, 4096, false);
        assert!(result.is_err(), "z=31 should be rejected");

        let result = TileTransform::from_zxy(30, 0, 0, 4096, false);
        assert!(result.is_ok(), "z=30 should be accepted");

        let result = TileTransform::from_zxy(0, 0, 0, 4096, false);
        assert!(result.is_ok(), "z=0 should be accepted");
    }

    #[test]
    fn tile_transform_zoom_zero_covers_world() {
        let xf = TileTransform::from_zxy(0, 0, 0, 4096, false).unwrap();

        let circumference = 2.0 * PI * 6_378_137.0;
        let half = circumference / 2.0;

        assert!(
            (xf.x_origin + half).abs() < 1.0,
            "x_origin at z=0 should be -half_circumference"
        );
        assert!(
            (xf.y_origin - half).abs() < 1.0,
            "y_origin at z=0 should be +half_circumference"
        );

        let tile_scale = circumference / 4096.0;
        assert!(
            (xf.x_scale - tile_scale).abs() < 1e-6,
            "x_scale should equal circumference / extent"
        );
        assert!(
            (xf.y_scale + tile_scale).abs() < 1e-6,
            "y_scale should be negative (flipped)"
        );
    }

    #[test]
    fn tile_transform_apply_maps_origin_and_extent() {
        let xf = TileTransform::from_zxy(0, 0, 0, 4096, false).unwrap();

        let origin = xf.apply([0, 0]);
        assert!(
            (origin[0] - xf.x_origin).abs() < 1e-6,
            "apply([0,0]).x should equal x_origin"
        );
        assert!(
            (origin[1] - xf.y_origin).abs() < 1e-6,
            "apply([0,0]).y should equal y_origin"
        );

        let far_corner = xf.apply([4096, 4096]);
        let circumference = 2.0 * PI * 6_378_137.0;
        let half = circumference / 2.0;
        assert!(
            (far_corner[0] - half).abs() < 1.0,
            "apply([4096,4096]).x should reach +half"
        );
        assert!(
            (far_corner[1] + half).abs() < 1.0,
            "apply([4096,4096]).y should reach -half"
        );
    }

    #[test]
    fn tile_transform_tms_vs_xyz() {
        let xyz = TileTransform::from_zxy(1, 0, 0, 4096, false).unwrap();
        let tms = TileTransform::from_zxy(1, 0, 1, 4096, true).unwrap();

        assert!(
            (xyz.x_origin - tms.x_origin).abs() < 1e-6,
            "same tile via TMS and XYZ should produce same x_origin"
        );
        assert!(
            (xyz.y_origin - tms.y_origin).abs() < 1e-6,
            "same tile via TMS and XYZ should produce same y_origin"
        );
    }

    #[test]
    fn fixture_parse_and_feature_collection() {
        let fixture_path = "../../test/synthetic/0x01/point.mlt";
        let data = fs::read(fixture_path)
            .unwrap_or_else(|e| panic!("failed to read fixture {fixture_path}: {e}"));

        let mut layers = parse_layers(&data).expect("parse_layers should succeed");
        for layer in &mut layers {
            layer.decode_all().expect("decode_all should succeed");
        }

        assert!(!layers.is_empty(), "should parse at least one layer");
        let l = layers[0].as_layer01().expect("first layer should be v0.1");
        assert!(!l.name.is_empty(), "layer name should be non-empty");

        let fc = FeatureCollection::from_layers(&layers).expect("FeatureCollection should succeed");
        assert!(
            !fc.features.is_empty(),
            "feature collection should have features"
        );
    }

    #[test]
    fn fixture_geom_to_wkb_produces_valid_output() {
        let fixture_path = "../../test/synthetic/0x01/polygon.mlt";
        let data = fs::read(fixture_path)
            .unwrap_or_else(|e| panic!("failed to read fixture {fixture_path}: {e}"));

        let mut layers = parse_layers(&data).expect("parse_layers should succeed");
        for layer in &mut layers {
            layer.decode_all().expect("decode_all should succeed");
        }

        let l = layers[0].as_layer01().expect("first layer should be v0.1");
        let geom = match &l.geometry {
            MltGeometry::Decoded(g) => g,
            _ => panic!("geometry not decoded"),
        };

        let wkb = geom_to_wkb(geom, 0, None).expect("geom_to_wkb should succeed");
        assert!(
            wkb.len() >= 5,
            "WKB must be at least 5 bytes (byte order + type)"
        );
        assert_eq!(wkb[0], 0x01, "WKB byte order should be little-endian");
        let wkb_type = u32::from_le_bytes([wkb[1], wkb[2], wkb[3], wkb[4]]);
        assert_eq!(
            wkb_type, 3,
            "polygon fixture should produce WKB type 3 (Polygon)"
        );
    }

    #[test]
    fn fixture_geom_to_wkb_with_transform() {
        let fixture_path = "../../test/synthetic/0x01/point.mlt";
        let data = fs::read(fixture_path)
            .unwrap_or_else(|e| panic!("failed to read fixture {fixture_path}: {e}"));

        let mut layers = parse_layers(&data).expect("parse_layers should succeed");
        for layer in &mut layers {
            layer.decode_all().expect("decode_all should succeed");
        }

        let l = layers[0].as_layer01().expect("first layer should be v0.1");
        let geom = match &l.geometry {
            MltGeometry::Decoded(g) => g,
            _ => panic!("geometry not decoded"),
        };

        let xf = TileTransform::from_zxy(0, 0, 0, l.extent, false).unwrap();

        let wkb_raw = geom_to_wkb(geom, 0, None).expect("raw wkb should succeed");
        let wkb_xf = geom_to_wkb(geom, 0, Some(xf)).expect("transformed wkb should succeed");

        assert_eq!(
            wkb_raw.len(),
            wkb_xf.len(),
            "raw and transformed WKB should have the same length"
        );
        assert_ne!(
            wkb_raw, wkb_xf,
            "transformed WKB should differ from raw (unless coordinates are trivially 0)"
        );
    }

    #[test]
    fn fixture_line_produces_wkb_linestring() {
        let fixture_path = "../../test/synthetic/0x01/line.mlt";
        let data = fs::read(fixture_path)
            .unwrap_or_else(|e| panic!("failed to read fixture {fixture_path}: {e}"));

        let mut layers = parse_layers(&data).expect("parse_layers should succeed");
        for layer in &mut layers {
            layer.decode_all().expect("decode_all should succeed");
        }

        let l = layers[0].as_layer01().expect("first layer should be v0.1");
        let geom = match &l.geometry {
            MltGeometry::Decoded(g) => g,
            _ => panic!("geometry not decoded"),
        };

        let wkb = geom_to_wkb(geom, 0, None).expect("geom_to_wkb should succeed");
        assert!(wkb.len() >= 5);
        let wkb_type = u32::from_le_bytes([wkb[1], wkb[2], wkb[3], wkb[4]]);
        assert_eq!(
            wkb_type, 2,
            "line fixture should produce WKB type 2 (LineString)"
        );
    }
}
