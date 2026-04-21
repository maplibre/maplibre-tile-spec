//! MLT v2 experimental encoder and decoder.
//!
//! This module implements a minimal v2 wire format for round-trip experimentation.
//! The format uses tag `2` to distinguish v2 layers from v1 layers (tag `1`).
//!
//! Simplifications in this initial implementation:
//! - VarInt physical encoding only (no FastPFor, no Morton)
//! - ComponentwiseDelta + VarInt for vertex coordinates
//! - StrPlain (lengths + raw bytes) for strings (no FSST, no shared-dict)
//! - Own-presence bitfields for optional columns (no shared-presence / OptRef*)
//! - Pure single-geometry-type layers only (no mixed geometry)

use geo_types::{
    Coord, Geometry, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
};
use integer_encoding::{VarInt as _, VarIntWriter as _};
use zigzag::ZigZag as _;

use crate::MltError;
use crate::MltResult;
use crate::codecs::varint::parse_varint;
use crate::codecs::zigzag::encode_componentwise_delta_vec2s;
use crate::decoder::{GeometryType, PropValue, TileFeature, TileLayer01};

// ── Encoding byte bit positions ───────────────────────────────────────────────
// bit  7: has_explicit_count
// bits 6-4: logical  (0=None, 1=Delta, 2=CwDelta, 3=Rle, 4=DeltaRle, 5=Morton)
// bits 3-2: physical (0=None-noLen, 1=None-withLen, 2=VarInt, 3=FastPFor128)
// bits 1-0: reserved (0)

/// `logical=None, physical=VarInt, count from context`
const ENC_VARINT: u8 = 0x08;
/// `logical=None, physical=VarInt, explicit count follows`
const ENC_VARINT_EXPL: u8 = 0x88;
/// `logical=CwDelta, physical=VarInt, explicit count follows`
const ENC_CWDELTA_VARINT_EXPL: u8 = 0xA8;
/// `logical=None, physical=None-noLen, explicit count follows`
const ENC_RAW_EXPL: u8 = 0x80;

// ── Geometry layout byte values ───────────────────────────────────────────────

/// MLT v2 geometry section layout codes.
///
/// Encodes which geometry streams are present and in what order.
/// Dict and tessellation variants are excluded from this initial implementation.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GeoLayout {
    /// Types, Vertices
    Points = 0,
    /// Types, GeoLengths, Vertices
    MultiPoints = 2,
    /// Types, PartLengths, Vertices
    Lines = 4,
    /// Types, GeoLengths, PartLengths, Vertices
    MultiLines = 6,
    /// Types, PartLengths, RingLengths, Vertices
    Polygons = 8,
    /// Types, GeoLengths, PartLengths, RingLengths, Vertices
    MultiPolygons = 10,
}

impl TryFrom<u8> for GeoLayout {
    type Error = MltError;
    fn try_from(v: u8) -> MltResult<Self> {
        match v {
            0 => Ok(Self::Points),
            2 => Ok(Self::MultiPoints),
            4 => Ok(Self::Lines),
            6 => Ok(Self::MultiLines),
            8 => Ok(Self::Polygons),
            10 => Ok(Self::MultiPolygons),
            _ => Err(MltError::NotImplemented("unsupported v2 geometry layout")),
        }
    }
}

// ── Column type codes (reuse v1 codes for backward clarity) ──────────────────

// IDs
const COL_ID: u8 = 0;
const COL_OPT_ID: u8 = 1;
// Scalars (mirror v1 ColumnType values)
const COL_BOOL: u8 = 10;
const COL_OPT_BOOL: u8 = 11;
const COL_I8: u8 = 12;
const COL_OPT_I8: u8 = 13;
const COL_U8: u8 = 14;
const COL_OPT_U8: u8 = 15;
const COL_I32: u8 = 16;
const COL_OPT_I32: u8 = 17;
const COL_U32: u8 = 18;
const COL_OPT_U32: u8 = 19;
const COL_I64: u8 = 20;
const COL_OPT_I64: u8 = 21;
const COL_U64: u8 = 22;
const COL_OPT_U64: u8 = 23;
const COL_F32: u8 = 24;
const COL_OPT_F32: u8 = 25;
const COL_F64: u8 = 26;
const COL_OPT_F64: u8 = 27;
// Strings: StrPlain / OptStrPlain
const COL_STR: u8 = 28;
const COL_OPT_STR: u8 = 29;

// ── Encoder ───────────────────────────────────────────────────────────────────

impl TileLayer01 {
    /// Encode this layer to the MLT **v2** experimental wire format.
    ///
    /// Returns a complete framed record: `[varint(body_len+1)][tag=2][body…]`
    /// ready to be concatenated with other layers in a tile.
    ///
    /// Returns an empty slice for empty layers (no features).
    pub fn encode_v2(&self) -> MltResult<Vec<u8>> {
        if self.features.is_empty() {
            return Ok(Vec::new());
        }

        let feature_count = self.features.len();

        // ── Geometry ──────────────────────────────────────────────────────────
        let geom = collect_geometry(&self.features)?;

        // ── IDs ───────────────────────────────────────────────────────────────
        let has_ids = self.features.iter().any(|f| f.id.is_some());
        let all_ids: Vec<Option<u64>> = if has_ids {
            self.features.iter().map(|f| f.id).collect()
        } else {
            Vec::new()
        };

        // ── Property columns ──────────────────────────────────────────────────
        let prop_count = self.property_names.len();

        // ── Build body ────────────────────────────────────────────────────────
        let mut body: Vec<u8> = Vec::new();

        // name
        let name_bytes = self.name.as_bytes();
        body.write_varint(name_bytes.len() as u32)
            .map_err(MltError::from)?;
        body.extend_from_slice(name_bytes);

        // extent
        body.write_varint(self.extent).map_err(MltError::from)?;

        // feature_count  (v2 addition)
        body.write_varint(feature_count as u32)
            .map_err(MltError::from)?;

        // ── Geometry section ──────────────────────────────────────────────────
        body.push(geom.layout as u8);
        write_geometry_streams(
            &mut body,
            &geom.types,
            geom.geo_lengths.as_deref(),
            geom.part_lengths.as_deref(),
            geom.ring_lengths.as_deref(),
            &geom.vertices,
            feature_count,
        )?;

        // ── Columns ───────────────────────────────────────────────────────────
        let col_count = u32::try_from(usize::from(has_ids) + prop_count)?;
        body.write_varint(col_count).map_err(MltError::from)?;

        // ID column
        if has_ids {
            write_id_column(&mut body, &all_ids, feature_count)?;
        }

        // Property columns
        for (i, name) in self.property_names.iter().enumerate() {
            let vals: Vec<&PropValue> = self.features.iter().map(|f| &f.properties[i]).collect();
            write_property_column(&mut body, name, &vals, feature_count)?;
        }

        // ── Frame: [varint(body_len+1)][tag=2][body] ──────────────────────────
        let size = u32::try_from(body.len() + 1)?; // +1 for the tag byte
        let mut out = Vec::with_capacity(5 + 1 + body.len());
        out.write_varint(size).map_err(MltError::from)?;
        out.push(2_u8); // tag = 2
        out.extend_from_slice(&body);

        Ok(out)
    }
}

