//! Data model for the annotated binary dump (see [`crate::dump`]).

use crate::wire::StreamMeta;

/// Whether a region is tile metadata or an opaque data payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionKind {
    /// Framing, schema, or stream-header bytes, annotated byte- and bit-for-byte.
    Meta,
    /// A stream payload, rendered as raw hex plus best-effort decoded values.
    DataBlob,
}

/// How a [`RegionKind::DataBlob`] payload is decoded for display.
///
/// Best-effort: on any decode error the renderer falls back to raw hex.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeHint {
    /// Nullability bitmap (byte-RLE → packed bits).
    Presence,
    /// Boolean data stream (byte-RLE → bools).
    Bool,
    /// Signed 32-bit integers (`i8`/`i32` columns).
    I32,
    /// Unsigned 32-bit integers (`u8`/`u32` columns, offsets, lengths).
    U32,
    /// Signed 64-bit integers (`i64` columns).
    I64,
    /// Unsigned 64-bit integers (`u64` columns, 64-bit ids).
    U64,
    /// 32-bit floats.
    F32,
    /// 64-bit floats.
    F64,
    /// Opaque bytes (string / dictionary / FSST payloads); shown as hex + UTF-8 preview.
    Bytes,
}

/// One sub-field of a bit-packed byte, e.g. a nibble of `stream_type`.
#[derive(Debug, Clone)]
pub struct BitField {
    /// Inclusive high bit index (7..=0, MSB first).
    pub hi: u8,
    /// Inclusive low bit index.
    pub lo: u8,
    /// The extracted field value.
    pub raw: u64,
    /// Human-readable meaning, e.g. `"physical = VarInt"`.
    pub meaning: String,
}

/// Stream metadata attached to a [`RegionKind::DataBlob`] so the renderer can decode it.
#[derive(Debug, Clone, Copy)]
pub struct BlobInfo {
    pub meta: StreamMeta,
    pub hint: DecodeHint,
}

/// A single annotated span of the tile buffer.
///
/// Emitted in pre-order. Containers bracket their children and may overlap them.
/// Leaf regions partition the buffer exactly; the coverage test relies on this.
#[derive(Debug, Clone)]
pub struct Region {
    /// Absolute byte offset into the tile buffer.
    pub offset: usize,
    /// Byte length of this region.
    pub len: usize,
    /// Nesting depth, for indentation.
    pub depth: usize,
    /// Short label, e.g. `"column[2].type"` or `"num_values"`.
    pub label: String,
    /// Rendered scalar value (varint value, string, enum name), if any.
    pub value: Option<String>,
    /// Bit-level breakdown; empty unless this is a bit-packed byte.
    pub bits: Vec<BitField>,
    pub kind: RegionKind,
    /// True for structural groups that span their children (excluded from coverage).
    pub container: bool,
    /// Present for `DataBlob` regions that carry decodable stream metadata.
    pub blob: Option<BlobInfo>,
}

/// The full annotation of a tile: a flat, depth-tagged region list.
pub struct DumpTree {
    /// Total length of the annotated buffer.
    pub buf_len: usize,
    /// Regions in pre-order (containers before their children).
    pub regions: Vec<Region>,
}
