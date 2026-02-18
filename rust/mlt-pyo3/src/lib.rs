use std::f64::consts::PI;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};

use mlt_nom::geojson::FeatureCollection;
use mlt_nom::v01::{DecodedGeometry, DecodedProperty, PropValue};

fn mlt_err(e: mlt_nom::MltError) -> PyErr {
    PyValueError::new_err(format!("MLT decode error: {e}"))
}

/// Affine transform from tile-local coords to EPSG:3857 meters.
#[derive(Clone, Copy)]
struct TileTransform {
    x_origin: f64,
    y_origin: f64,
    x_scale: f64,
    y_scale: f64,
}

impl TileTransform {
    /// Build a transform from tile z/x/y coordinates.
    ///
    /// `tms`: if true, y uses TMS convention (y=0 at south, used by OpenMapTiles
    /// and MBTiles). If false, y uses XYZ / slippy-map convention (y=0 at north,
    /// used by OSM tile servers).
    fn from_zxy(z: u32, x: u32, y: u32, extent: u32, tms: bool) -> Self {
        let n = f64::from(1_u32 << z);
        let circumference = 2.0 * PI * 6_378_137.0;
        let tile_size = circumference / n;
        let half = circumference / 2.0;

        // Convert TMS y to XYZ y if needed (y_xyz = 2^z - 1 - y_tms)
        let y_xyz = if tms {
            (1_u32 << z).saturating_sub(1).saturating_sub(y)
        } else {
            y
        };

        // In XYZ convention: y=0 is the north edge of the map.
        // The tile's north (top) edge in EPSG:3857 meters:
        let x_origin = f64::from(x) * tile_size - half;
        let y_origin = half - f64::from(y_xyz) * tile_size;

        let scale = tile_size / f64::from(extent);

        TileTransform {
            x_origin,
            y_origin,
            x_scale: scale,
            y_scale: -scale, // tile pixel-y grows downward, EPSG:3857 y grows upward
        }
    }

    fn apply(self, coord: [i32; 2]) -> [f64; 2] {
        [
            self.x_origin + f64::from(coord[0]) * self.x_scale,
            self.y_origin + f64::from(coord[1]) * self.y_scale,
        ]
    }
}

/// A decoded MLT feature with geometry, id, and properties.
#[pyclass]
struct MltFeature {
    #[pyo3(get)]
    id: Option<u64>,
    #[pyo3(get)]
    geometry_type: String,
    #[pyo3(get)]
    wkb: Py<PyBytes>,
    #[pyo3(get)]
    properties: Py<PyDict>,
}

/// A decoded MLT layer containing features.
#[pyclass]
struct MltLayer {
    #[pyo3(get)]
    name: String,
    #[pyo3(get)]
    extent: u32,
    #[pyo3(get)]
    features: Vec<Py<MltFeature>>,
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

fn geom_to_wkb(
    geom: &DecodedGeometry,
    index: usize,
    xf: Option<TileTransform>,
) -> Result<Vec<u8>, mlt_nom::MltError> {
    use mlt_nom::geojson::Geometry;

    let gj = geom.to_geojson(index)?;
    let mut buf = Vec::with_capacity(128);

    match gj {
        Geometry::Point(c) => {
            buf.push(0x01);
            push_u32(&mut buf, 1);
            push_coord(&mut buf, c, xf);
        }
        Geometry::LineString(coords) => {
            buf.push(0x01);
            push_u32(&mut buf, 2);
            push_u32(&mut buf, coords.len() as u32);
            for c in &coords {
                push_coord(&mut buf, *c, xf);
            }
        }
        Geometry::Polygon(rings) => {
            buf.push(0x01);
            push_u32(&mut buf, 3);
            push_u32(&mut buf, rings.len() as u32);
            for ring in &rings {
                push_u32(&mut buf, ring.len() as u32);
                for c in ring {
                    push_coord(&mut buf, *c, xf);
                }
            }
        }
        Geometry::MultiPoint(coords) => {
            buf.push(0x01);
            push_u32(&mut buf, 4);
            push_u32(&mut buf, coords.len() as u32);
            for c in &coords {
                buf.push(0x01);
                push_u32(&mut buf, 1);
                push_coord(&mut buf, *c, xf);
            }
        }
        Geometry::MultiLineString(lines) => {
            buf.push(0x01);
            push_u32(&mut buf, 5);
            push_u32(&mut buf, lines.len() as u32);
            for line in &lines {
                buf.push(0x01);
                push_u32(&mut buf, 2);
                push_u32(&mut buf, line.len() as u32);
                for c in line {
                    push_coord(&mut buf, *c, xf);
                }
            }
        }
        Geometry::MultiPolygon(polygons) => {
            buf.push(0x01);
            push_u32(&mut buf, 6);
            push_u32(&mut buf, polygons.len() as u32);
            for polygon in &polygons {
                buf.push(0x01);
                push_u32(&mut buf, 3);
                push_u32(&mut buf, polygon.len() as u32);
                for ring in polygon {
                    push_u32(&mut buf, ring.len() as u32);
                    for c in ring {
                        push_coord(&mut buf, *c, xf);
                    }
                }
            }
        }
    }

    Ok(buf)
}

fn prop_value_to_py(py: Python<'_>, pv: &PropValue, i: usize) -> PyObject {
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
            let val = prop_value_to_py(py, &p.values, i);
            prop_dict.set_item(&p.name, val)?;
        }