// ── Geometry collection ───────────────────────────────────────────────────────

struct CollectedGeometry {
    layout: GeoLayout,
    /// Geometry type per feature (u8 repr of GeometryType)
    types: Vec<u32>,
    /// Number of component geometries per multi-geometry feature
    geo_lengths: Option<Vec<u32>>,
    /// Number of rings per polygon, or vertices per line
    part_lengths: Option<Vec<u32>>,
    /// Number of vertices per ring
    ring_lengths: Option<Vec<u32>>,
    /// Flat vertex buffer `[x0, y0, x1, y1, …]`
    vertices: Vec<i32>,
}

fn collect_geometry(features: &[TileFeature]) -> MltResult<CollectedGeometry> {
    let has_multi = features.iter().any(|f| {
        matches!(
            f.geometry,
            Geometry::MultiPoint(_) | Geometry::MultiLineString(_) | Geometry::MultiPolygon(_)
        )
    });
    let has_ring = features
        .iter()
        .any(|f| matches!(f.geometry, Geometry::Polygon(_) | Geometry::MultiPolygon(_)));
    let has_line = features.iter().any(|f| {
        matches!(
            f.geometry,
            Geometry::LineString(_) | Geometry::MultiLineString(_)
        )
    });
    let has_part = has_line || has_ring;

    let layout = match (has_multi, has_ring, has_part) {
        (_, true, _) if has_multi => GeoLayout::MultiPolygons,
        (_, true, _) => GeoLayout::Polygons,
        (_, false, true) if has_multi => GeoLayout::MultiLines,
        (_, false, true) => GeoLayout::Lines,
        (true, false, false) => GeoLayout::MultiPoints,
        (false, false, false) => GeoLayout::Points,
    };

    let mut types = Vec::with_capacity(features.len());
    let mut geo_lengths: Option<Vec<u32>> = if has_multi { Some(Vec::new()) } else { None };
    let mut part_lengths: Option<Vec<u32>> = if has_part { Some(Vec::new()) } else { None };
    let mut ring_lengths: Option<Vec<u32>> = if has_ring { Some(Vec::new()) } else { None };
    let mut vertices: Vec<i32> = Vec::new();

    for feat in features {
        push_feature_geometry(
            &feat.geometry,
            &mut types,
            &mut geo_lengths,
            &mut part_lengths,
            &mut ring_lengths,
            &mut vertices,
        )?;
    }

    Ok(CollectedGeometry {
        layout,
        types,
        geo_lengths,
        part_lengths,
        ring_lengths,
        vertices,
    })
}

fn push_coords(coords: impl Iterator<Item = Coord<i32>>, vertices: &mut Vec<i32>) -> u32 {
    let start = vertices.len();
    for c in coords {
        vertices.push(c.x);
        vertices.push(c.y);
    }
    u32::try_from((vertices.len() - start) / 2).expect("vertex count fits u32")
}

/// For polygon rings: GeoJSON rings include the closing coordinate (first == last),
/// which MLT does not store. Drop the last coordinate if it equals the first.
fn ring_vertex_count(ring: &LineString<i32>, vertices: &mut Vec<i32>) -> u32 {
    let coords = &ring.0;
    let has_closing = coords.len() >= 2 && coords.first() == coords.last();
    let coords_to_write = if has_closing {
        &coords[..coords.len() - 1]
    } else {
        &coords[..]
    };
    push_coords(coords_to_write.iter().copied(), vertices)
}

fn push_feature_geometry(
    geom: &Geometry<i32>,
    types: &mut Vec<u32>,
    geo_lengths: &mut Option<Vec<u32>>,
    part_lengths: &mut Option<Vec<u32>>,
    ring_lengths: &mut Option<Vec<u32>>,
    vertices: &mut Vec<i32>,
) -> MltResult<()> {
    match geom {
        Geometry::Point(pt) => {
            types.push(GeometryType::Point as u32);
            vertices.push(pt.0.x);
            vertices.push(pt.0.y);
        }
        Geometry::MultiPoint(mp) => {
            types.push(GeometryType::MultiPoint as u32);
            let count = push_coords(mp.0.iter().map(|p| p.0), vertices);
            geo_lengths.as_mut().map(|v| v.push(count));
        }
        Geometry::LineString(ls) => {
            types.push(GeometryType::LineString as u32);
            let count = push_coords(ls.0.iter().copied(), vertices);
            part_lengths.as_mut().map(|v| v.push(count));
        }
        Geometry::MultiLineString(mls) => {
            types.push(GeometryType::MultiLineString as u32);
            geo_lengths.as_mut().map(|v| v.push(mls.0.len() as u32));
            for ls in &mls.0 {
                let count = push_coords(ls.0.iter().copied(), vertices);
                part_lengths.as_mut().map(|v| v.push(count));
            }
        }
        Geometry::Polygon(poly) => {
            types.push(GeometryType::Polygon as u32);
            let ring_count = 1 + poly.interiors().len();
            part_lengths.as_mut().map(|v| v.push(ring_count as u32));
            let ext_count = ring_vertex_count(poly.exterior(), vertices);
            ring_lengths.as_mut().map(|v| v.push(ext_count));
            for hole in poly.interiors() {
                let hole_count = ring_vertex_count(hole, vertices);
                ring_lengths.as_mut().map(|v| v.push(hole_count));
            }
        }
        Geometry::MultiPolygon(mp) => {
            types.push(GeometryType::MultiPolygon as u32);
            geo_lengths.as_mut().map(|v| v.push(mp.0.len() as u32));
            for poly in &mp.0 {
                let ring_count = 1 + poly.interiors().len();
                part_lengths.as_mut().map(|v| v.push(ring_count as u32));
                let ext_count = ring_vertex_count(poly.exterior(), vertices);
                ring_lengths.as_mut().map(|v| v.push(ext_count));
                for hole in poly.interiors() {
                    let hole_count = ring_vertex_count(hole, vertices);
                    ring_lengths.as_mut().map(|v| v.push(hole_count));
                }
            }
        }
        _other => {
            return Err(MltError::NotImplemented(
                "v2 encoder: unsupported geometry type (GeometryCollection not supported)",
            ));
        }
    }
    Ok(())
}

// ── Stream write helpers ──────────────────────────────────────────────────────

/// Write a VarInt-encoded `u32` stream with implicit count (= `feature_count`).
fn write_u32_stream_implicit(buf: &mut Vec<u8>, values: &[u32]) -> MltResult<()> {
    buf.push(ENC_VARINT);
    let mut tmp = Vec::new();
    for &v in values {
        tmp.write_varint(v).map_err(MltError::from)?;
    }
    buf.write_varint(tmp.len() as u32).map_err(MltError::from)?;
    buf.extend_from_slice(&tmp);
    Ok(())
}

/// Write a VarInt-encoded `u32` stream with explicit count.
fn write_u32_stream_explicit(buf: &mut Vec<u8>, values: &[u32]) -> MltResult<()> {
    buf.push(ENC_VARINT_EXPL);
    buf.write_varint(values.len() as u32)
        .map_err(MltError::from)?;
    let mut tmp = Vec::new();
    for &v in values {
        tmp.write_varint(v).map_err(MltError::from)?;
    }
    buf.write_varint(tmp.len() as u32).map_err(MltError::from)?;
    buf.extend_from_slice(&tmp);
    Ok(())
}

