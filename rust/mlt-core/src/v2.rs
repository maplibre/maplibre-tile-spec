//! MLT v2 experimental encoder and decoder.
//!
//! This module implements a minimal v2 wire format for round-trip experimentation.
//! The format uses tag `2` to distinguish v2 layers from v1 layers (tag `1`).
//!
//! Encoding strategies (each stream picks the smallest result automatically):
//!
//! - **Feature ordering**: tries `Unsorted`, `SpatialMorton`, and `SpatialHilbert`, keeping
//!   the ordering that produces the smallest encoded tile.  Mirrors the v1 optimizer.
//! - **Geometry types stream**: tries VarInt and RLE.
//!   For single-type layers (all Points, all Lines …) RLE reduces from N bytes to ~3 bytes.
//! - **Vertex data**: ComponentwiseDelta applied first, then tries:
//!   - VarInt (1 byte per small delta)
//!   - FastPFor128 LE (bit-packing in blocks of 128; wins for layers with many vertices)
//! - **Integer ID columns**: tries plain VarInt and Delta+VarInt.
//!   Sequential OSM IDs encode as 1-byte deltas each.
//! - **String columns**: tries three encodings, keeps smallest:
//!   - StrPlain / OptStrPlain: per-value byte lengths + raw UTF-8 bytes.
//!   - StrDict / OptStrDict: deduplicated dictionary + per-feature indices.
//!     Best for low-cardinality columns (road class, surface type …).
//!   - StrFsst / OptStrFsst: FSST symbol-table compression.
//!     Best for high-cardinality columns (street names, place names …).
//! - **Presence**: raw packed bitfields for optional columns (no bool-RLE header).
//! - **Shared dictionaries**: string columns with similar content are grouped using MinHash
//!   similarity (same algorithm as v1).  Each group encodes one shared dictionary corpus
//!   (plain or FSST-compressed, whichever is smaller) followed by per-child index streams.
//!   Mirrors v1's `ColumnType::SharedDict` optimization.  Note: property column order in the
//!   decoded layer may differ from the original when grouping is applied (shared-dict children
//!   are emitted together before the next non-grouped column).

use std::collections::HashMap;

use fastpfor::{AnyLenCodec as _, FastPFor128};
use geo_types::{
    Coord, Geometry, LineString, MultiLineString, MultiPoint, MultiPolygon, Point, Polygon,
};
use integer_encoding::{VarInt as _, VarIntWriter as _};
use zigzag::ZigZag as _;

use crate::codecs::fsst::compress_fsst;
use crate::codecs::varint::parse_varint;
use crate::codecs::zigzag::encode_componentwise_delta_vec2s;
use crate::decoder::{GeometryType, PropValue, TileFeature, TileLayer01};
use crate::encoder::{
    EncoderConfig, SortStrategy, StagedSharedDict, StagedSharedDictItem, StringGroup,
    group_string_properties, spatial_sort_likely_to_help,
};
use crate::utils::{BinarySerializer as _, parse_string};
use crate::{MltError, MltResult};

/// Minimum feature count above which the bounding-box heuristic is applied
/// before deciding whether to try spatial sort trials.  Mirrors v1's threshold.
const SORT_TRIAL_THRESHOLD: usize = 512;

// ── Encoding byte bit positions ───────────────────────────────────────────────
// bit  7: has_explicit_count
// bits 6-4: logical  (0=None, 1=Delta, 2=CwDelta, 3=Rle, 4=DeltaRle, 5=Morton)
// bits 3-2: physical (0=None-noLen, 1=None-withLen, 2=VarInt, 3=FastPFor128)
// bits 1-0: reserved (0)

/// `logical=None, physical=VarInt, count from context`
const ENC_VARINT: u8 = 0x08;
/// `logical=None, physical=VarInt, explicit count follows`
const ENC_VARINT_EXPL: u8 = 0x88;
/// `logical=Delta, physical=VarInt, count from context`
const ENC_DELTA_VARINT: u8 = 0x18;
/// `logical=CwDelta, physical=VarInt, explicit count follows`
const ENC_CWDELTA_VARINT_EXPL: u8 = 0xA8;
/// `logical=CwDelta, physical=FastPFor128, explicit count follows`
const ENC_CWDELTA_FP128_EXPL: u8 = 0xAC;
/// `logical=Rle, physical=reserved (00), count from context; byte_length always follows`
const ENC_RLE: u8 = 0x30;
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

/// Wire column type byte for MLT v2 property columns.
///
/// Each variant maps to its discriminant value on the wire.
/// Optional variants encode the same data as their non-optional counterpart
/// but are preceded by a packed presence bitfield (except `OptStrDict`, which
/// uses index 0 to signal null).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum ColType {
    // IDs
    Id = 0,
    OptId = 1,
    // Scalars (mirror v1 ColumnType values)
    Bool = 10,
    OptBool = 11,
    I8 = 12,
    OptI8 = 13,
    U8 = 14,
    OptU8 = 15,
    I32 = 16,
    OptI32 = 17,
    U32 = 18,
    OptU32 = 19,
    I64 = 20,
    OptI64 = 21,
    U64 = 22,
    OptU64 = 23,
    F32 = 24,
    OptF32 = 25,
    F64 = 26,
    OptF64 = 27,
    // Strings: plain lengths + raw bytes (presence bitfield when optional)
    Str = 28,
    OptStr = 29,
    // Strings: deduplicated dictionary; index 0 = null for OptStrDict
    StrDict = 30,
    OptStrDict = 31,
    // Strings: FSST-compressed corpus (presence bitfield when optional)
    StrFsst = 32,
    OptStrFsst = 33,
    // Strings: cross-column shared dictionary (plain corpus)
    StrSharedDict = 34,
    // Strings: cross-column shared dictionary (FSST corpus)
    StrSharedDictFsst = 35,
}

impl TryFrom<u8> for ColType {
    type Error = MltError;
    fn try_from(v: u8) -> MltResult<Self> {
        match v {
            0 => Ok(Self::Id),
            1 => Ok(Self::OptId),
            10 => Ok(Self::Bool),
            11 => Ok(Self::OptBool),
            12 => Ok(Self::I8),
            13 => Ok(Self::OptI8),
            14 => Ok(Self::U8),
            15 => Ok(Self::OptU8),
            16 => Ok(Self::I32),
            17 => Ok(Self::OptI32),
            18 => Ok(Self::U32),
            19 => Ok(Self::OptU32),
            20 => Ok(Self::I64),
            21 => Ok(Self::OptI64),
            22 => Ok(Self::U64),
            23 => Ok(Self::OptU64),
            24 => Ok(Self::F32),
            25 => Ok(Self::OptF32),
            26 => Ok(Self::F64),
            27 => Ok(Self::OptF64),
            28 => Ok(Self::Str),
            29 => Ok(Self::OptStr),
            30 => Ok(Self::StrDict),
            31 => Ok(Self::OptStrDict),
            32 => Ok(Self::StrFsst),
            33 => Ok(Self::OptStrFsst),
            34 => Ok(Self::StrSharedDict),
            35 => Ok(Self::StrSharedDictFsst),
            _ => Err(MltError::NotImplemented("unsupported v2 column type")),
        }
    }
}