        let feat = Py::new(
            py,
            MltFeature {
                id,
                geometry_type: format!("{gt}"),
                wkb,
                properties: prop_dict.unbind(),
            },
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
#[pyfunction]
#[pyo3(signature = (data, z=None, x=None, y=None, tms=true))]
fn decode_mlt(
    py: Python<'_>,
    data: &[u8],
    z: Option<u32>,
    x: Option<u32>,
    y: Option<u32>,
    tms: bool,
) -> PyResult<Vec<MltLayer>> {
    let mut layers = mlt_nom::parse_layers(data).map_err(mlt_err)?;

    for layer in &mut layers {
        layer.decode_all().map_err(mlt_err)?;
    }

    let mut result = Vec::with_capacity(layers.len());

    for layer in &layers {
        let l = layer
            .as_layer01()
            .ok_or_else(|| PyValueError::new_err("unsupported layer tag (expected 0x01)"))?;

        let xf = match (z, x, y) {
            (Some(z), Some(x), Some(y)) => Some(TileTransform::from_zxy(z, x, y, l.extent, tms)),
            _ => None,
        };

        let geom = match &l.geometry {
            mlt_nom::v01::Geometry::Decoded(g) => g,
            _ => return Err(PyValueError::new_err("geometry not decoded")),
        };

        let ids = match &l.id {
            mlt_nom::v01::Id::Decoded(d) => d.0.as_deref(),
            mlt_nom::v01::Id::None => None,
            _ => return Err(PyValueError::new_err("id not decoded")),
        };

        let props: Vec<&DecodedProperty> = l
            .properties
            .iter()
            .map(|p| match p {
                mlt_nom::v01::Property::Decoded(d) => Ok(d),
                _ => Err(PyValueError::new_err("property not decoded")),
            })
            .collect::<PyResult<_>>()?;

        let features = build_features(py, geom, ids, &props, xf)?;

        result.push(MltLayer {
            name: l.name.to_string(),
            extent: l.extent,
            features,
        });
    }

    Ok(result)
}

/// Decode an MLT binary blob and return GeoJSON as a string.
#[pyfunction]
fn decode_mlt_to_geojson(data: &[u8]) -> PyResult<String> {
    let mut layers = mlt_nom::parse_layers(data).map_err(mlt_err)?;
    for layer in &mut layers {
        layer.decode_all().map_err(mlt_err)?;
    }
    let fc = FeatureCollection::from_layers(&layers).map_err(mlt_err)?;
    serde_json::to_string(&fc).map_err(|e| PyValueError::new_err(format!("JSON error: {e}")))
}

/// Return a list of layer names without fully decoding.
#[pyfunction]
fn list_layers(data: &[u8]) -> PyResult<Vec<String>> {
    let layers = mlt_nom::parse_layers(data).map_err(mlt_err)?;
    Ok(layers
        .iter()
        .filter_map(|l| l.as_layer01().map(|l| l.name.to_string()))
        .collect())
}

#[pymodule]
fn mlt_pyo3(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(decode_mlt, m)?)?;
    m.add_function(wrap_pyfunction!(decode_mlt_to_geojson, m)?)?;
    m.add_function(wrap_pyfunction!(list_layers, m)?)?;
    m.add_class::<MltLayer>()?;
    m.add_class::<MltFeature>()?;
    Ok(())
}