/// Write a VarInt-encoded `u64` stream with implicit count.
fn write_u64_stream_implicit(buf: &mut Vec<u8>, values: &[u64]) -> MltResult<()> {
    buf.push(ENC_VARINT);
    let mut tmp = Vec::new();
    for &v in values {
        tmp.write_varint(v).map_err(MltError::from)?;
    }
    buf.write_varint(tmp.len() as u32).map_err(MltError::from)?;
    buf.extend_from_slice(&tmp);
    Ok(())
}

/// Write a ZigZag+VarInt encoded `i32` stream with implicit count.
fn write_i32_stream_implicit(buf: &mut Vec<u8>, values: &[i32]) -> MltResult<()> {
    buf.push(ENC_VARINT);
    let mut tmp = Vec::new();
    for &v in values {
        tmp.write_varint(i32::encode(v)).map_err(MltError::from)?;
    }
    buf.write_varint(tmp.len() as u32).map_err(MltError::from)?;
    buf.extend_from_slice(&tmp);
    Ok(())
}

/// Write a ZigZag+VarInt encoded `i64` stream with implicit count.
fn write_i64_stream_implicit(buf: &mut Vec<u8>, values: &[i64]) -> MltResult<()> {
    buf.push(ENC_VARINT);
    let mut tmp = Vec::new();
    for &v in values {
        tmp.write_varint(i64::encode(v)).map_err(MltError::from)?;
    }
    buf.write_varint(tmp.len() as u32).map_err(MltError::from)?;
    buf.extend_from_slice(&tmp);
    Ok(())
}

/// Write a fixed-width raw byte stream with explicit count (number of data bytes).
fn write_raw_bytes(buf: &mut Vec<u8>, data: &[u8]) -> MltResult<()> {
    buf.push(ENC_RAW_EXPL);
    buf.write_varint(data.len() as u32)
        .map_err(MltError::from)?;
    buf.extend_from_slice(data);
    Ok(())
}

/// Write vertex data using ComponentwiseDelta + VarInt with explicit count (vertex pairs).
fn write_cwdelta_vertices(buf: &mut Vec<u8>, flat_verts: &[i32]) -> MltResult<()> {
    debug_assert!(
        flat_verts.len() % 2 == 0,
        "vertex buffer must have even length"
    );
    buf.push(ENC_CWDELTA_VARINT_EXPL);
    let pair_count = flat_verts.len() / 2;
    buf.write_varint(pair_count as u32)
        .map_err(MltError::from)?;

    let mut encoded: Vec<u32> = Vec::new();
    encode_componentwise_delta_vec2s(flat_verts, &mut encoded);

    let mut tmp = Vec::new();
    for v in &encoded {
        tmp.write_varint(*v).map_err(MltError::from)?;
    }
    buf.write_varint(tmp.len() as u32).map_err(MltError::from)?;
    buf.extend_from_slice(&tmp);
    Ok(())
}

fn write_geometry_streams(
    buf: &mut Vec<u8>,
    types: &[u32],
    geo_lengths: Option<&[u32]>,
    part_lengths: Option<&[u32]>,
    ring_lengths: Option<&[u32]>,
    vertices: &[i32],
    feature_count: usize,
) -> MltResult<()> {
    debug_assert_eq!(types.len(), feature_count);
    // Types stream: count = feature_count (implicit)
    write_u32_stream_implicit(buf, types)?;
    if let Some(g) = geo_lengths {
        write_u32_stream_explicit(buf, g)?;
    }
    if let Some(p) = part_lengths {
        write_u32_stream_explicit(buf, p)?;
    }
    if let Some(r) = ring_lengths {
        write_u32_stream_explicit(buf, r)?;
    }
    write_cwdelta_vertices(buf, vertices)
}

// ── Presence bitfield ─────────────────────────────────────────────────────────

/// Write `ceil(feature_count / 8)` bytes of a packed presence bitfield.
///
/// Bit `i` (LSB-first within each byte) is set when `values[i]` is non-null.
fn write_presence(buf: &mut Vec<u8>, null_mask: &[bool]) {
    let byte_count = null_mask.len().div_ceil(8);
    let start = buf.len();
    buf.resize(start + byte_count, 0u8);
    for (i, &present) in null_mask.iter().enumerate() {
        if present {
            buf[start + i / 8] |= 1 << (i % 8);
        }
    }
}

// ── ID column ─────────────────────────────────────────────────────────────────

fn write_id_column(buf: &mut Vec<u8>, ids: &[Option<u64>], _feature_count: usize) -> MltResult<()> {
    let any_null = ids.iter().any(|id| id.is_none());

    if any_null {
        // OptId: presence bitfield + data for present values
        buf.push(COL_OPT_ID);
        let presence: Vec<bool> = ids.iter().map(|id| id.is_some()).collect();
        write_presence(buf, &presence);
        let values: Vec<u64> = ids.iter().filter_map(|id| *id).collect();
        write_u64_stream_implicit(buf, &values)?;
    } else {
        // Id: all values present
        buf.push(COL_ID);
        let values: Vec<u64> = ids.iter().map(|id| id.unwrap_or(0)).collect();
        write_u64_stream_implicit(buf, &values)?;
    }

    Ok(())
}

// ── Property column ───────────────────────────────────────────────────────────

fn write_property_column(
    buf: &mut Vec<u8>,
    name: &str,
    values: &[&PropValue],
    feature_count: usize,
) -> MltResult<()> {
    debug_assert_eq!(values.len(), feature_count);

    // Determine the dominant type and whether any value is null
    let any_null = values.iter().any(|v| is_null(v));

    let col_type = prop_column_type(values[0], any_null);
    buf.push(col_type);

    // name
    let name_bytes = name.as_bytes();
    buf.write_varint(name_bytes.len() as u32)
        .map_err(MltError::from)?;
    buf.extend_from_slice(name_bytes);

    // presence bitfield (for optional columns)
    if any_null {
        let presence: Vec<bool> = values.iter().map(|v| !is_null(v)).collect();
        write_presence(buf, &presence);
    }

    // data streams
    write_prop_data(buf, values, any_null)
}

fn is_null(v: &PropValue) -> bool {
    match v {
        PropValue::Bool(o) => o.is_none(),
        PropValue::I8(o) => o.is_none(),
        PropValue::U8(o) => o.is_none(),
        PropValue::I32(o) => o.is_none(),
        PropValue::U32(o) => o.is_none(),
        PropValue::I64(o) => o.is_none(),
        PropValue::U64(o) => o.is_none(),
        PropValue::F32(o) => o.is_none(),
        PropValue::F64(o) => o.is_none(),
        PropValue::Str(o) => o.is_none(),
    }
}