/// Flag byte for each child within a shared-dict column: bit 0 = child has null values.
const CHILD_OPTIONAL: u8 = 0x01;

// ── Encoder ───────────────────────────────────────────────────────────────────

impl TileLayer01 {
    /// Encode this layer to the MLT **v2** experimental wire format.
    ///
    /// Respects the same [`EncoderConfig`] flags as the v1 encoder:
    /// - `try_spatial_morton_sort` / `try_spatial_hilbert_sort`: spatial sort trials.
    /// - `allow_fsst`: FSST string compression.
    /// - `allow_fpf`: FastPFor128 vertex compression.
    /// - `allow_shared_dict`: cross-column shared dictionary grouping.
    ///
    /// Returns a complete framed record: `[varint(body_len+1)][tag=2][body…]`
    /// ready to be concatenated with other layers in a tile.
    ///
    /// Returns an empty `Vec` for layers with no features.
    pub fn encode_v2(&self, cfg: EncoderConfig) -> MltResult<Vec<u8>> {
        if self.features.is_empty() {
            return Ok(Vec::new());
        }

        // Baseline: unsorted feature order.
        let mut best = self.encode_v2_with_sort(cfg)?;

        // Apply the same spatial-sort heuristic as the v1 optimizer.
        let try_spatial =
            self.features.len() < SORT_TRIAL_THRESHOLD || spatial_sort_likely_to_help(self);

        if try_spatial {
            let strategies = [
                (SortStrategy::SpatialMorton, cfg.try_spatial_morton_sort),
                (SortStrategy::SpatialHilbert, cfg.try_spatial_hilbert_sort),
            ];
            for (strategy, enabled) in strategies {
                if !enabled {
                    continue;
                }
                let mut candidate = self.clone();
                candidate.sort(strategy);
                let bytes = candidate.encode_v2_with_sort(cfg)?;
                if bytes.len() < best.len() {
                    best = bytes;
                }
            }
        }

        Ok(best)
    }

    /// Encode this layer to v2 bytes with features in their **current** order.
    ///
    /// Called by [`encode_v2`] once per sort-strategy trial.
    fn encode_v2_with_sort(&self, cfg: EncoderConfig) -> MltResult<Vec<u8>> {
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

        // ── Collect column chunks ─────────────────────────────────────────────
        // We must know the total column count before writing it, so collect all
        // encoded chunks first, then write col_count + chunks together.
        //
        // For shared-dict groups, we compare the shared-dict chunk against encoding
        // each member individually and keep whichever is smaller.

        // Group similar string columns with MinHash (mirrors v1); skip when disabled.
        let groups = if cfg.allow_shared_dict {
            group_string_properties(self)
        } else {
            Vec::new()
        };
        let col_to_group: HashMap<usize, &StringGroup> = groups
            .iter()
            .flat_map(|g| g.columns.iter().map(move |(_, i)| (*i, g)))
            .collect();
        let mut group_start: HashMap<usize, &StringGroup> =
            groups.iter().map(|g| (g.columns[0].1, g)).collect();

        let mut col_chunks: Vec<Vec<u8>> = Vec::new();

        // ID column (single chunk)
        if has_ids {
            let mut chunk = Vec::new();
            write_id_column(&mut chunk, &all_ids, feature_count)?;
            col_chunks.push(chunk);
        }

        // Property columns
        for (col_idx, name) in self.property_names.iter().enumerate() {
            if let Some(g) = group_start.remove(&col_idx) {
                // Try shared-dict encoding (all group members as one encoded column).
                let shared_chunk = encode_shared_dict_chunk(g, &self.features, cfg.allow_fsst)?;

                // Try individual encoding for every group member as a baseline.
                let individual_chunks: Vec<Vec<u8>> = g
                    .columns
                    .iter()
                    .map(|(_, c_idx)| {
                        let full_name = &self.property_names[*c_idx];
                        let vals: Vec<&PropValue> = self
                            .features
                            .iter()
                            .map(|f| &f.properties[*c_idx])
                            .collect();
                        let mut chunk = Vec::new();
                        write_property_column(
                            &mut chunk,
                            full_name,
                            &vals,
                            feature_count,
                            cfg.allow_fsst,
                        )?;
                        Ok(chunk)
                    })
                    .collect::<MltResult<_>>()?;
                let individual_total: usize = individual_chunks.iter().map(|c| c.len()).sum();

                if shared_chunk.len() <= individual_total {
                    col_chunks.push(shared_chunk);
                } else {
                    col_chunks.extend(individual_chunks);
                }
            } else if !col_to_group.contains_key(&col_idx) {
                let vals: Vec<&PropValue> = self
                    .features
                    .iter()
                    .map(|f| &f.properties[col_idx])
                    .collect();
                let mut chunk = Vec::new();
                write_property_column(&mut chunk, name, &vals, feature_count, cfg.allow_fsst)?;
                col_chunks.push(chunk);
            }
            // else: column absorbed into a shared-dict group handled above.
        }

        // ── Build body ────────────────────────────────────────────────────────
        let mut body: Vec<u8> = Vec::new();

        // name
        body.write_string(&self.name).map_err(MltError::from)?;

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
            cfg.allow_fpf,
        )?;

