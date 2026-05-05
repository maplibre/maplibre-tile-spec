//! Encode row-oriented [`TileLayer`]s as MVT (Mapbox Vector Tile) bytes.

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use geo_types::{Coord, Geometry, Polygon};
use prost::Message as _;
use zigzag::ZigZag;

use super::vector_tile as vt;
use crate::decoder::{PropValue, TileLayer};
use crate::{MltError, MltResult};

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
    use crate::decoder::TileFeature;
    use crate::mvt::mvt_to_tile_layers;

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