fn prop_column_type(sample: &PropValue, any_null: bool) -> u8 {
    let (non_opt, opt) = match sample {
        PropValue::Bool(_) => (COL_BOOL, COL_OPT_BOOL),
        PropValue::I8(_) => (COL_I8, COL_OPT_I8),
        PropValue::U8(_) => (COL_U8, COL_OPT_U8),
        PropValue::I32(_) => (COL_I32, COL_OPT_I32),
        PropValue::U32(_) => (COL_U32, COL_OPT_U32),
        PropValue::I64(_) => (COL_I64, COL_OPT_I64),
        PropValue::U64(_) => (COL_U64, COL_OPT_U64),
        PropValue::F32(_) => (COL_F32, COL_OPT_F32),
        PropValue::F64(_) => (COL_F64, COL_OPT_F64),
        PropValue::Str(_) => (COL_STR, COL_OPT_STR),
    };
    if any_null { opt } else { non_opt }
}

fn write_prop_data(buf: &mut Vec<u8>, values: &[&PropValue], any_null: bool) -> MltResult<()> {
    match values[0] {
        PropValue::Bool(_) => {
            let data: Vec<u8> = values
                .iter()
                .filter(|v| !is_null(v))
                .map(|v| prop_bool(v) as u8)
                .collect();
            write_raw_bytes(buf, &data)?;
        }
        PropValue::I8(_) => {
            let data: Vec<u8> = values
                .iter()
                .filter(|v| !is_null(v))
                .map(|v| i8::encode(prop_i8(v)) as u8)
                .collect();
            write_raw_bytes(buf, &data)?;
        }
        PropValue::U8(_) => {
            let data: Vec<u8> = values
                .iter()
                .filter(|v| !is_null(v))
                .map(|v| prop_u8(v))
                .collect();
            write_raw_bytes(buf, &data)?;
        }
        PropValue::I32(_) => {
            let data: Vec<i32> = values
                .iter()
                .filter(|v| !is_null(v))
                .map(|v| prop_i32(v))
                .collect();
            let implicit_count = !any_null;
            if implicit_count {
                write_i32_stream_implicit(buf, &data)?;
            } else {
                write_i32_stream_as_explicit(buf, &data)?;
            }
        }
        PropValue::U32(_) => {
            let data: Vec<u32> = values
                .iter()
                .filter(|v| !is_null(v))
                .map(|v| prop_u32(v))
                .collect();
            let implicit_count = !any_null;
            if implicit_count {
                write_u32_stream_implicit(buf, &data)?;
            } else {
                write_u32_popcount(buf, &data)?;
            }
        }
        PropValue::I64(_) => {
            let data: Vec<i64> = values
                .iter()
                .filter(|v| !is_null(v))
                .map(|v| prop_i64(v))
                .collect();
            if !any_null {
                write_i64_stream_implicit(buf, &data)?;
            } else {
                write_i64_stream_as_explicit(buf, &data)?;
            }
        }
        PropValue::U64(_) => {
            let data: Vec<u64> = values
                .iter()
                .filter(|v| !is_null(v))
                .map(|v| prop_u64(v))
                .collect();
            if !any_null {
                write_u64_stream_implicit(buf, &data)?;
            } else {
                write_u64_popcount(buf, &data)?;
            }
        }
        PropValue::F32(_) => {
            let mut data = Vec::new();
            for v in values.iter().filter(|v| !is_null(v)) {
                data.extend_from_slice(&prop_f32(v).to_le_bytes());
            }
            write_raw_bytes(buf, &data)?;
        }
        PropValue::F64(_) => {
            let mut data = Vec::new();
            for v in values.iter().filter(|v| !is_null(v)) {
                data.extend_from_slice(&prop_f64(v).to_le_bytes());
            }
            write_raw_bytes(buf, &data)?;
        }
        PropValue::Str(_) => {
            let strings: Vec<Option<&str>> = values
                .iter()
                .map(|v| match v {
                    PropValue::Str(Some(s)) => Some(s.as_str()),
                    _ => None,
                })
                .collect();
            write_string_col_data(buf, &strings, any_null)?;
        }
    }
    Ok(())
}

/// Write string column data streams (lengths + raw bytes).
/// Only the non-null strings are written (count = popcount when any_null, else feature_count).
fn write_string_col_data(
    buf: &mut Vec<u8>,
    strings: &[Option<&str>],
    any_null: bool,
) -> MltResult<()> {
    let present: Vec<&str> = strings.iter().filter_map(|s| *s).collect();
    let lengths: Vec<u32> = present.iter().map(|s| s.len() as u32).collect();
    let all_bytes: Vec<u8> = present.iter().flat_map(|s| s.as_bytes()).copied().collect();

    // Lengths stream: count = popcount (implicit when any_null, else feature_count)
    if any_null {
        // count = popcount(presence), which the decoder can compute → use implicit
        write_u32_stream_implicit(buf, &lengths)?;
    } else {
        write_u32_stream_implicit(buf, &lengths)?;
    }
    // String data: explicit count = total bytes
    write_raw_bytes(buf, &all_bytes)?;

    Ok(())
}

// ── VarInt stream helpers for optional columns (count = popcount) ─────────────

fn write_i32_stream_as_explicit(buf: &mut Vec<u8>, values: &[i32]) -> MltResult<()> {
    buf.push(ENC_VARINT); // count = popcount (implicit for optional columns)
    let mut tmp = Vec::new();
    for &v in values {
        tmp.write_varint(i32::encode(v)).map_err(MltError::from)?;
    }
    buf.write_varint(tmp.len() as u32).map_err(MltError::from)?;
    buf.extend_from_slice(&tmp);
    Ok(())
}

fn write_i64_stream_as_explicit(buf: &mut Vec<u8>, values: &[i64]) -> MltResult<()> {
    buf.push(ENC_VARINT); // count = popcount
    let mut tmp = Vec::new();
    for &v in values {
        tmp.write_varint(i64::encode(v)).map_err(MltError::from)?;
    }
    buf.write_varint(tmp.len() as u32).map_err(MltError::from)?;
    buf.extend_from_slice(&tmp);
    Ok(())
}

fn write_u32_popcount(buf: &mut Vec<u8>, values: &[u32]) -> MltResult<()> {
    buf.push(ENC_VARINT); // count = popcount
    let mut tmp = Vec::new();
    for &v in values {
        tmp.write_varint(v).map_err(MltError::from)?;
    }
    buf.write_varint(tmp.len() as u32).map_err(MltError::from)?;
    buf.extend_from_slice(&tmp);
    Ok(())
}

fn write_u64_popcount(buf: &mut Vec<u8>, values: &[u64]) -> MltResult<()> {
    buf.push(ENC_VARINT); // count = popcount
    let mut tmp = Vec::new();
    for &v in values {
        tmp.write_varint(v).map_err(MltError::from)?;
    }
    buf.write_varint(tmp.len() as u32).map_err(MltError::from)?;
    buf.extend_from_slice(&tmp);
    Ok(())
}

// ── PropValue field accessors ─────────────────────────────────────────────────