        // ── Columns ───────────────────────────────────────────────────────────
        body.write_varint(col_chunks.len() as u32)
            .map_err(MltError::from)?;
        for chunk in col_chunks {
            body.extend_from_slice(&chunk);
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

// ── Stream body builders (return raw bytes, no enc_byte prefix) ───────────────

/// Build the VarInt body for a `u32` slice (no enc_byte, no count, no length).
fn build_varint_body_u32(values: &[u32]) -> MltResult<Vec<u8>> {
    let mut tmp = Vec::new();
    for &v in values {
        tmp.write_varint(v).map_err(MltError::from)?;
    }
    Ok(tmp)
}

/// Build the RLE body for a `u32` slice: pairs of `(run_length, value)` as VarInts.
fn build_rle_body_u32(values: &[u32]) -> MltResult<Vec<u8>> {
    let mut tmp = Vec::new();
    let mut i = 0;
    while i < values.len() {
        let val = values[i];
        let mut run: u32 = 1;
        while i + (run as usize) < values.len() && values[i + (run as usize)] == val {
            run += 1;
        }
        tmp.write_varint(run).map_err(MltError::from)?;
        tmp.write_varint(val).map_err(MltError::from)?;
        i += run as usize;
    }
    Ok(tmp)
}

/// Build the Delta+VarInt body for a `u64` slice (deltas from the previous value).
/// The first element is the original value (delta from 0).
fn build_delta_body_u64(values: &[u64]) -> MltResult<Vec<u8>> {
    let mut tmp = Vec::new();
    let mut prev = 0u64;
    for &v in values {
        let delta = v.wrapping_sub(prev);
        tmp.write_varint(delta).map_err(MltError::from)?;
        prev = v;
    }
    Ok(tmp)
}

// ── Stream write helpers ──────────────────────────────────────────────────────

/// Write a VarInt-encoded `u32` stream with implicit count (= `feature_count`).
fn write_u32_stream_implicit(buf: &mut Vec<u8>, values: &[u32]) -> MltResult<()> {
    buf.push(ENC_VARINT);
    let body = build_varint_body_u32(values)?;
    buf.write_varint(body.len() as u32)
        .map_err(MltError::from)?;
    buf.extend_from_slice(&body);
    Ok(())
}

/// Write a `u32` types stream choosing the smaller of VarInt vs RLE encoding.
///
/// For pure single-type layers `[0, 0, 0, …]` this reduces from N bytes to ~3.
fn write_types_stream(buf: &mut Vec<u8>, values: &[u32]) -> MltResult<()> {
    let varint_body = build_varint_body_u32(values)?;
    let rle_body = build_rle_body_u32(values)?;

    // VarInt overhead: 1 (enc_byte) + varint(body_len) + body
    // RLE overhead:    1 (enc_byte) + varint(body_len) + body  (same shape)
    if rle_body.len() < varint_body.len() {
        buf.push(ENC_RLE);
        buf.write_varint(rle_body.len() as u32)
            .map_err(MltError::from)?;
        buf.extend_from_slice(&rle_body);
    } else {
        buf.push(ENC_VARINT);
        buf.write_varint(varint_body.len() as u32)
            .map_err(MltError::from)?;
        buf.extend_from_slice(&varint_body);
    }
    Ok(())
}

/// Write a VarInt-encoded `u32` stream with explicit count.
fn write_u32_stream_explicit(buf: &mut Vec<u8>, values: &[u32]) -> MltResult<()> {
    buf.push(ENC_VARINT_EXPL);
    buf.write_varint(values.len() as u32)
        .map_err(MltError::from)?;
    let body = build_varint_body_u32(values)?;
    buf.write_varint(body.len() as u32)
        .map_err(MltError::from)?;
    buf.extend_from_slice(&body);
    Ok(())
}

/// Write a `u64` stream choosing the smaller of plain VarInt vs Delta+VarInt.
///
/// For sequential IDs (common in OSM) delta encoding reduces each ID to 1 byte.
fn write_u64_stream_best(buf: &mut Vec<u8>, values: &[u64]) -> MltResult<()> {
    let plain_body = {
        let mut tmp = Vec::new();
        for &v in values {
            tmp.write_varint(v).map_err(MltError::from)?;
        }
        tmp
    };
    let delta_body = build_delta_body_u64(values)?;

    if delta_body.len() < plain_body.len() {
        buf.push(ENC_DELTA_VARINT);
        buf.write_varint(delta_body.len() as u32)
            .map_err(MltError::from)?;
        buf.extend_from_slice(&delta_body);
    } else {
        buf.push(ENC_VARINT);
        buf.write_varint(plain_body.len() as u32)
            .map_err(MltError::from)?;
        buf.extend_from_slice(&plain_body);
    }
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
fn write_cwdelta_vertices_varint(pair_count: u32, encoded: &[u32]) -> MltResult<Vec<u8>> {
    let mut out = Vec::new();
    out.push(ENC_CWDELTA_VARINT_EXPL);
    out.write_varint(pair_count).map_err(MltError::from)?;
    let mut tmp = Vec::new();
    for &v in encoded {
        tmp.write_varint(v).map_err(MltError::from)?;
    }
    out.write_varint(tmp.len() as u32).map_err(MltError::from)?;
    out.extend_from_slice(&tmp);
    Ok(out)
}

/// Write vertex data using ComponentwiseDelta + FastPFor128 with explicit count (vertex pairs).
///
/// FastPFor128 requires the input length to be a multiple of 128. The tail is handled
/// by the VarByte portion of the `Composition(FastPFor128, VariableByte)` codec.
/// Bytes are stored in little-endian u32 order (no byte-swap needed on x86).
fn write_cwdelta_vertices_fp128(pair_count: u32, encoded: &[u32]) -> MltResult<Vec<u8>> {
    if encoded.is_empty() {
        // FP128 can't encode an empty slice; fall back handled by caller.
        return Ok(Vec::new());
    }
    let mut scratch: Vec<u32> = Vec::with_capacity(encoded.len() + 1024);
    FastPFor128::default()
        .encode(encoded, &mut scratch)
        .map_err(|_| MltError::NotImplemented("v2: FastPFor128 encode error"))?;

    // v2 wire format: little-endian u32 words (no byte swap on LE hosts).
    let byte_data: Vec<u8> = scratch.iter().flat_map(|&w| w.to_le_bytes()).collect();

    let mut out = Vec::new();
    out.push(ENC_CWDELTA_FP128_EXPL);
    out.write_varint(pair_count).map_err(MltError::from)?;
    out.write_varint(byte_data.len() as u32)
        .map_err(MltError::from)?;
    out.extend_from_slice(&byte_data);
    Ok(out)
}

/// Write vertex data: tries CwDelta+FastPFor128 and CwDelta+VarInt, keeps the smaller.
/// When `allow_fpf` is false only VarInt is tried (mirrors `EncoderConfig::allow_fpf`).
fn write_cwdelta_vertices(buf: &mut Vec<u8>, flat_verts: &[i32], allow_fpf: bool) -> MltResult<()> {
    debug_assert!(
        flat_verts.len() % 2 == 0,
        "vertex buffer must have even length"
    );
    let pair_count = (flat_verts.len() / 2) as u32;

    let mut encoded: Vec<u32> = Vec::new();
    encode_componentwise_delta_vec2s(flat_verts, &mut encoded);

    let varint_bytes = write_cwdelta_vertices_varint(pair_count, &encoded)?;

    if allow_fpf {
        let fp128_bytes = write_cwdelta_vertices_fp128(pair_count, &encoded)?;
        if !fp128_bytes.is_empty() && fp128_bytes.len() < varint_bytes.len() {
            buf.extend_from_slice(&fp128_bytes);
            return Ok(());
        }
    }
    buf.extend_from_slice(&varint_bytes);
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
    allow_fpf: bool,
) -> MltResult<()> {
    debug_assert_eq!(types.len(), feature_count);
    // Types stream: count = feature_count (implicit); prefer RLE when all same type.
    write_types_stream(buf, types)?;
    if let Some(g) = geo_lengths {
        write_u32_stream_explicit(buf, g)?;
    }
    if let Some(p) = part_lengths {
        write_u32_stream_explicit(buf, p)?;
    }
    if let Some(r) = ring_lengths {
        write_u32_stream_explicit(buf, r)?;
    }
    write_cwdelta_vertices(buf, vertices, allow_fpf)
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
        buf.push(ColType::OptId as u8);
        let presence: Vec<bool> = ids.iter().map(|id| id.is_some()).collect();
        write_presence(buf, &presence);
        let values: Vec<u64> = ids.iter().filter_map(|id| *id).collect();
        write_u64_stream_best(buf, &values)?;
    } else {
        // Id: all values present; Delta+VarInt compresses sequential OSM IDs well.
        buf.push(ColType::Id as u8);
        let values: Vec<u64> = ids.iter().map(|id| id.unwrap_or(0)).collect();
        write_u64_stream_best(buf, &values)?;
    }

    Ok(())
}

// ── Property column ───────────────────────────────────────────────────────────

fn write_property_column(
    buf: &mut Vec<u8>,
    name: &str,
    values: &[&PropValue],
    feature_count: usize,
    allow_fsst: bool,
) -> MltResult<()> {
    debug_assert_eq!(values.len(), feature_count);

    let any_null = values.iter().any(|v| is_null(v));

    // String columns: choose the smallest available encoding.
    if matches!(values[0], PropValue::Str(_)) {
        let strings: Vec<Option<&str>> = values
            .iter()
            .map(|v| match v {
                PropValue::Str(Some(s)) => Some(s.as_str()),
                _ => None,
            })
            .collect();

        let plain_bytes = build_string_plain_data(&strings, any_null)?;
        let dict_bytes = build_string_dict_data(&strings, any_null)?;
        let fsst_bytes = if allow_fsst {
            build_string_fsst_data(&strings, any_null)?
        } else {
            Vec::new()
        };

        // Choose whichever encoding yields the fewest bytes, then write once.
        // Dict encoding uses index 0 for null — no separate presence bitfield.
        let best_size = if fsst_bytes.is_empty() {
            plain_bytes.len().min(dict_bytes.len())
        } else {
            plain_bytes
                .len()
                .min(dict_bytes.len())
                .min(fsst_bytes.len())
        };

        let (col_type, data, with_presence) =
            if !fsst_bytes.is_empty() && best_size == fsst_bytes.len() {
                (
                    if any_null {
                        ColType::OptStrFsst
                    } else {
                        ColType::StrFsst
                    },
                    &fsst_bytes,
                    any_null,
                )
            } else if best_size == dict_bytes.len() {
                (
                    if any_null {
                        ColType::OptStrDict
                    } else {
                        ColType::StrDict
                    },
                    &dict_bytes,
                    false,
                )
            } else {
                (
                    if any_null {
                        ColType::OptStr
                    } else {
                        ColType::Str
                    },
                    &plain_bytes,
                    any_null,
                )
            };

        buf.push(col_type as u8);
        buf.write_string(name).map_err(MltError::from)?;
        if with_presence {
            let presence: Vec<bool> = strings.iter().map(|s| s.is_some()).collect();
            write_presence(buf, &presence);
        }
        buf.extend_from_slice(data);
        return Ok(());
    }

    let col_type = prop_column_type(values[0], any_null);
    buf.push(col_type as u8);
    buf.write_string(name).map_err(MltError::from)?;

    if any_null {
        let presence: Vec<bool> = values.iter().map(|v| !is_null(v)).collect();
        write_presence(buf, &presence);
    }

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

fn prop_column_type(sample: &PropValue, any_null: bool) -> ColType {
    let (non_opt, opt) = match sample {
        PropValue::Bool(_) => (ColType::Bool, ColType::OptBool),
        PropValue::I8(_) => (ColType::I8, ColType::OptI8),
        PropValue::U8(_) => (ColType::U8, ColType::OptU8),
        PropValue::I32(_) => (ColType::I32, ColType::OptI32),
        PropValue::U32(_) => (ColType::U32, ColType::OptU32),
        PropValue::I64(_) => (ColType::I64, ColType::OptI64),
        PropValue::U64(_) => (ColType::U64, ColType::OptU64),
        PropValue::F32(_) => (ColType::F32, ColType::OptF32),
        PropValue::F64(_) => (ColType::F64, ColType::OptF64),
        PropValue::Str(_) => (ColType::Str, ColType::OptStr),
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
            write_i32_stream_implicit(buf, &data)?;
        }
        PropValue::U32(_) => {
            let data: Vec<u32> = values
                .iter()
                .filter(|v| !is_null(v))
                .map(|v| prop_u32(v))
                .collect();
            write_u32_stream_implicit(buf, &data)?;
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
            write_u64_stream_implicit(buf, &data)?;
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
            // Handled earlier in write_property_column; should not reach here.
            unreachable!("Str columns are handled before write_prop_data");
        }
    }
    Ok(())
}

/// Build the bytes for a StrPlain / OptStrPlain column data section.
/// Returns: lengths_stream + raw_bytes_stream (no col_type, no name, no presence).
fn build_string_plain_data(strings: &[Option<&str>], _any_null: bool) -> MltResult<Vec<u8>> {
    let present: Vec<&str> = strings.iter().filter_map(|s| *s).collect();
    let lengths: Vec<u32> = present.iter().map(|s| s.len() as u32).collect();
    let all_bytes: Vec<u8> = present.iter().flat_map(|s| s.as_bytes()).copied().collect();

    let mut buf = Vec::new();
    // Lengths stream: count = len(present), encoded with implicit count.
    write_u32_stream_implicit(&mut buf, &lengths)?;
    // String data: raw bytes with explicit byte count.
    write_raw_bytes(&mut buf, &all_bytes)?;
    Ok(buf)
}

/// Build the bytes for a StrDict / OptStrDict column data section.
///
/// Wire layout: `dict_lengths_stream | dict_data_stream | indices_stream`
///
/// - `dict_lengths_stream`: VarInt, explicit count = number of unique values.
/// - `dict_data_stream`:    raw bytes, explicit count = total UTF-8 bytes in dict.
/// - `indices_stream`:      VarInt, implicit count = feature_count.
///   - `OptStrDict`: index 0 = null; indices 1..N map to dict entries 0..N-1.
///   - `StrDict`:    indices 0..N-1 map to dict entries directly.
fn build_string_dict_data(strings: &[Option<&str>], any_null: bool) -> MltResult<Vec<u8>> {
    // Build ordered dictionary (preserves first-occurrence order).
    let mut dict: Vec<&str> = Vec::new();
    let mut dict_index: HashMap<&str, u32> = HashMap::new();

    for s in strings.iter().filter_map(|s| *s) {
        if !dict_index.contains_key(s) {
            let idx = dict.len() as u32;
            dict.push(s);
            dict_index.insert(s, idx);
        }
    }

    // Build per-feature index stream.
    let mut indices: Vec<u32> = Vec::with_capacity(strings.len());
    for s in strings {
        match s {
            None => {
                // OptStrDict: null = index 0; StrDict should not have nulls.
                indices.push(0);
            }
            Some(s) => {
                let dict_pos = dict_index[*s];
                if any_null {
                    // Shift by 1: index 0 reserved for null.
                    indices.push(dict_pos + 1);
                } else {
                    indices.push(dict_pos);
                }
            }
        }
    }

    let dict_lengths: Vec<u32> = dict.iter().map(|s| s.len() as u32).collect();
    let dict_bytes: Vec<u8> = dict.iter().flat_map(|s| s.as_bytes()).copied().collect();

    let mut buf = Vec::new();
    // dict_lengths: explicit count = number of dict entries.
    write_u32_stream_explicit(&mut buf, &dict_lengths)?;
    // dict_data: raw bytes.
    write_raw_bytes(&mut buf, &dict_bytes)?;
    // indices: implicit count = feature_count.
    write_u32_stream_implicit(&mut buf, &indices)?;
    Ok(buf)
}

/// Build the bytes for a StrFsst / OptStrFsst column data section.
///
/// Wire layout: `sym_lengths_stream | sym_bytes_stream | val_lengths_stream | corpus_stream`
///
/// - `sym_lengths_stream`: VarInt, explicit count = number of symbols.
/// - `sym_bytes_stream`:   raw bytes, explicit count = total symbol bytes.
/// - `val_lengths_stream`: VarInt, implicit count = popcount (or feature_count if non-optional).
/// - `corpus_stream`:      raw bytes, explicit count = compressed corpus size.
fn build_string_fsst_data(strings: &[Option<&str>], _any_null: bool) -> MltResult<Vec<u8>> {
    let present: Vec<&str> = strings.iter().filter_map(|s| *s).collect();
    // FSST requires at least one string to train on.
    if present.is_empty() {
        // Fall back to an empty structure that the decoder can handle.
        let mut buf = Vec::new();
        write_u32_stream_explicit(&mut buf, &[])?; // 0 symbols
        write_raw_bytes(&mut buf, &[])?; // 0 symbol bytes
        write_u32_stream_implicit(&mut buf, &[])?; // 0 lengths
        write_raw_bytes(&mut buf, &[])?; // 0 corpus bytes
        return Ok(buf);
    }

    let raw = compress_fsst(&present);

    let mut buf = Vec::new();
    write_u32_stream_explicit(&mut buf, &raw.symbol_lengths)?;
    write_raw_bytes(&mut buf, &raw.symbol_bytes)?;
    write_u32_stream_implicit(&mut buf, &raw.value_lengths)?;
    write_raw_bytes(&mut buf, &raw.corpus)?;
    Ok(buf)
}

// ── Shared-dictionary encoding ────────────────────────────────────────────────

/// Build the per-feature index stream for one child of a shared-dict group.
///
/// - Non-optional child: index `k` → dict entry `k` (0-based).
/// - Optional child:     index `0` = null; index `k` (k ≥ 1) → dict entry `k-1`.
fn build_item_indices(
    item: &StagedSharedDictItem,
    span_to_idx: &HashMap<(u32, u32), u32>,
    optional: bool,
) -> Vec<u32> {
    let mut span_iter = item.dense_spans();
    item.presence_bools()
        .map(|present| {
            if present {
                let span = span_iter
                    .next()
                    .expect("v2 SharedDict: presence/dense mismatch");
                let dict_idx = *span_to_idx
                    .get(&span)
                    .expect("v2 SharedDict: span not in dict");
                if optional { dict_idx + 1 } else { dict_idx }
            } else {
                0 // null slot (only reachable when `optional`)
            }
        })
        .collect()
}

/// Build the encoded bytes for a plain-corpus shared-dict column (`COL_STR_SHARED_DICT`).
///
/// Wire layout:
/// ```text
/// [u8: COL_STR_SHARED_DICT]
/// [varint: prefix_len] [prefix bytes]
/// [varint: child_count]
/// [dict_lengths: ENC_VARINT_EXPL, count = dict_entry_count, body]
/// [dict_data:    ENC_RAW_EXPL, count = total UTF-8 bytes]
/// for each child:
///   [u8: child_flags]   bit 0 = optional
///   [varint: suffix_len] [suffix bytes]
///   [indices: ENC_VARINT, implicit count = feature_count, body]
///       non-optional: 0..N-1 → dict index
///       optional:     0 = null, 1..N → dict index + 1
/// ```
fn build_shared_dict_plain(
    prefix: &str,
    items: &[StagedSharedDictItem],
    dict_strings: &[&str],
    span_to_idx: &HashMap<(u32, u32), u32>,
) -> MltResult<Vec<u8>> {
    let mut buf = Vec::new();
    buf.push(ColType::StrSharedDict as u8);
    buf.write_string(prefix).map_err(MltError::from)?;
    buf.write_varint(items.len() as u32)
        .map_err(MltError::from)?;

    let dict_lengths: Vec<u32> = dict_strings.iter().map(|s| s.len() as u32).collect();
    let dict_bytes: Vec<u8> = dict_strings
        .iter()
        .flat_map(|s| s.as_bytes())
        .copied()
        .collect();
    write_u32_stream_explicit(&mut buf, &dict_lengths)?;
    write_raw_bytes(&mut buf, &dict_bytes)?;

    for item in items {
        let optional = item.has_presence();
        buf.push(if optional { CHILD_OPTIONAL } else { 0 });
        buf.write_string(&item.suffix).map_err(MltError::from)?;
        let indices = build_item_indices(item, span_to_idx, optional);
        write_u32_stream_implicit(&mut buf, &indices)?;
    }

    Ok(buf)
}

/// Build the encoded bytes for a FSST-corpus shared-dict column (`COL_STR_SHARED_DICT_FSST`).
///
/// Same as [`build_shared_dict_plain`] but the corpus section becomes:
/// ```text
/// [sym_lengths: ENC_VARINT_EXPL] [sym_bytes: ENC_RAW_EXPL]
/// [val_lengths: ENC_VARINT_EXPL, count = dict_entry_count] [corpus: ENC_RAW_EXPL]
/// ```
/// Per-child index semantics are identical to the plain variant.
fn build_shared_dict_fsst(
    prefix: &str,
    items: &[StagedSharedDictItem],
    dict_strings: &[&str],
    span_to_idx: &HashMap<(u32, u32), u32>,
) -> MltResult<Vec<u8>> {
    let raw = compress_fsst(dict_strings);

    let mut buf = Vec::new();
    buf.push(ColType::StrSharedDictFsst as u8);
    buf.write_string(prefix).map_err(MltError::from)?;
    buf.write_varint(items.len() as u32)
        .map_err(MltError::from)?;

    write_u32_stream_explicit(&mut buf, &raw.symbol_lengths)?;
    write_raw_bytes(&mut buf, &raw.symbol_bytes)?;
    write_u32_stream_explicit(&mut buf, &raw.value_lengths)?;
    write_raw_bytes(&mut buf, &raw.corpus)?;

    for item in items {
        let optional = item.has_presence();
        buf.push(if optional { CHILD_OPTIONAL } else { 0 });
        buf.write_string(&item.suffix).map_err(MltError::from)?;
        let indices = build_item_indices(item, span_to_idx, optional);
        write_u32_stream_implicit(&mut buf, &indices)?;
    }

    Ok(buf)
}

/// Encode a shared-dict group, automatically choosing plain vs FSST corpus.
///
/// Returns the encoded bytes for a single shared-dict column entry.  The caller
/// should compare this against individually-encoded columns and use whichever is
/// smaller (see [`encode_v2_with_sort`]).
///
/// When `allow_fsst` is false only the plain-corpus variant is tried.
fn encode_shared_dict_chunk(
    group: &StringGroup,
    features: &[TileFeature],
    allow_fsst: bool,
) -> MltResult<Vec<u8>> {
    // Extract per-column string values in stable column-index order.
    let mut order: Vec<usize> = (0..group.columns.len()).collect();
    order.sort_by_key(|&i| group.columns[i].1);

    let columns: Vec<(String, Vec<Option<String>>)> = order
        .iter()
        .map(|&i| {
            let (suffix, col_idx) = &group.columns[i];
            let values: Vec<Option<String>> = features
                .iter()
                .map(|f| match f.properties.get(*col_idx) {
                    Some(PropValue::Str(Some(s))) => Some(s.clone()),
                    _ => None,
                })
                .collect();
            (suffix.clone(), values)
        })
        .collect();

    // Build the shared corpus (dedup strings across all children into one buffer).
    let shared_dict = StagedSharedDict::new(
        group.prefix.clone(),
        columns
            .iter()
            .map(|(s, v)| (s.as_str(), v.iter().map(|o| o.as_deref()))),
    )?;

    // Unique dictionary entries (sorted, deduped spans → stable dict order).
    let dict_spans = {
        let mut s: Vec<(u32, u32)> = shared_dict
            .items
            .iter()
            .flat_map(|item| item.dense_spans())
            .collect();
        s.sort_unstable();
        s.dedup();
        s
    };
    let dict_strings: Vec<&str> = dict_spans
        .iter()
        .map(|&(s, e)| {
            shared_dict
                .get((s, e))
                .ok_or(MltError::NotImplemented("v2: shared dict span OOB"))
        })
        .collect::<MltResult<Vec<_>>>()?;
    let span_to_idx: HashMap<(u32, u32), u32> = dict_spans
        .iter()
        .copied()
        .enumerate()
        .map(|(i, span)| (span, i as u32))
        .collect();

    let plain_buf = build_shared_dict_plain(
        &group.prefix,
        &shared_dict.items,
        &dict_strings,
        &span_to_idx,
    )?;

    if allow_fsst && !dict_strings.is_empty() {
        let fsst_buf = build_shared_dict_fsst(
            &group.prefix,
            &shared_dict.items,
            &dict_strings,
            &span_to_idx,
        )?;
        if fsst_buf.len() < plain_buf.len() {
            return Ok(fsst_buf);
        }
    }

    Ok(plain_buf)
}

// ── VarInt stream helpers for optional columns (count = popcount) ─────────────
// These reuse the same wire format as their _implicit counterparts; the count is
// determined by the decoder from context (feature_count or popcount).

fn write_i64_stream_as_explicit(buf: &mut Vec<u8>, values: &[i64]) -> MltResult<()> {
    write_i64_stream_implicit(buf, values)
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
    let (data, name) = parse_string(data)?;
    let name = name.to_string();

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
        let col_type = ColType::try_from(data[0])?;
        data = &data[1..];

        match col_type {
            ColType::Id => {
                let (new_data, ids) = decode_id_column(data, feature_count)?;
                data = new_data;
                for (i, id) in ids.into_iter().enumerate() {
                    feature_ids[i] = Some(id);
                }
            }
            ColType::OptId => {
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
            ColType::Bool
            | ColType::OptBool
            | ColType::I8
            | ColType::OptI8
            | ColType::U8
            | ColType::OptU8
            | ColType::I32
            | ColType::OptI32
            | ColType::U32
            | ColType::OptU32
            | ColType::I64
            | ColType::OptI64
            | ColType::U64
            | ColType::OptU64
            | ColType::F32
            | ColType::OptF32
            | ColType::F64
            | ColType::OptF64
            | ColType::Str
            | ColType::OptStr => {
                let (new_data, col_name, col_values) =
                    decode_property_column(data, col_type, feature_count, &mut presence_groups)?;
                data = new_data;
                property_names.push(col_name);
                property_columns.push(col_values);
            }
            ColType::StrDict | ColType::OptStrDict => {
                let (new_data, col_name, col_values) =
                    decode_string_dict_column(data, col_type, feature_count)?;
                data = new_data;
                property_names.push(col_name);
                property_columns.push(col_values);
            }
            ColType::StrFsst | ColType::OptStrFsst => {
                let (new_data, col_name, col_values) =
                    decode_string_fsst_column(data, col_type, feature_count)?;
                data = new_data;
                property_names.push(col_name);
                property_columns.push(col_values);
            }
            ColType::StrSharedDict | ColType::StrSharedDictFsst => {
                let (new_data, columns) = decode_shared_dict_v2(data, col_type, feature_count)?;
                data = new_data;
                for (col_name, col_values) in columns {
                    property_names.push(col_name);
                    property_columns.push(col_values);
                }
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

/// Read a VarInt (or RLE) u32 stream.
///
/// If `explicit` is true, read the count from the stream; otherwise use `implicit_count`.
/// Supports:
/// - `logical=0` (None) + `physical=2` (VarInt): plain VarInt stream.
/// - `logical=3` (Rle):  byte_length always present; data is `(run_len, value)` VarInt pairs.
fn read_u32_stream(
    data: &[u8],
    implicit_count: usize,
    explicit: bool,
) -> MltResult<(&[u8], Vec<u32>)> {
    let (data, has_expl_count, logical, physical) = read_enc_byte(data)?;

    // ── RLE: logical=3, physical bits reserved (00) ──────────────────────────
    if logical == 3 {
        // byte_length always follows for RLE; no separate count field.
        let (data, byte_len) = parse_varint::<u32>(data)?;
        let byte_len = byte_len as usize;
        if data.len() < byte_len {
            return Err(MltError::BufferUnderflow(byte_len as u32, data.len()));
        }
        let encoded = &data[..byte_len];
        let remaining = &data[byte_len..];

        let capacity = if has_expl_count || explicit {
            implicit_count
        } else {
            16
        };
        let mut values = Vec::with_capacity(capacity);
        let mut pos = 0;
        while pos < byte_len {
            let (run_len, c1) = u32::decode_var(&encoded[pos..])
                .ok_or(MltError::BufferUnderflow(4, encoded.len() - pos))?;
            pos += c1;
            let (val, c2) = u32::decode_var(&encoded[pos..])
                .ok_or(MltError::BufferUnderflow(4, encoded.len() - pos))?;
            pos += c2;
            for _ in 0..run_len {
                values.push(val);
            }
        }
        return Ok((remaining, values));
    }

    // ── VarInt: logical=0, physical=2 ────────────────────────────────────────
    if logical != 0 || physical != 2 {
        return Err(MltError::NotImplemented(
            "v2 decoder: unsupported encoding for u32 stream",
        ));
    }

    let (data, count) = if has_expl_count || explicit {
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

/// Read a VarInt (or Delta+VarInt) u64 stream.
///
/// Supports:
/// - `logical=0` (None) + `physical=2` (VarInt): plain VarInt.
/// - `logical=1` (Delta) + `physical=2` (VarInt): VarInt-encoded deltas, prefix-sum to recover.
fn read_u64_stream(data: &[u8], implicit_count: usize) -> MltResult<(&[u8], Vec<u64>)> {
    let (data, _has_expl_count, logical, physical) = read_enc_byte(data)?;

    if physical != 2 || (logical != 0 && logical != 1) {
        return Err(MltError::NotImplemented(
            "v2 decoder: only VarInt / Delta+VarInt supported for u64 stream",
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

    // Apply prefix-sum to undo delta encoding.
    if logical == 1 {
        let mut acc = 0u64;
        for v in &mut values {
            acc = acc.wrapping_add(*v);
            *v = acc;
        }
    }

    Ok((remaining, values))
}

/// Read a ComponentwiseDelta + (VarInt or FastPFor128) vertex stream (explicit count = vertex pairs).
fn read_cwdelta_stream(data: &[u8]) -> MltResult<(&[u8], Vec<i32>)> {
    let (data, has_expl_count, logical, physical) = read_enc_byte(data)?;

    if logical != 2 {
        return Err(MltError::NotImplemented(
            "v2 decoder: only CwDelta logical encoding supported for vertex stream",
        ));
    }
    if physical != 2 && physical != 3 {
        return Err(MltError::NotImplemented(
            "v2 decoder: only VarInt(2) or FastPFor128(3) physical encoding for vertex stream",
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

    let coord_count = pair_count * 2;

    let u32_vals: Vec<u32> = if physical == 3 {
        // FastPFor128: bytes are LE u32 words
        if !byte_len.is_multiple_of(4) {
            return Err(MltError::NotImplemented(
                "v2 FP128: byte length not multiple of 4",
            ));
        }
        let words: Vec<u32> = encoded
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        let mut result = Vec::with_capacity(coord_count + 128);
        FastPFor128::default()
            .decode(&words, &mut result, Some(coord_count as u32))
            .map_err(|_| MltError::NotImplemented("v2: FastPFor128 decode error"))?;
        result.truncate(coord_count);
        result
    } else {
        // VarInt
        let mut vals = Vec::with_capacity(coord_count);
        let mut pos = 0;
        while pos < byte_len {
            let (v, consumed) = u32::decode_var(&encoded[pos..])
                .ok_or(MltError::BufferUnderflow(4, encoded.len() - pos))?;
            vals.push(v);
            pos += consumed;
        }
        vals
    };

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

fn decode_id_column<'a>(data: &'a [u8], feature_count: usize) -> MltResult<(&'a [u8], Vec<u64>)> {
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
    col_type: ColType,
    feature_count: usize,
    presence_groups: &mut Vec<Vec<bool>>,
) -> MltResult<(&'a [u8], String, Vec<PropValue>)> {
    let (data, name) = parse_string(data)?;
    let name = name.to_string();

    let is_optional = matches!(
        col_type,
        ColType::OptBool
            | ColType::OptI8
            | ColType::OptU8
            | ColType::OptI32
            | ColType::OptU32
            | ColType::OptI64
            | ColType::OptU64
            | ColType::OptF32
            | ColType::OptF64
            | ColType::OptStr
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
    col_type: ColType,
    feature_count: usize,
    popcount: usize,
    presence: &Option<Vec<bool>>,
) -> MltResult<(&'a [u8], Vec<PropValue>)> {
    match col_type {
        ColType::Bool | ColType::OptBool => {
            let (data, bytes) = read_raw_bytes_stream(data)?;
            let values = expand_with_presence(
                bytes.iter().map(|&b| PropValue::Bool(Some(b != 0))),
                presence,
                feature_count,
                PropValue::Bool(None),
            );
            Ok((data, values))
        }
        ColType::I8 | ColType::OptI8 => {
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
        ColType::U8 | ColType::OptU8 => {
            let (data, bytes) = read_raw_bytes_stream(data)?;
            let values = expand_with_presence(
                bytes.into_iter().map(|b| PropValue::U8(Some(b))),
                presence,
                feature_count,
                PropValue::U8(None),
            );
            Ok((data, values))
        }
        ColType::I32 | ColType::OptI32 => {
            let any_null = col_type == ColType::OptI32;
            let (data, ints) = read_i32_stream(data, popcount, any_null)?;
            let values = expand_with_presence(
                ints.into_iter().map(|v| PropValue::I32(Some(v))),
                presence,
                feature_count,
                PropValue::I32(None),
            );
            Ok((data, values))
        }
        ColType::U32 | ColType::OptU32 => {
            let (data, ints) = read_u32_stream(data, popcount, false)?;
            let values = expand_with_presence(
                ints.into_iter().map(|v| PropValue::U32(Some(v))),
                presence,
                feature_count,
                PropValue::U32(None),
            );
            Ok((data, values))
        }
        ColType::I64 | ColType::OptI64 => {
            let (data, ints) = read_i64_stream(data, popcount)?;
            let values = expand_with_presence(
                ints.into_iter().map(|v| PropValue::I64(Some(v))),
                presence,
                feature_count,
                PropValue::I64(None),
            );
            Ok((data, values))
        }
        ColType::U64 | ColType::OptU64 => {
            let (data, ints) = read_u64_stream(data, popcount)?;
            let values = expand_with_presence(
                ints.into_iter().map(|v| PropValue::U64(Some(v))),
                presence,
                feature_count,
                PropValue::U64(None),
            );
            Ok((data, values))
        }
        ColType::F32 | ColType::OptF32 => {
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
        ColType::F64 | ColType::OptF64 => {
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
        ColType::Str | ColType::OptStr => {
            decode_string_column(data, popcount, presence, feature_count)
        }
        // These variants are dispatched before reaching this function.
        ColType::Id
        | ColType::OptId
        | ColType::StrDict
        | ColType::OptStrDict
        | ColType::StrFsst
        | ColType::OptStrFsst
        | ColType::StrSharedDict
        | ColType::StrSharedDictFsst => {
            unreachable!("ColType::{col_type:?} is handled before decode_column_data")
        }
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

/// Decode a `StrFsst` or `OptStrFsst` column.
///
/// Wire layout: `[presence?] | sym_lengths | sym_bytes | val_lengths | corpus`
fn decode_string_fsst_column<'a>(
    data: &'a [u8],
    col_type: ColType,
    feature_count: usize,
) -> MltResult<(&'a [u8], String, Vec<PropValue>)> {
    let optional = col_type == ColType::OptStrFsst;

    let (data, name) = parse_string(data)?;
    let name = name.to_string();

    // presence (only for optional)
    let (data, presence) = if optional {
        let (d, p) = read_presence(data, feature_count)?;
        (d, Some(p))
    } else {
        (data, None)
    };
    let popcount = presence
        .as_ref()
        .map(|p| p.iter().filter(|&&x| x).count())
        .unwrap_or(feature_count);

    // val_lengths uses an implicit count (= popcount) in the per-column FSST variant.
    let (data, present_strings) = read_fsst_strings(data, popcount, false)?;

    let values = if let Some(pres) = &presence {
        let mut result = Vec::with_capacity(feature_count);
        let mut iter = present_strings.into_iter();
        for &present in pres {
            if present {
                result.push(PropValue::Str(iter.next().map(Some).unwrap_or(None)));
            } else {
                result.push(PropValue::Str(None));
            }
        }
        result
    } else {
        present_strings
            .into_iter()
            .map(|s| PropValue::Str(Some(s)))
            .collect()
    };

    Ok((data, name, values))
}

/// Decompress an FSST-encoded corpus into individual strings.
fn fsst_decompress(
    sym_lengths: &[u32],
    sym_bytes: &[u8],
    val_lengths: &[u32],
    corpus: &[u8],
) -> MltResult<Vec<String>> {
    // Build per-symbol offset table.
    let mut sym_offsets = vec![0usize; sym_lengths.len() + 1];
    for (i, &len) in sym_lengths.iter().enumerate() {
        sym_offsets[i + 1] = sym_offsets[i] + len as usize;
    }

    // Decompress corpus: 0xFF → literal next byte; else → expand symbol.
    let mut output: Vec<u8> = Vec::new();
    let mut i = 0;
    while i < corpus.len() {
        let sym_idx = corpus[i] as usize;
        if sym_idx == 255 {
            i += 1;
            if i < corpus.len() {
                output.push(corpus[i]);
            }
        } else if sym_idx < sym_lengths.len() {
            let start = sym_offsets[sym_idx];
            let end = sym_offsets[sym_idx + 1];
            output.extend_from_slice(&sym_bytes[start..end]);
        }
        i += 1;
    }

    // Split by value lengths.
    let mut strings = Vec::with_capacity(val_lengths.len());
    let mut offset = 0usize;
    for &len in val_lengths {
        let end = offset + len as usize;
        let s = std::str::from_utf8(
            output
                .get(offset..end)
                .ok_or(MltError::BufferUnderflow(len, output.len() - offset))?,
        )
        .map_err(MltError::from)?
        .to_string();
        strings.push(s);
        offset = end;
    }
    Ok(strings)
}

/// Decode a `StrDict` or `OptStrDict` column.
///
/// Wire layout: `dict_lengths_stream | dict_data_stream | indices_stream`
///
/// - `StrDict`:    all features have a value; index `k` → dict entry `k`.
/// - `OptStrDict`: index `0` = null; index `k` (k≥1) → dict entry `k-1`.
fn decode_string_dict_column<'a>(
    data: &'a [u8],
    col_type: ColType,
    feature_count: usize,
) -> MltResult<(&'a [u8], String, Vec<PropValue>)> {
    let optional = col_type == ColType::OptStrDict;

    let (data, name) = parse_string(data)?;
    let name = name.to_string();

    // dict_lengths: explicit count = number of dict entries
    let (data, dict_lengths) = read_u32_stream(data, 0, true)?;
    // dict_data: raw bytes
    let (data, dict_bytes) = read_raw_bytes_stream(data)?;
    // indices: implicit count = feature_count
    let (data, indices) = read_u32_stream(data, feature_count, false)?;

    // Reconstruct dictionary entries from lengths + raw bytes.
    let dict = build_strings_from_lengths(&dict_lengths, &dict_bytes)?;

    // Map per-feature indices back to PropValue.
    let mut values = Vec::with_capacity(feature_count);
    for &idx in &indices {
        let pv = if optional {
            if idx == 0 {
                PropValue::Str(None)
            } else {
                let entry = dict
                    .get(idx as usize - 1)
                    .ok_or(MltError::NotImplemented("v2 StrDict: index out of range"))?;
                PropValue::Str(Some(entry.clone()))
            }
        } else {
            let entry = dict
                .get(idx as usize)
                .ok_or(MltError::NotImplemented("v2 StrDict: index out of range"))?;
            PropValue::Str(Some(entry.clone()))
        };
        values.push(pv);
    }

    Ok((data, name, values))
}

/// Decode a `COL_STR_SHARED_DICT` or `COL_STR_SHARED_DICT_FSST` column.
///
/// Returns a list of `(property_name, per_feature_values)` — one entry per child column in
/// the group — so the caller can append them individually to `property_names` /
/// `property_columns`.
///
/// Wire layout (plain corpus, `COL_STR_SHARED_DICT`):
/// ```text
/// [varint: prefix_len] [prefix bytes]
/// [varint: child_count]
/// [dict_lengths: ENC_VARINT_EXPL] [dict_data: ENC_RAW_EXPL]
/// for each child:
///   [u8: flags]  bit 0 = optional
///   [varint: suffix_len] [suffix bytes]
///   [indices: ENC_VARINT, implicit count = feature_count]
/// ```
/// FSST variant (`COL_STR_SHARED_DICT_FSST`) replaces the corpus section with:
/// ```text
/// [sym_lengths: ENC_VARINT_EXPL] [sym_bytes: ENC_RAW_EXPL]
/// [val_lengths: ENC_VARINT_EXPL] [corpus: ENC_RAW_EXPL]
/// ```
fn decode_shared_dict_v2<'a>(
    data: &'a [u8],
    col_type: ColType,
    feature_count: usize,
) -> MltResult<(&'a [u8], Vec<(String, Vec<PropValue>)>)> {
    let use_fsst = col_type == ColType::StrSharedDictFsst;

    let (data, prefix) = parse_string(data)?;
    let prefix = prefix.to_string();

    let (data, child_count) = parse_varint::<u32>(data)?;
    let child_count = child_count as usize;

    // Decode shared dictionary corpus into a Vec<String> (one entry per dict index).
    let (data, dict_strings): (&[u8], Vec<String>) = if use_fsst {
        // val_lengths has an explicit count in the FSST shared-dict variant.
        read_fsst_strings(data, 0, true)?
    } else {
        let (data, dict_lengths) = read_u32_stream(data, 0, true)?;
        let (data, dict_bytes) = read_raw_bytes_stream(data)?;
        (
            data,
            build_strings_from_lengths(&dict_lengths, &dict_bytes)?,
        )
    };

    let mut result = Vec::with_capacity(child_count);
    let mut data = data;

    for _ in 0..child_count {
        if data.is_empty() {
            return Err(MltError::BufferUnderflow(1, 0));
        }
        let flags = data[0];
        data = &data[1..];
        let optional = (flags & CHILD_OPTIONAL) != 0;

        let (d, suffix) = parse_string(data)?;
        let suffix = suffix.to_string();
        data = d;

        let col_name = format!("{prefix}{suffix}");

        // indices: implicit count = feature_count
        let (d, indices) = read_u32_stream(data, feature_count, false)?;
        data = d;

        let values: MltResult<Vec<PropValue>> = indices
            .iter()
            .map(|&idx| {
                if optional && idx == 0 {
                    Ok(PropValue::Str(None))
                } else {
                    let dict_idx = if optional {
                        idx as usize - 1
                    } else {
                        idx as usize
                    };
                    let s = dict_strings
                        .get(dict_idx)
                        .ok_or(MltError::NotImplemented(
                            "v2 SharedDict: index out of range",
                        ))?
                        .clone();
                    Ok(PropValue::Str(Some(s)))
                }
            })
            .collect();

        result.push((col_name, values?));
    }

    Ok((data, result))
}

/// Reconstruct strings from a parallel `(lengths, flat_bytes)` pair.
fn build_strings_from_lengths(lengths: &[u32], bytes: &[u8]) -> MltResult<Vec<String>> {
    let mut strings = Vec::with_capacity(lengths.len());
    let mut offset = 0usize;
    for &len in lengths {
        let end = offset + len as usize;
        let s = std::str::from_utf8(bytes.get(offset..end).ok_or(MltError::BufferUnderflow(
            len,
            bytes.len().saturating_sub(offset),
        ))?)
        .map_err(MltError::from)?
        .to_string();
        strings.push(s);
        offset = end;
    }
    Ok(strings)
}

/// Read FSST-encoded streams and decompress them into plain strings.
///
/// `val_explicit` controls whether the value-lengths count is read from the stream
/// (`true`) or inferred from `val_count` (`false`).
fn read_fsst_strings<'a>(
    data: &'a [u8],
    val_count: usize,
    val_explicit: bool,
) -> MltResult<(&'a [u8], Vec<String>)> {
    let (data, sym_lengths) = read_u32_stream(data, 0, true)?;
    let (data, sym_bytes) = read_raw_bytes_stream(data)?;
    let (data, val_lengths) = read_u32_stream(data, val_count, val_explicit)?;
    let (data, corpus) = read_raw_bytes_stream(data)?;
    let strings = fsst_decompress(&sym_lengths, &sym_bytes, &val_lengths, &corpus)?;
    Ok((data, strings))
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
