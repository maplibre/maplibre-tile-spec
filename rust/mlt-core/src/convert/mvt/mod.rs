//! Convert MVT data to/from [`FeatureCollection`] / [`TileLayer`]

mod vector_tile;

use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};

use geo_types::{
    Coord, Geometry as Geom, Geometry, LineString, MultiLineString, MultiPoint, MultiPolygon,
    Point, Polygon,
};
use mvt_reader::Reader;
use mvt_reader::feature::{Feature as MvtFeature, Value as MvtValue};
use prost::Message as _;
use serde_json::{Number, Value};
use zigzag::ZigZag;

use self::vector_tile as vt;
use crate::decoder::{PropValue, TileFeature, TileLayer};
use crate::geojson::{Feature, FeatureCollection};
use crate::{MltError, MltResult};

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

/// Parse MVT binary data and convert each layer to a row-oriented [`TileLayer`].
///
/// Each MVT layer becomes one [`TileLayer`].  Property column types are inferred
/// from all features in the layer: the first non-null value seen for each column
/// determines its type, with `I64`+`U64` widened to `I64` and `F32`+`F64` widened
/// to `F64`; all other type conflicts fall back to `Str`.
pub fn mvt_to_tile_layers(data: Vec<u8>) -> MltResult<Vec<TileLayer>> {
    read_mvt_layers(data)?
        .into_iter()
        .map(mvt_layer_to_tile)
        .collect()
}

fn mvt_layer_to_tile(layer: MvtLayer) -> MltResult<TileLayer> {
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

    Ok(TileLayer {
        name: layer.name,
        extent: layer.extent,
        property_names: col_names,
        features: tile_features,
    })
}

fn coord(c: impl AsRef<Coord<f32>>) -> Coord<i32> {
    let c = c.as_ref();
    #[expect(clippy::cast_possible_truncation)]
    Coord {
        x: c.x.round() as i32,
        y: c.y.round() as i32,
    }
}