fn prop_bool(v: &PropValue) -> bool {
    if let PropValue::Bool(Some(b)) = v {
        *b
    } else {
        false
    }
}
fn prop_i8(v: &PropValue) -> i8 {
    if let PropValue::I8(Some(b)) = v {
        *b
    } else {
        0
    }
}
fn prop_u8(v: &PropValue) -> u8 {
    if let PropValue::U8(Some(b)) = v {
        *b
    } else {
        0
    }
}
fn prop_i32(v: &PropValue) -> i32 {
    if let PropValue::I32(Some(b)) = v {
        *b
    } else {
        0
    }
}
fn prop_u32(v: &PropValue) -> u32 {
    if let PropValue::U32(Some(b)) = v {
        *b
    } else {
        0
    }
}
fn prop_i64(v: &PropValue) -> i64 {
    if let PropValue::I64(Some(b)) = v {
        *b
    } else {
        0
    }
}
fn prop_u64(v: &PropValue) -> u64 {
    if let PropValue::U64(Some(b)) = v {
        *b
    } else {
        0
    }
}
fn prop_f32(v: &PropValue) -> f32 {
    if let PropValue::F32(Some(b)) = v {
        *b
    } else {
        0.0
    }
}
fn prop_f64(v: &PropValue) -> f64 {
    if let PropValue::F64(Some(b)) = v {
        *b
    } else {
        0.0
    }
}

// ── Decoder ───────────────────────────────────────────────────────────────────

/// Decode an MLT **v2** layer body (the bytes after the `tag=2` byte).
///
/// Returns a [`TileLayer01`] that can be used by the rest of the MLT tooling.
pub fn decode_v2_layer(data: &[u8]) -> MltResult<TileLayer01> {
    // ── Header ────────────────────────────────────────────────────────────────
    let (data, name_len) = parse_varint::<u32>(data)?;
    let name_len = name_len as usize;
    if data.len() < name_len {
        return Err(MltError::BufferUnderflow(name_len as u32, data.len()));
    }
    let name = std::str::from_utf8(&data[..name_len])?.to_string();
    let data = &data[name_len..];

    let (data, extent) = parse_varint::<u32>(data)?;
    let (data, feature_count) = parse_varint::<u32>(data)?;
    let feature_count = feature_count as usize;

    // ── Geometry section ──────────────────────────────────────────────────────
    if data.is_empty() {
        return Err(MltError::MissingGeometry);
    }
    let layout = GeoLayout::try_from(data[0])?;
    let data = &data[1..];

    let (data, geometries) = decode_geometry(data, layout, feature_count)?;

    // ── Column count ──────────────────────────────────────────────────────────
    let (mut data, col_count) = parse_varint::<u32>(data)?;

    // ── Columns ───────────────────────────────────────────────────────────────
    let mut feature_ids: Vec<Option<u64>> = vec![None; feature_count];
    let mut property_names: Vec<String> = Vec::new();
    let mut property_columns: Vec<Vec<PropValue>> = Vec::new();
    let mut presence_groups: Vec<Vec<bool>> = Vec::new();

    for _ in 0..col_count {
        if data.is_empty() {
            return Err(MltError::BufferUnderflow(1, 0));
        }
        let col_type = data[0];
        data = &data[1..];

        match col_type {
            COL_ID => {
                let (new_data, ids) = decode_id_column(data, false, feature_count)?;
                data = new_data;
                for (i, id) in ids.into_iter().enumerate() {
                    feature_ids[i] = Some(id);
                }
            }
            COL_OPT_ID => {
                let (new_data, presence, ids) = decode_opt_id_column(data, feature_count)?;
                data = new_data;
                presence_groups.push(presence.clone());
                let mut iter = ids.into_iter();
                for (i, &present) in presence.iter().enumerate() {
                    if present {
                        feature_ids[i] = iter.next();
                    }
                }
            }
            COL_BOOL | COL_OPT_BOOL | COL_I8 | COL_OPT_I8 | COL_U8 | COL_OPT_U8 | COL_I32
            | COL_OPT_I32 | COL_U32 | COL_OPT_U32 | COL_I64 | COL_OPT_I64 | COL_U64
            | COL_OPT_U64 | COL_F32 | COL_OPT_F32 | COL_F64 | COL_OPT_F64 | COL_STR
            | COL_OPT_STR => {
                let (new_data, col_name, col_values) =
                    decode_property_column(data, col_type, feature_count, &mut presence_groups)?;
                data = new_data;
                property_names.push(col_name);
                property_columns.push(col_values);
            }
            _ => {
                return Err(MltError::NotImplemented("unsupported v2 column type"));
            }
        }
    }

    // ── Assemble TileLayer01 ──────────────────────────────────────────────────
    let prop_count = property_names.len();
    let features = geometries
        .into_iter()
        .enumerate()
        .map(|(i, geom)| {
            let id = feature_ids[i];
            let properties = (0..prop_count)
                .map(|j| property_columns[j][i].clone())
                .collect();
            TileFeature {
                id,
                geometry: geom,
                properties,
            }
        })
        .collect();

    Ok(TileLayer01 {
        name,
        extent,
        property_names,
        features,
    })
}

// ── Geometry decoding ─────────────────────────────────────────────────────────

fn decode_geometry<'a>(
    data: &'a [u8],
    layout: GeoLayout,
    feature_count: usize,
) -> MltResult<(&'a [u8], Vec<Geometry<i32>>)> {
    // Types stream: count = feature_count (implicit)
    let (data, type_vals) = read_u32_stream(data, feature_count, false)?;
    let geo_types: Vec<GeometryType> = type_vals
        .iter()
        .map(|&v| GeometryType::try_from(v as u8).map_err(MltError::from))
        .collect::<MltResult<_>>()?;

    // Optional auxiliary streams, depending on layout
    let (data, geo_lengths) = if matches!(
        layout,
        GeoLayout::MultiPoints | GeoLayout::MultiLines | GeoLayout::MultiPolygons
    ) {
        let (d, v) = read_u32_stream(data, 0, true)?;
        (d, Some(v))
    } else {
        (data, None)
    };

    let (data, part_lengths) = if matches!(
        layout,
        GeoLayout::Lines | GeoLayout::MultiLines | GeoLayout::Polygons | GeoLayout::MultiPolygons
    ) {
        let (d, v) = read_u32_stream(data, 0, true)?;
        (d, Some(v))
    } else {
        (data, None)
    };

    let (data, ring_lengths) = if matches!(layout, GeoLayout::Polygons | GeoLayout::MultiPolygons) {
        let (d, v) = read_u32_stream(data, 0, true)?;
        (d, Some(v))
    } else {
        (data, None)
    };

    // Vertices stream: explicit count (vertex pairs)
    let (data, vertices) = read_cwdelta_stream(data)?;

    let geometries = reconstruct_geometries(
        layout,
        &geo_types,
        geo_lengths.as_deref(),
        part_lengths.as_deref(),
        ring_lengths.as_deref(),
        &vertices,
    )?;

    Ok((data, geometries))
}

