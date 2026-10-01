mod encode;
mod feature;
mod tile_transform;

use std::iter::once;

use mlt_core::geo_types::{Coord, Geometry, LineString, Polygon};
use mlt_core::geojson::FeatureCollection;
use mlt_core::{
    Decoder, GeometryType, Layer, LendingIterator, MltError, MltResult, ParsedLayer01, Parser,
    PropValueRef, ZStep,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use pyo3_stub_gen::define_stub_info_gatherer;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};
use tile_transform::TileTransform;

use crate::feature::MltFeature;

#[expect(clippy::needless_pass_by_value, reason = "helper util")]
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
    /// The power of ten of the z grid's step in metres, or `None` when the vertices carry no z.
    #[pyo3(get)]
    z_step: Option<i8>,
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

/// Writes WKB, taking the next z for each stored vertex when the layer has z.
///
/// Z follows xy: the grid value in tile coordinates, the elevation in metres once `xf` projects.
struct WkbWriter<'a> {
    buf: Vec<u8>,
    xf: Option<TileTransform>,
    z: Option<(ZStep, &'a [i32])>,
    vertex: usize,
}

impl<'a> WkbWriter<'a> {
    fn new(xf: Option<TileTransform>, z: Option<(ZStep, &'a [i32])>) -> Self {
        Self {
            buf: Vec::with_capacity(128),
            xf,
            z,
            vertex: 0,
        }
    }

    fn f64(&mut self, v: f64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn len(&mut self, len: usize) -> MltResult<()> {
        self.u32(u32::try_from(len).map_err(|_| MltError::IntegerOverflow)?);
        Ok(())
    }

    /// The byte order and type, in ISO WKB's Z variant when the layer has z.
    fn header(&mut self, ty: u32) {
        self.buf.push(0x01);
        self.u32(if self.z.is_some() { ty + 1000 } else { ty });
    }

    /// Write the next stored vertex, returning its z.
    fn vertex(&mut self, coord: Coord<i32>) -> Option<i32> {
        let z = self
            .z
            .map(|(_, z)| z.get(self.vertex).copied().unwrap_or_default());
        self.vertex += 1;
        self.coord(coord, z);
        z
    }

    fn coord(&mut self, coord: Coord<i32>, z: Option<i32>) {
        let [x, y] = match self.xf {
            Some(xf) => xf.apply(coord.into()),
            None => [coord.x, coord.y].map(f64::from),
        };
        self.f64(x);
        self.f64(y);
        if let (Some((step, _)), Some(z)) = (self.z, z) {
            let z = if self.xf.is_some() {
                step.elevation(z)
            } else {
                f64::from(z)
            };
            self.f64(z);
        }
    }

    fn point(&mut self, coord: Coord<i32>) {
        self.header(1);
        self.vertex(coord);
    }

    fn line(&mut self, line: &LineString<i32>) -> MltResult<()> {
        self.header(2);
        self.len(line.0.len())?;
        for &c in &line.0 {
            self.vertex(c);
        }
        Ok(())
    }

    /// A ring's closing position repeats its first, z included, because MLT does not store it.
    fn ring(&mut self, ring: &LineString<i32>) -> MltResult<()> {
        let coords = &ring.0;
        self.len(coords.len())?;
        let Some((&first, rest)) = coords.split_first() else {
            return Ok(());
        };
        let first_z = self.vertex(first);
        match rest.split_last() {
            Some((&last, middle)) if last == first => {
                for &c in middle {
                    self.vertex(c);
                }
                self.coord(first, first_z);
            }
            _ => {
                for &c in rest {
                    self.vertex(c);
                }
            }
        }
        Ok(())
    }

    fn polygon(&mut self, poly: &Polygon<i32>) -> MltResult<()> {
        self.header(3);
        self.len(poly.interiors().len() + 1)?;
        for ring in once(poly.exterior()).chain(poly.interiors()) {
            self.ring(ring)?;
        }
        Ok(())
    }

    fn geometry(&mut self, geom: &Geometry<i32>) -> MltResult<()> {
        match geom {
            Geometry::Point(p) => self.point(p.0),
            Geometry::LineString(line) => self.line(line)?,
            Geometry::Polygon(poly) => self.polygon(poly)?,
            Geometry::MultiPoint(points) => {
                self.header(4);
                self.len(points.0.len())?;
                for p in &points.0 {
                    self.point(p.0);
                }
            }
            Geometry::MultiLineString(lines) => {
                self.header(5);
                self.len(lines.0.len())?;
                for line in &lines.0 {
                    self.line(line)?;
                }
            }
            Geometry::MultiPolygon(polygons) => {
                self.header(6);
                self.len(polygons.0.len())?;
                for polygon in &polygons.0 {
                    self.polygon(polygon)?;
                }
            }
            Geometry::Line(_)
            | Geometry::GeometryCollection(_)
            | Geometry::Rect(_)
            | Geometry::Triangle(_) => {
                return Err(MltError::NotImplemented("unsupported geometry type"));
            }
        }
        Ok(())
    }

    fn finish(self) -> MltResult<Vec<u8>> {
        match self.z {
            Some((_, z)) if z.len() != self.vertex => Err(MltError::ZVertexCountMismatch {
                expected: self.vertex,
                actual: z.len(),
            }),
            _ => Ok(self.buf),
        }
    }
}

fn geom32_to_wkb(
    geom: &Geometry<i32>,
    xf: Option<TileTransform>,
    z: Option<(ZStep, &[i32])>,
) -> MltResult<Vec<u8>> {
    let mut writer = WkbWriter::new(xf, z);
    writer.geometry(geom)?;
    writer.finish()
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
    let geometry = layer.geometry_values();
    let z_step = geometry.z_step();
    let mut features = Vec::new();
    let mut feat_iter = layer.iter_features();
    let mut index = 0;
    while let Some(feat_result) = feat_iter.next() {
        let feat = feat_result.map_err(mlt_err)?;
        let geometry_type = GeometryType::try_from(feat.geometry())
            .map_or_else(|()| "Unknown".to_string(), |gt| gt.to_string());
        let z = z_step
            .map(|step| geometry.z(index).map(|z| (step, z)))
            .transpose()
            .map_err(mlt_err)?;
        index += 1;
        let z = z.as_ref().map(|(step, z)| (*step, z.as_slice()));
        let wkb_bytes = geom32_to_wkb(feat.geometry(), xf, z).map_err(mlt_err)?;
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
/// A layer whose vertices carry z writes them into the WKB as its third ordinate.
/// They are grid values in raw tile coordinates, and elevations in metres once transformed.
///
/// `tms`: when True (the default), treat `y` as TMS convention (y=0 at south,
/// used by `OpenMapTiles` / `MBTiles`). Set to False for XYZ / slippy-map tiles
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
        let decoded = match lazy_layer {
            Layer::Tag01(layer) => layer.decode_all(&mut dec).map_err(mlt_err)?,
            Layer::Tag02(layer) => layer.decode_all(&mut dec).map_err(mlt_err)?.into_layer(),
            Layer::Unknown(_) | _ => {
                return Err(PyValueError::new_err(
                    "unsupported layer tag (expected 0x01 or 0x02)",
                ));
            }
        };
        let extent = decoded.extent().get();
        let xf = match (z, x, y) {
            (Some(z), Some(x), Some(y)) => Some(TileTransform::from_zxy(z, x, y, extent, tms)?),
            _ => None,
        };
        result.push(MltLayer {
            name: decoded.name().to_string(),
            extent,
            z_step: decoded.geometry_values().z_step().map(ZStep::exponent),
            features: build_features(py, &decoded, xf)?,
        });
    }

    Ok(result)
}

/// Decode an MLT binary blob and return `GeoJSON` as a string.
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
        .filter_map(|l| l.name().map(str::to_string))
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

    use mlt_core::{Decoder, GeometryValues, ParsedLayer};

    use super::*;

    fn first_layer01<'a, 'b>(layers: &'b [ParsedLayer<'a>]) -> &'b ParsedLayer01<'a> {
        let ParsedLayer::Tag01(l) = &layers[0] else {
            panic!("first layer should be 0x01")
        };
        l
    }

    fn geom_to_wkb(
        geom: &GeometryValues,
        index: usize,
        xf: Option<TileTransform>,
    ) -> MltResult<Vec<u8>> {
        geom32_to_wkb(&geom.to_geojson(index)?, xf, None)
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
        let l = first_layer01(&decoded);
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

        let l = first_layer01(&decoded);
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

        let l = first_layer01(&decoded);
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

        let l = first_layer01(&decoded);
        let geom = l.geometry_values();

        let wkb = geom_to_wkb(geom, 0, None).expect("geom_to_wkb should succeed");
        assert!(wkb.len() >= 5);
        let wkb_type = u32::from_le_bytes([wkb[1], wkb[2], wkb[3], wkb[4]]);
        assert_eq!(
            wkb_type, 2,
            "line fixture should produce WKB type 2 (LineString)"
        );
    }
}