fn convert_geometry(geom: &Geom<f32>) -> MltResult<Geometry<i32>> {
    Ok(match geom {
        Geom::Point(v) => Geometry::<i32>::Point(Point(coord(v))),
        Geom::MultiPoint(v) => {
            Geometry::<i32>::MultiPoint(MultiPoint(v.iter().map(|p| Point(coord(p))).collect()))
        }
        Geom::LineString(v) => {
            Geometry::<i32>::LineString(LineString(v.coords().map(coord).collect()))
        }
        Geom::MultiLineString(v) => Geometry::<i32>::MultiLineString(MultiLineString(
            v.iter()
                .map(|ls| LineString(ls.coords().map(coord).collect()))
                .collect(),
        )),
        Geom::Polygon(v) => Geometry::<i32>::Polygon(convert_polygon(v)),
        Geom::MultiPolygon(v) => {
            Geometry::<i32>::MultiPolygon(MultiPolygon(v.iter().map(convert_polygon).collect()))
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

// ── TileLayer → MVT ───────────────────────────────────────────────────────────

const MVT_MOVE_TO: u32 = 1;
const MVT_LINE_TO: u32 = 2;
const MVT_CLOSE_PATH: u32 = 7;

/// Encode row-oriented [`TileLayer`]s as MVT (Mapbox Vector Tile) bytes.
///
/// Geometry rings whose first vertex equals their last are written without the
/// closing duplicate — MVT's `ClosePath` command closes the ring implicitly.
pub fn tile_layers_to_mvt(layers: Vec<TileLayer>) -> MltResult<Vec<u8>> {
    let tile = vt::Tile {
        layers: layers
            .into_iter()
            .map(encode_layer)
            .collect::<MltResult<_>>()?,
    };
    Ok(tile.encode_to_vec())
}

fn encode_layer(layer: TileLayer) -> MltResult<vt::tile::Layer> {
    let mut values: Vec<vt::tile::Value> = Vec::new();
    let mut value_index = ValueIndex::default();

    let mut features = Vec::with_capacity(layer.features.len());
    for feat in layer.features {
        let geometry = encode_geometry(&feat.geometry)?;
        let geom_type = geom_type(&feat.geometry);

        let mut tags: Vec<u32> = Vec::new();
        for (col_idx, prop) in feat.properties.into_iter().enumerate() {
            let Some(mvt_value) = prop_to_mvt_value(prop) else {
                continue;
            };
            let key_idx = u32::try_from(col_idx)?;
            let value_idx = intern_value(&mut values, &mut value_index, mvt_value)?;
            tags.push(key_idx);
            tags.push(value_idx);
        }

        features.push(vt::tile::Feature {
            id: feat.id,
            tags,
            r#type: Some(geom_type as i32),
            geometry,
        });
    }

    Ok(vt::tile::Layer {
        version: 2,
        name: layer.name,
        features,
        keys: layer.property_names,
        values,
        extent: Some(layer.extent),
    })
}

/// Per-layer dedup table for [`vt::tile::Value`]. Strings are interned in a
/// separate map keyed by `&str` so the hit path doesn't clone the candidate
/// string just to hash it.
#[derive(Default)]
struct ValueIndex {
    strings: HashMap<String, u32>,
    scalars: HashMap<ScalarKey, u32>,
}

/// Hashable / comparable form of every non-string [`vt::tile::Value`] field.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
enum ScalarKey {
    Float(u32),
    Double(u64),
    Int(i64),
    UInt(u64),
    SInt(i64),
    Bool(bool),
}

fn intern_value(
    values: &mut Vec<vt::tile::Value>,
    index: &mut ValueIndex,
    value: vt::tile::Value,
) -> MltResult<u32> {
    if let Some(s) = &value.string_value {
        if let Some(&idx) = index.strings.get(s.as_str()) {
            return Ok(idx);
        }
        let idx = u32::try_from(values.len())?;
        let key = s.clone();
        values.push(value);
        index.strings.insert(key, idx);
        return Ok(idx);
    }
    match index.scalars.entry(scalar_key(&value)) {
        Entry::Occupied(o) => Ok(*o.get()),
        Entry::Vacant(v) => {
            let idx = u32::try_from(values.len())?;
            values.push(value);
            v.insert(idx);
            Ok(idx)
        }
    }
}

/// `prop_to_mvt_value` always sets exactly one field, and strings are handled
/// upstream of this call.
fn scalar_key(v: &vt::tile::Value) -> ScalarKey {
    if let Some(f) = v.float_value {
        ScalarKey::Float(f.to_bits())
    } else if let Some(d) = v.double_value {
        ScalarKey::Double(d.to_bits())
    } else if let Some(i) = v.int_value {
        ScalarKey::Int(i)
    } else if let Some(u) = v.uint_value {
        ScalarKey::UInt(u)
    } else if let Some(s) = v.sint_value {
        ScalarKey::SInt(s)
    } else if let Some(b) = v.bool_value {
        ScalarKey::Bool(b)
    } else {
        unreachable!("vt::tile::Value with no field set")
    }
}

fn prop_to_mvt_value(prop: PropValue) -> Option<vt::tile::Value> {
    let mut v = vt::tile::Value::default();
    match prop {
        PropValue::Bool(Some(b)) => v.bool_value = Some(b),
        PropValue::I8(Some(i)) => v.sint_value = Some(i64::from(i)),
        PropValue::U8(Some(u)) => v.uint_value = Some(u64::from(u)),
        PropValue::I32(Some(i)) => v.sint_value = Some(i64::from(i)),
        PropValue::U32(Some(u)) => v.uint_value = Some(u64::from(u)),
        PropValue::I64(Some(i)) => v.sint_value = Some(i),
        PropValue::U64(Some(u)) => v.uint_value = Some(u),
        PropValue::F32(Some(f)) => v.float_value = Some(f),
        PropValue::F64(Some(f)) => v.double_value = Some(f),
        PropValue::Str(Some(s)) => v.string_value = Some(s),
        _ => return None,
    }
    Some(v)
}

fn geom_type(g: &Geometry<i32>) -> vt::tile::GeomType {
    match g {
        Geometry::Point(_) | Geometry::MultiPoint(_) => vt::tile::GeomType::Point,
        Geometry::LineString(_) | Geometry::MultiLineString(_) => vt::tile::GeomType::Linestring,
        Geometry::Polygon(_) | Geometry::MultiPolygon(_) => vt::tile::GeomType::Polygon,
        _ => vt::tile::GeomType::Unknown,
    }
}

/// Accumulator for MVT command/parameter integers, tracking the running cursor
/// used by delta-encoded coordinates.
struct GeomBuf {
    out: Vec<u32>,
    cursor: Coord<i32>,
}

impl GeomBuf {
    fn new() -> Self {
        Self {
            out: Vec::new(),
            cursor: Coord { x: 0, y: 0 },
        }
    }

    fn push_command(&mut self, id: u32, count: u32) {
        debug_assert!(id <= 7);
        self.out.push((count << 3) | id);
    }

    /// Emit one MVT command followed by its zigzag-delta parameters.
    fn push_points(&mut self, cmd: u32, len: u32, points: impl IntoIterator<Item = Coord<i32>>) {
        self.push_command(cmd, len);
        for p in points {
            self.out
                .push(<i32 as ZigZag>::encode(p.x.wrapping_sub(self.cursor.x)));
            self.out
                .push(<i32 as ZigZag>::encode(p.y.wrapping_sub(self.cursor.y)));
            self.cursor = p;
        }
    }

    fn close_path(&mut self) {
        self.push_command(MVT_CLOSE_PATH, 1);
    }
}

/// Returns the ring without its trailing closing vertex if it is explicitly closed.
fn unclose(ring: &[Coord<i32>]) -> &[Coord<i32>] {
    if ring.len() >= 2 && ring[0] == ring[ring.len() - 1] {
        &ring[..ring.len() - 1]
    } else {
        ring
    }
}

fn encode_geometry(g: &Geometry<i32>) -> MltResult<Vec<u32>> {
    let mut buf = GeomBuf::new();
    match g {
        Geometry::Point(p) => encode_points(&mut buf, std::iter::once(p.0))?,
        Geometry::MultiPoint(mp) => encode_points(&mut buf, mp.0.iter().map(|p| p.0))?,
        Geometry::LineString(ls) => encode_linestring(&mut buf, &ls.0)?,
        Geometry::MultiLineString(mls) => {
            for ls in &mls.0 {
                encode_linestring(&mut buf, &ls.0)?;
            }
        }
        Geometry::Polygon(poly) => encode_polygon(&mut buf, poly)?,
        Geometry::MultiPolygon(mpoly) => {
            for poly in &mpoly.0 {
                encode_polygon(&mut buf, poly)?;
            }
        }
        _ => {
            return Err(MltError::BadMvtGeometry(
                "unsupported geometry variant for MVT encoding",
            ));
        }
    }
    Ok(buf.out)
}

fn encode_points(
    buf: &mut GeomBuf,
    points: impl ExactSizeIterator<Item = Coord<i32>>,
) -> MltResult<()> {
    let len = u32::try_from(points.len())?;
    if len > 0 {
        buf.push_points(MVT_MOVE_TO, len, points);
    }
    Ok(())
}

fn encode_linestring(buf: &mut GeomBuf, coords: &[Coord<i32>]) -> MltResult<()> {
    let Some((first, rest)) = coords.split_first() else {
        return Ok(());
    };
    buf.push_points(MVT_MOVE_TO, 1, std::iter::once(*first));
    if !rest.is_empty() {
        buf.push_points(
            MVT_LINE_TO,
            u32::try_from(rest.len())?,
            rest.iter().copied(),
        );
    }
    Ok(())
}

fn encode_ring(buf: &mut GeomBuf, ring: &[Coord<i32>]) -> MltResult<()> {
    let pts = unclose(ring);
    if pts.is_empty() {
        return Ok(());
    }
    encode_linestring(buf, pts)?;
    buf.close_path();
    Ok(())
}

fn encode_polygon(buf: &mut GeomBuf, poly: &Polygon<i32>) -> MltResult<()> {
    for ring in std::iter::once(poly.exterior()).chain(poly.interiors()) {
        encode_ring(buf, &ring.0)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_yields_empty_output() {
        let bytes = tile_layers_to_mvt(Vec::new()).unwrap();
        let decoded = mvt_to_tile_layers(bytes).unwrap();
        assert!(decoded.is_empty());
    }

    /// `mvt-reader` surfaces every polygon as a `MultiPolygon`, and decodes
    /// `ClosePath` by repeating the first vertex; an input with the closing
    /// duplicate must therefore round-trip without growing extra vertices.
    #[test]
    fn ring_is_implicitly_closed() {
        use geo_types::{LineString, Polygon};
        let ring = vec![
            (0_i32, 0_i32).into(),
            (10, 0).into(),
            (10, 10).into(),
            (0, 10).into(),
            (0, 0).into(),
        ];
        let layer = TileLayer {
            name: "L".into(),
            extent: 4096,
            property_names: vec![],
            features: vec![TileFeature {
                id: Some(1),
                geometry: Geometry::Polygon(Polygon::new(LineString(ring), vec![])),
                properties: vec![],
            }],
        };
        let bytes = tile_layers_to_mvt(vec![layer]).unwrap();
        let back = mvt_to_tile_layers(bytes).unwrap();
        let Geometry::MultiPolygon(mp) = &back[0].features[0].geometry else {
            panic!(
                "expected multipolygon, got {:?}",
                back[0].features[0].geometry
            );
        };
        let p = &mp.0[0];
        assert_eq!(p.exterior().0.len(), 5);
        assert_eq!(p.exterior().0.first(), p.exterior().0.last());
    }
}