fn reconstruct_geometries(
    layout: GeoLayout,
    types: &[GeometryType],
    geo_lengths: Option<&[u32]>,
    part_lengths: Option<&[u32]>,
    ring_lengths: Option<&[u32]>,
    vertices: &[i32],
) -> MltResult<Vec<Geometry<i32>>> {
    let mut geoms = Vec::with_capacity(types.len());
    let mut vert_pos: usize = 0; // current position in vertices (counting i32 elements)

    match layout {
        GeoLayout::Points => {
            for _ in types {
                let x = vertices[vert_pos];
                let y = vertices[vert_pos + 1];
                vert_pos += 2;
                geoms.push(Geometry::Point(Point(Coord { x, y })));
            }
        }
        GeoLayout::MultiPoints => {
            let geo_lens = geo_lengths.expect("MultiPoints requires GeoLengths");
            for &geo_len in geo_lens {
                let pts: Vec<Point<i32>> = (0..geo_len as usize)
                    .map(|_| {
                        let x = vertices[vert_pos];
                        let y = vertices[vert_pos + 1];
                        vert_pos += 2;
                        Point(Coord { x, y })
                    })
                    .collect();
                geoms.push(Geometry::MultiPoint(MultiPoint(pts)));
            }
        }
        GeoLayout::Lines => {
            let part_lens = part_lengths.expect("Lines requires PartLengths");
            for &part_len in part_lens {
                let coords: Vec<Coord<i32>> = (0..part_len as usize)
                    .map(|_| {
                        let c = Coord {
                            x: vertices[vert_pos],
                            y: vertices[vert_pos + 1],
                        };
                        vert_pos += 2;
                        c
                    })
                    .collect();
                geoms.push(Geometry::LineString(LineString(coords)));
            }
        }
        GeoLayout::MultiLines => {
            let geo_lens = geo_lengths.expect("MultiLines requires GeoLengths");
            let part_lens = part_lengths.expect("MultiLines requires PartLengths");
            let mut part_idx = 0;
            for &geo_len in geo_lens {
                let lines: Vec<LineString<i32>> = (0..geo_len as usize)
                    .map(|_| {
                        let part_len = part_lens[part_idx] as usize;
                        part_idx += 1;
                        let coords: Vec<Coord<i32>> = (0..part_len)
                            .map(|_| {
                                let c = Coord {
                                    x: vertices[vert_pos],
                                    y: vertices[vert_pos + 1],
                                };
                                vert_pos += 2;
                                c
                            })
                            .collect();
                        LineString(coords)
                    })
                    .collect();
                geoms.push(Geometry::MultiLineString(MultiLineString(lines)));
            }
        }
        GeoLayout::Polygons => {
            let part_lens = part_lengths.expect("Polygons requires PartLengths");
            let ring_lens = ring_lengths.expect("Polygons requires RingLengths");
            let mut ring_idx = 0;
            for &part_len in part_lens {
                let rings: Vec<LineString<i32>> = (0..part_len as usize)
                    .map(|_| {
                        let ring_len = ring_lens[ring_idx] as usize;
                        ring_idx += 1;
                        let mut coords: Vec<Coord<i32>> = (0..ring_len)
                            .map(|_| {
                                let c = Coord {
                                    x: vertices[vert_pos],
                                    y: vertices[vert_pos + 1],
                                };
                                vert_pos += 2;
                                c
                            })
                            .collect();
                        // Close the ring (MLT omits the closing vertex)
                        if let Some(&first) = coords.first() {
                            coords.push(first);
                        }
                        LineString(coords)
                    })
                    .collect();
                let mut ring_iter = rings.into_iter();
                let exterior = ring_iter.next().unwrap_or_else(|| LineString(vec![]));
                let holes: Vec<LineString<i32>> = ring_iter.collect();
                geoms.push(Geometry::Polygon(Polygon::new(exterior, holes)));
            }
        }
        GeoLayout::MultiPolygons => {
            let geo_lens = geo_lengths.expect("MultiPolygons requires GeoLengths");
            let part_lens = part_lengths.expect("MultiPolygons requires PartLengths");
            let ring_lens = ring_lengths.expect("MultiPolygons requires RingLengths");
            let mut part_idx = 0;
            let mut ring_idx = 0;
            for &geo_len in geo_lens {
                let polys: Vec<Polygon<i32>> = (0..geo_len as usize)
                    .map(|_| {
                        let part_len = part_lens[part_idx] as usize;
                        part_idx += 1;
                        let rings: Vec<LineString<i32>> = (0..part_len)
                            .map(|_| {
                                let ring_len = ring_lens[ring_idx] as usize;
                                ring_idx += 1;
                                let mut coords: Vec<Coord<i32>> = (0..ring_len)
                                    .map(|_| {
                                        let c = Coord {
                                            x: vertices[vert_pos],
                                            y: vertices[vert_pos + 1],
                                        };
                                        vert_pos += 2;
                                        c
                                    })
                                    .collect();
                                if let Some(&first) = coords.first() {
                                    coords.push(first);
                                }
                                LineString(coords)
                            })
                            .collect();
                        let mut ring_iter = rings.into_iter();
                        let exterior = ring_iter.next().unwrap_or_else(|| LineString(vec![]));
                        Polygon::new(exterior, ring_iter.collect())
                    })
                    .collect();
                geoms.push(Geometry::MultiPolygon(MultiPolygon(polys)));
            }
        }
    }

    Ok(geoms)
}

// ── Stream read helpers ───────────────────────────────────────────────────────

/// Read one encoding byte, returning (remaining, has_explicit_count, logical, physical).
fn read_enc_byte(data: &[u8]) -> MltResult<(&[u8], bool, u8, u8)> {
    if data.is_empty() {
        return Err(MltError::BufferUnderflow(1, 0));
    }
    let b = data[0];
    let has_explicit_count = (b & 0x80) != 0;
    let logical = (b >> 4) & 0x07;
    let physical = (b >> 2) & 0x03;
    Ok((&data[1..], has_explicit_count, logical, physical))
}

/// Read a VarInt u32 stream. If `explicit` is true, read the count from the stream;
/// otherwise use `implicit_count`. After reading count, reads byte_length and the data.
fn read_u32_stream(
    data: &[u8],
    implicit_count: usize,
    explicit: bool,
) -> MltResult<(&[u8], Vec<u32>)> {
    let (data, has_expl_count, logical, physical) = read_enc_byte(data)?;

    if logical != 0 {
        // Only None logical is supported here
        return Err(MltError::NotImplemented(
            "v2 decoder: unsupported logical encoding for integer stream",
        ));
    }
    if physical != 2 {
        return Err(MltError::NotImplemented(
            "v2 decoder: only VarInt physical encoding supported for integer stream",
        ));
    }

    let (data, count) = if has_expl_count || explicit {
        let (d, c) = parse_varint::<u32>(data)?;
        (d, c as usize)
    } else {
        (data, implicit_count)
    };

    // byte_length
    let (data, byte_len) = parse_varint::<u32>(data)?;
    let byte_len = byte_len as usize;
    if data.len() < byte_len {
        return Err(MltError::BufferUnderflow(byte_len as u32, data.len()));
    }
    let encoded = &data[..byte_len];
    let remaining = &data[byte_len..];

    let mut values = Vec::with_capacity(count);
    let mut pos = 0;
    while pos < byte_len {
        let (v, consumed) = u32::decode_var(&encoded[pos..])
            .ok_or(MltError::BufferUnderflow(4, encoded.len() - pos))?;
        values.push(v);
        pos += consumed;
    }

    Ok((remaining, values))
}

