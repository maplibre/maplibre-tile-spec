mod encode;
mod feature;
mod tile_transform;

use mlt_core::geojson::FeatureCollection;
use mlt_core::wkt::Wkt;
use mlt_core::wkt::types::{Coord, Dimension, LineString, Polygon};
use mlt_core::{
    Decoder, GeometryType, Layer, LendingIterator, MltError, MltResult, ParsedLayer01, Parser,
    PropValueRef,
};
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

/// Append one coordinate as little-endian f64 `x, y` (and `z` when `has_z`).
///
/// The tile transform applies to the planar `x`/`y` only; `z` is elevation and is written as-is.
fn push_coord(buf: &mut Vec<u8>, c: &Coord<i32>, has_z: bool, xf: Option<TileTransform>) {
    let [x, y] = match xf {
        Some(xf) => xf.apply([c.x, c.y]),
        None => [f64::from(c.x), f64::from(c.y)],
    };
    buf.extend_from_slice(&x.to_le_bytes());
    buf.extend_from_slice(&y.to_le_bytes());
    if has_z {
        buf.extend_from_slice(&f64::from(c.z.unwrap_or(0)).to_le_bytes());
    }
}

fn push_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

/// ISO WKB geometry type code: `base` for 2D, `base + 1000` for the Z (3D) variant.
fn wkb_type(base: u32, has_z: bool) -> u32 {
    if has_z { base + 1000 } else { base }
}

/// Append a coordinate sequence: a `u32` count followed by each coordinate.
fn push_coords(buf: &mut Vec<u8>, coords: &[Coord<i32>], has_z: bool, xf: Option<TileTransform>) {
    push_u32(buf, coords.len() as u32);
    for c in coords {
        push_coord(buf, c, has_z, xf);
    }
}

fn push_linestring(
    buf: &mut Vec<u8>,
    line: &LineString<i32>,
    has_z: bool,
    xf: Option<TileTransform>,
) {
    buf.push(0x01);
    push_u32(buf, wkb_type(2, has_z));
    push_coords(buf, line.coords(), has_z, xf);
}

fn push_polygon(buf: &mut Vec<u8>, poly: &Polygon<i32>, has_z: bool, xf: Option<TileTransform>) {
    buf.push(0x01);
    push_u32(buf, wkb_type(3, has_z));
    push_u32(buf, poly.rings().len() as u32);
    for ring in poly.rings() {
        push_coords(buf, ring.coords(), has_z, xf);
    }
}

/// Encode a decoded geometry as WKB. 3D geometries are emitted as ISO WKB with the Z coordinate
/// preserved (type code `base + 1000`); 2D geometries are unchanged.
fn geom32_to_wkb(geom: &Wkt<i32>, xf: Option<TileTransform>) -> MltResult<Vec<u8>> {
    let has_z = matches!(geom.dimension(), Dimension::XYZ);
    let mut buf = Vec::with_capacity(128);
    match geom {
        Wkt::Point(p) => {
            buf.push(0x01);
            push_u32(&mut buf, wkb_type(1, has_z));
            if let Some(c) = p.coord() {
                push_coord(&mut buf, c, has_z, xf);
            }
        }
        Wkt::LineString(line) => push_linestring(&mut buf, line, has_z, xf),
        Wkt::Polygon(poly) => push_polygon(&mut buf, poly, has_z, xf),
        Wkt::MultiPoint(mp) => {
            buf.push(0x01);
            push_u32(&mut buf, wkb_type(4, has_z));
            push_u32(&mut buf, mp.points().len() as u32);
            for p in mp.points() {
                buf.push(0x01);
                push_u32(&mut buf, wkb_type(1, has_z));
                if let Some(c) = p.coord() {
                    push_coord(&mut buf, c, has_z, xf);
                }
            }
        }
        Wkt::MultiLineString(mls) => {
            buf.push(0x01);
            push_u32(&mut buf, wkb_type(5, has_z));
            push_u32(&mut buf, mls.line_strings().len() as u32);
            for line in mls.line_strings() {
                push_linestring(&mut buf, line, has_z, xf);
            }
        }
        Wkt::MultiPolygon(mp) => {
            buf.push(0x01);
            push_u32(&mut buf, wkb_type(6, has_z));
            push_u32(&mut buf, mp.polygons().len() as u32);
            for poly in mp.polygons() {
                push_polygon(&mut buf, poly, has_z, xf);
            }
        }
        Wkt::GeometryCollection(_) => {
            return Err(MltError::NotImplemented("unsupported geometry type"));
        }
    }
    Ok(buf)
}