/// Read a raw-bytes stream (physical=None-noLen, logical=None, explicit count = byte count).
fn read_raw_bytes_stream(data: &[u8]) -> MltResult<(&[u8], Vec<u8>)> {
    let (data, has_expl_count, logical, physical) = read_enc_byte(data)?;

    if logical != 0 || physical != 0 || !has_expl_count {
        return Err(MltError::NotImplemented(
            "v2 decoder: unexpected encoding byte for raw bytes stream",
        ));
    }

    let (data, count) = parse_varint::<u32>(data)?;
    let count = count as usize;
    if data.len() < count {
        return Err(MltError::BufferUnderflow(count as u32, data.len()));
    }
    Ok((&data[count..], data[..count].to_vec()))
}

/// Read a ZigZag+VarInt i32 stream (count from context = explicit).
fn read_i32_stream(
    data: &[u8],
    implicit_count: usize,
    _has_any_null: bool,
) -> MltResult<(&[u8], Vec<i32>)> {
    let (data, has_expl_count, logical, physical) = read_enc_byte(data)?;

    if logical != 0 || physical != 2 {
        return Err(MltError::NotImplemented(
            "v2 decoder: only VarInt/ZigZag supported for i32 stream",
        ));
    }

    let (data, count) = if has_expl_count {
        let (d, c) = parse_varint::<u32>(data)?;
        (d, c as usize)
    } else {
        (data, implicit_count)
    };

    let (data, byte_len) = parse_varint::<u32>(data)?;
    let byte_len = byte_len as usize;
    if data.len() < byte_len {
        return Err(MltError::BufferUnderflow(byte_len as u32, data.len()));
    }
    let encoded = &data[..byte_len];
    let remaining = &data[byte_len..];

    let mut values = Vec::with_capacity(count);
    let mut pos = 0;
    while pos < byte_len {
        let (v, consumed) = u32::decode_var(&encoded[pos..])
            .ok_or(MltError::BufferUnderflow(4, encoded.len() - pos))?;
        values.push(i32::decode(v));
        pos += consumed;
    }

    Ok((remaining, values))
}

/// Read a ZigZag+VarInt i64 stream.
fn read_i64_stream(data: &[u8], implicit_count: usize) -> MltResult<(&[u8], Vec<i64>)> {
    let (data, _has_expl_count, logical, physical) = read_enc_byte(data)?;

    if logical != 0 || physical != 2 {
        return Err(MltError::NotImplemented(
            "v2 decoder: only VarInt/ZigZag supported for i64 stream",
        ));
    }

    let (data, byte_len) = parse_varint::<u32>(data)?;
    let byte_len = byte_len as usize;
    if data.len() < byte_len {
        return Err(MltError::BufferUnderflow(byte_len as u32, data.len()));
    }
    let encoded = &data[..byte_len];
    let remaining = &data[byte_len..];

    let mut values = Vec::with_capacity(implicit_count);
    let mut pos = 0;
    while pos < byte_len {
        let (v, consumed) = u64::decode_var(&encoded[pos..])
            .ok_or(MltError::BufferUnderflow(8, encoded.len() - pos))?;
        values.push(i64::decode(v));
        pos += consumed;
    }

    Ok((remaining, values))
}

/// Read a VarInt u64 stream.
fn read_u64_stream(data: &[u8], implicit_count: usize) -> MltResult<(&[u8], Vec<u64>)> {
    let (data, _has_expl_count, logical, physical) = read_enc_byte(data)?;

    if logical != 0 || physical != 2 {
        return Err(MltError::NotImplemented(
            "v2 decoder: only VarInt supported for u64 stream",
        ));
    }

    let (data, byte_len) = parse_varint::<u32>(data)?;
    let byte_len = byte_len as usize;
    if data.len() < byte_len {
        return Err(MltError::BufferUnderflow(byte_len as u32, data.len()));
    }
    let encoded = &data[..byte_len];
    let remaining = &data[byte_len..];

    let mut values = Vec::with_capacity(implicit_count);
    let mut pos = 0;
    while pos < byte_len {
        let (v, consumed) = u64::decode_var(&encoded[pos..])
            .ok_or(MltError::BufferUnderflow(8, encoded.len() - pos))?;
        values.push(v);
        pos += consumed;
    }

    Ok((remaining, values))
}

/// Read a ComponentwiseDelta + VarInt vertex stream (explicit count = vertex pairs).
fn read_cwdelta_stream(data: &[u8]) -> MltResult<(&[u8], Vec<i32>)> {
    let (data, has_expl_count, logical, physical) = read_enc_byte(data)?;

    if logical != 2 || physical != 2 {
        return Err(MltError::NotImplemented(
            "v2 decoder: only CwDelta+VarInt supported for vertex stream",
        ));
    }

    let (data, pair_count) = if has_expl_count {
        let (d, c) = parse_varint::<u32>(data)?;
        (d, c as usize)
    } else {
        return Err(MltError::NotImplemented(
            "v2 decoder: vertex stream must have explicit count",
        ));
    };

    let (data, byte_len) = parse_varint::<u32>(data)?;
    let byte_len = byte_len as usize;
    if data.len() < byte_len {
        return Err(MltError::BufferUnderflow(byte_len as u32, data.len()));
    }
    let encoded = &data[..byte_len];
    let remaining = &data[byte_len..];

    // Decode VarInt u32 values
    let coord_count = pair_count * 2;
    let mut u32_vals = Vec::with_capacity(coord_count);
    let mut pos = 0;
    while pos < byte_len {
        let (v, consumed) = u32::decode_var(&encoded[pos..])
            .ok_or(MltError::BufferUnderflow(4, encoded.len() - pos))?;
        u32_vals.push(v);
        pos += consumed;
    }

    // Apply inverse CwDelta
    let mut result = Vec::with_capacity(coord_count);
    let mut last_x: i32 = 0;
    let mut last_y: i32 = 0;
    for chunk in u32_vals.chunks_exact(2) {
        last_x = last_x.wrapping_add(i32::decode(chunk[0]));
        last_y = last_y.wrapping_add(i32::decode(chunk[1]));
        result.push(last_x);
        result.push(last_y);
    }

    Ok((remaining, result))
}

// ── Presence bitfield ─────────────────────────────────────────────────────────

/// Read `ceil(feature_count / 8)` bytes of presence bitfield and return a `Vec<bool>`.
fn read_presence(data: &[u8], feature_count: usize) -> MltResult<(&[u8], Vec<bool>)> {
    let byte_count = feature_count.div_ceil(8);
    if data.len() < byte_count {
        return Err(MltError::BufferUnderflow(byte_count as u32, data.len()));
    }
    let mut presence = Vec::with_capacity(feature_count);
    for i in 0..feature_count {
        presence.push((data[i / 8] >> (i % 8)) & 1 == 1);
    }
    Ok((&data[byte_count..], presence))
}

// ── ID column decoding ────────────────────────────────────────────────────────

fn decode_id_column<'a>(
    data: &'a [u8],
    _optional: bool,
    feature_count: usize,
) -> MltResult<(&'a [u8], Vec<u64>)> {
    read_u64_stream(data, feature_count)
}

fn decode_opt_id_column<'a>(
    data: &'a [u8],
    feature_count: usize,
) -> MltResult<(&'a [u8], Vec<bool>, Vec<u64>)> {
    let (data, presence) = read_presence(data, feature_count)?;
    let popcount = presence.iter().filter(|&&p| p).count();
    let (data, ids) = read_u64_stream(data, popcount)?;
    Ok((data, presence, ids))
}

// ── Property column decoding ──────────────────────────────────────────────────

fn decode_property_column<'a>(
    data: &'a [u8],
    col_type: u8,
    feature_count: usize,
    presence_groups: &mut Vec<Vec<bool>>,
) -> MltResult<(&'a [u8], String, Vec<PropValue>)> {
    // name
    let (data, name_len) = parse_varint::<u32>(data)?;
    let name_len = name_len as usize;
    if data.len() < name_len {
        return Err(MltError::BufferUnderflow(name_len as u32, data.len()));
    }
    let name = std::str::from_utf8(&data[..name_len])?.to_string();
    let data = &data[name_len..];

    let is_optional = matches!(
        col_type,
        COL_OPT_BOOL
            | COL_OPT_I8
            | COL_OPT_U8
            | COL_OPT_I32
            | COL_OPT_U32
            | COL_OPT_I64
            | COL_OPT_U64
            | COL_OPT_F32
            | COL_OPT_F64
            | COL_OPT_STR
    );

    let (data, presence) = if is_optional {
        let (d, p) = read_presence(data, feature_count)?;
        presence_groups.push(p.clone());
        (d, Some(p))
    } else {
        (data, None)
    };

    let popcount = presence
        .as_ref()
        .map_or(feature_count, |p| p.iter().filter(|&&b| b).count());

    let (data, values) = decode_column_data(data, col_type, feature_count, popcount, &presence)?;

    Ok((data, name, values))
}

fn decode_column_data<'a>(
    data: &'a [u8],
    col_type: u8,
    feature_count: usize,
    popcount: usize,
    presence: &Option<Vec<bool>>,
) -> MltResult<(&'a [u8], Vec<PropValue>)> {
    match col_type {
        COL_BOOL | COL_OPT_BOOL => {
            let (data, bytes) = read_raw_bytes_stream(data)?;
            let values = expand_with_presence(
                bytes.iter().map(|&b| PropValue::Bool(Some(b != 0))),
                presence,
                feature_count,
                PropValue::Bool(None),
            );
            Ok((data, values))
        }
        COL_I8 | COL_OPT_I8 => {
            let (data, bytes) = read_raw_bytes_stream(data)?;
            let values = expand_with_presence(
                bytes
                    .iter()
                    .map(|&b| PropValue::I8(Some(i8::decode(b as u32 as u8)))),
                presence,
                feature_count,
                PropValue::I8(None),
            );
            Ok((data, values))
        }
        COL_U8 | COL_OPT_U8 => {
            let (data, bytes) = read_raw_bytes_stream(data)?;
            let values = expand_with_presence(
                bytes.into_iter().map(|b| PropValue::U8(Some(b))),
                presence,
                feature_count,
                PropValue::U8(None),
            );
            Ok((data, values))
        }
        COL_I32 | COL_OPT_I32 => {
            let any_null = col_type == COL_OPT_I32;
            let (data, ints) = read_i32_stream(data, popcount, any_null)?;
            let values = expand_with_presence(
                ints.into_iter().map(|v| PropValue::I32(Some(v))),
                presence,
                feature_count,
                PropValue::I32(None),
            );
            Ok((data, values))
        }
        COL_U32 | COL_OPT_U32 => {
            let (data, ints) = read_u32_stream(data, popcount, false)?;
            let values = expand_with_presence(
                ints.into_iter().map(|v| PropValue::U32(Some(v))),
                presence,
                feature_count,
                PropValue::U32(None),
            );
            Ok((data, values))
        }
        COL_I64 | COL_OPT_I64 => {
            let (data, ints) = read_i64_stream(data, popcount)?;
            let values = expand_with_presence(
                ints.into_iter().map(|v| PropValue::I64(Some(v))),
                presence,
                feature_count,
                PropValue::I64(None),
            );
            Ok((data, values))
        }
        COL_U64 | COL_OPT_U64 => {
            let (data, ints) = read_u64_stream(data, popcount)?;
            let values = expand_with_presence(
                ints.into_iter().map(|v| PropValue::U64(Some(v))),
                presence,
                feature_count,
                PropValue::U64(None),
            );
            Ok((data, values))
        }
        COL_F32 | COL_OPT_F32 => {
            let (data, bytes) = read_raw_bytes_stream(data)?;
            let floats: Vec<f32> = bytes
                .chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect();
            let values = expand_with_presence(
                floats.into_iter().map(|v| PropValue::F32(Some(v))),
                presence,
                feature_count,
                PropValue::F32(None),
            );
            Ok((data, values))
        }
        COL_F64 | COL_OPT_F64 => {
            let (data, bytes) = read_raw_bytes_stream(data)?;
            let floats: Vec<f64> = bytes
                .chunks_exact(8)
                .map(|c| f64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
                .collect();
            let values = expand_with_presence(
                floats.into_iter().map(|v| PropValue::F64(Some(v))),
                presence,
                feature_count,
                PropValue::F64(None),
            );
            Ok((data, values))
        }
        COL_STR | COL_OPT_STR => decode_string_column(data, popcount, presence, feature_count),
        _ => Err(MltError::NotImplemented("v2 decoder: unknown column type")),
    }
}

fn decode_string_column<'a>(
    data: &'a [u8],
    popcount: usize,
    presence: &Option<Vec<bool>>,
    feature_count: usize,
) -> MltResult<(&'a [u8], Vec<PropValue>)> {
    // Lengths stream (count = popcount, implicit)
    let (data, lengths) = read_u32_stream(data, popcount, false)?;
    // String data (explicit byte count)
    let (data, str_bytes) = read_raw_bytes_stream(data)?;

    // Reconstruct strings from lengths
    let mut strings: Vec<Option<String>> = Vec::with_capacity(popcount);
    let mut offset = 0;
    for &len in &lengths {
        let len = len as usize;
        let s = std::str::from_utf8(&str_bytes[offset..offset + len])?.to_string();
        strings.push(Some(s));
        offset += len;
    }

    let values = expand_with_presence(
        strings.into_iter().map(|s| PropValue::Str(s)),
        presence,
        feature_count,
        PropValue::Str(None),
    );
    Ok((data, values))
}

/// Expand a stream of non-null values back to `feature_count` values by
/// inserting `null_val` for absent features according to the presence bitfield.
fn expand_with_presence<I: Iterator<Item = PropValue>>(
    present_values: I,
    presence: &Option<Vec<bool>>,
    feature_count: usize,
    null_val: PropValue,
) -> Vec<PropValue> {
    match presence {
        None => present_values.collect(),
        Some(pres) => {
            let mut result = Vec::with_capacity(feature_count);
            let mut iter = present_values;
            for &present in pres {
                if present {
                    result.push(iter.next().unwrap_or(null_val.clone()));
                } else {
                    result.push(null_val.clone());
                }
            }
            result
        }
    }
}