fn prop_value_to_py(py: Python<'_>, v: PropValueRef<'_>) -> Py<PyAny> {
    match v {
        PropValueRef::Bool(b) => b.into_pyobject(py).unwrap().to_owned().into_any().unbind(),
        PropValueRef::I8(n) => n.into_pyobject(py).unwrap().into_any().unbind(),
        PropValueRef::U8(n) => n.into_pyobject(py).unwrap().into_any().unbind(),
        PropValueRef::I32(n) => n.into_pyobject(py).unwrap().into_any().unbind(),
        PropValueRef::U32(n) => n.into_pyobject(py).unwrap().into_any().unbind(),
        PropValueRef::I64(n) => n.into_pyobject(py).unwrap().into_any().unbind(),
        PropValueRef::U64(n) => n.into_pyobject(py).unwrap().into_any().unbind(),
        PropValueRef::F32(n) => n.into_pyobject(py).unwrap().into_any().unbind(),
        PropValueRef::F64(n) => n.into_pyobject(py).unwrap().into_any().unbind(),
        PropValueRef::Str(s) => s.into_pyobject(py).unwrap().into_any().unbind(),
    }
}

fn build_features(
    py: Python<'_>,
    layer: &ParsedLayer01<'_>,
    xf: Option<TileTransform>,
) -> PyResult<Vec<Py<MltFeature>>> {
    let mut features = Vec::new();
    let mut feat_iter = layer.iter_features();
    while let Some(feat_result) = feat_iter.next() {
        let feat = feat_result.map_err(mlt_err)?;
        let geometry_type = GeometryType::try_from(feat.geometry())
            .map(|gt| gt.to_string())
            .unwrap_or_else(|_| "Unknown".to_string());
        let wkb_bytes = geom32_to_wkb(feat.geometry(), xf).map_err(mlt_err)?;
        let wkb = PyBytes::new(py, &wkb_bytes).unbind();
        let prop_dict = PyDict::new(py);
        for p in feat.iter_properties() {
            prop_dict.set_item(p.name().to_string(), prop_value_to_py(py, p.value()))?;
        }
        let feature = MltFeature::new(feat.id(), geometry_type, wkb, prop_dict.unbind());
        features.push(Py::new(py, feature)?);
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
    let mut dec = Decoder::default();
    let mut result = Vec::new();
    for lazy_layer in Parser::default().parse_layers(data).map_err(mlt_err)? {
        let Layer::Tag01(layer01) = lazy_layer else {
            return Err(PyValueError::new_err(
                "unsupported layer tag (expected 0x01)",
            ));
        };
        let decoded = layer01.decode_all(&mut dec).map_err(mlt_err)?;
        let extent = decoded.extent().get();
        let xf = match (z, x, y) {
            (Some(z), Some(x), Some(y)) => Some(TileTransform::from_zxy(z, x, y, extent, tms)?),
            _ => None,
        };
        result.push(MltLayer {
            name: decoded.name().to_string(),
            extent,
            features: build_features(py, &decoded, xf)?,
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
    let mut dec = Decoder::default();
    let layers = dec
        .decode_all(Parser::default().parse_layers(data).map_err(mlt_err)?)
        .map_err(mlt_err)?;
    let fc = FeatureCollection::from_layers(layers).map_err(mlt_err)?;
    serde_json::to_string(&fc).map_err(|e| PyValueError::new_err(format!("JSON error: {e}")))
}

/// Return a list of layer names without fully decoding.
#[gen_stub_pyfunction]
#[pyfunction]
fn list_layers(
    #[gen_stub(override_type(type_repr = "bytes"))] data: &[u8],
) -> PyResult<Vec<String>> {
    let layers = Parser::default().parse_layers(data).map_err(mlt_err)?;
    Ok(layers
        .iter()
        .filter_map(|l| l.as_layer01().map(|l| l.name().to_string()))
        .collect())
}

#[pymodule]
fn maplibre_tiles(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(decode_mlt, m)?)?;
    m.add_function(wrap_pyfunction!(decode_mlt_to_geojson, m)?)?;
    m.add_function(wrap_pyfunction!(list_layers, m)?)?;
    m.add_function(wrap_pyfunction!(encode::geojson::encode_geojson, m)?)?;
    m.add_function(wrap_pyfunction!(encode::mvt::encode_mvt, m)?)?;
    m.add_class::<MltLayer>()?;
    m.add_class::<MltFeature>()?;
    Ok(())
}

define_stub_info_gatherer!(stub_info);

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;
    use std::fs;

    use mlt_core::{Decoder, GeometryValues};

    use super::*;

    fn geom_to_wkb(
        geom: &GeometryValues,
        index: usize,
        xf: Option<TileTransform>,
    ) -> MltResult<Vec<u8>> {
        geom32_to_wkb(&geom.to_geojson(index)?, xf)
    }

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

        let layers = Parser::default()
            .parse_layers(&data)
            .expect("parse_layers should succeed");
        let mut dec = Decoder::default();
        let decoded = dec.decode_all(layers).expect("decode_all should succeed");

        assert!(!decoded.is_empty(), "should parse at least one layer");
        let l = decoded[0].as_layer01().expect("first layer should be v0.1");
        assert!(!l.name().is_empty(), "layer name should be non-empty");

        let fc = FeatureCollection::from_layers(decoded).expect("FeatureCollection should succeed");
        assert!(
            !fc.features.is_empty(),
            "feature collection should have features"
        );
    }

    #[test]
    fn fixture_geom_to_wkb_produces_valid_output() {
        let fixture_path = "../../test/synthetic/0x01/poly.mlt";
        let data = fs::read(fixture_path)
            .unwrap_or_else(|e| panic!("failed to read fixture {fixture_path}: {e}"));

        let layers = Parser::default()
            .parse_layers(&data)
            .expect("parse_layers should succeed");
        let mut dec = Decoder::default();
        let decoded = dec.decode_all(layers).expect("decode_all should succeed");

        let l = decoded[0].as_layer01().expect("first layer should be v0.1");
        let geom = l.geometry_values();

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

        let layers = Parser::default()
            .parse_layers(&data)
            .expect("parse_layers should succeed");
        let mut dec = Decoder::default();
        let decoded = dec.decode_all(layers).expect("decode_all should succeed");

        let l = decoded[0].as_layer01().expect("first layer should be v0.1");
        let geom = l.geometry_values();

        let xf = TileTransform::from_zxy(0, 0, 0, l.extent().get(), false).unwrap();

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

        let layers = Parser::default()
            .parse_layers(&data)
            .expect("parse_layers should succeed");
        let mut dec = Decoder::default();
        let decoded = dec.decode_all(layers).expect("decode_all should succeed");

        let l = decoded[0].as_layer01().expect("first layer should be v0.1");
        let geom = l.geometry_values();

        let wkb = geom_to_wkb(geom, 0, None).expect("geom_to_wkb should succeed");
        assert!(wkb.len() >= 5);
        let wkb_type = u32::from_le_bytes([wkb[1], wkb[2], wkb[3], wkb[4]]);
        assert_eq!(
            wkb_type, 2,
            "line fixture should produce WKB type 2 (LineString)"
        );
    }

    /// A 3D geometry must emit ISO WKB with the Z coordinate preserved (type `base + 1000`).
    #[test]
    fn wkb_preserves_z_for_3d_linestring() {
        use mlt_core::wkt::Wkt;
        use mlt_core::wkt::types::{Coord, Dimension, LineString};

        let z = |x, y, z| Coord {
            x,
            y,
            z: Some(z),
            m: None,
        };
        let ls = Wkt::LineString(LineString::new(
            vec![z(1, 2, 3), z(4, 5, 6)],
            Dimension::XYZ,
        ));
        let geom = GeometryValues::default().with_geom(&ls);

        let wkb = geom32_to_wkb(&geom.to_geojson(0).unwrap(), None).expect("wkb");

        // byte order (1) + type (4) + count (4) + 2 coords * 3 components * 8 bytes.
        assert_eq!(wkb.len(), 1 + 4 + 4 + 2 * 3 * 8);
        let wkb_type = u32::from_le_bytes([wkb[1], wkb[2], wkb[3], wkb[4]]);
        assert_eq!(wkb_type, 1002, "3D LineString -> ISO WKB type 1002");
        // The first coordinate's Z (third f64, after x and y) must be 3.0.
        let z0 = f64::from_le_bytes(wkb[9 + 16..9 + 24].try_into().unwrap());
        assert_eq!(z0, 3.0);
    }
}
