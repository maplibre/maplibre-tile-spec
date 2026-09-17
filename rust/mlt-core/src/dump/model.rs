//! Data model for the annotated binary dump (see [`crate::dump`]).

use std::fmt::{Display, Formatter, Result as FmtResult};

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
    /// Nullability bitmap (byte-RLE -> packed bits).
    Presence,
    /// Boolean data stream (byte-RLE -> bools).
    Bool,
    /// Signed 32-bit integers (`i8`/`i32` columns).
    I32,
    /// Unsigned 32-bit integers (`u8`/`u32` columns, offsets, lengths).
    U32,
    /// Signed 64-bit integers (`i64` columns).
    I64,
    /// ALP offsets, put back on their frame of reference.
    #[cfg(feature = "unstable-v2")]
    Alp(crate::decoder::Alp),
    /// Unsigned 64-bit integers (`u64` columns, 64-bit ids).
    U64,
    /// 32-bit floats.
    F32,
    /// 64-bit floats.
    F64,
    /// Opaque bytes (string / dictionary / FSST payloads); shown as hex + UTF-8 preview.
    Bytes,
    /// A raw LSB0 bitfield, not a stream: `ceil(num_values/8)` packed bytes.
    #[cfg(feature = "unstable-v2")]
    PackedBits,
}

/// One sub-field of a bit-packed byte, e.g. a nibble of `stream_type`.
#[derive(Debug, Clone)]
pub struct BitField {
    /// Inclusive high bit index (7..=0, MSB first).
    hi: u8,
    /// Inclusive low bit index.
    lo: u8,
    /// The extracted field value, shifted down to bit 0.
    raw: u64,
    /// Human-readable meaning, e.g. `"physical = VarInt"`.
    meaning: String,
}

impl BitField {
    /// One field of a packed byte, located by the same mask constant the parser reads it
    /// with, so the dump cannot drift from the wire format.
    ///
    /// `raw` is the masked bits shifted down to bit 0, so it renders as the field's own
    /// value rather than its in-byte position. `mask` must be one contiguous run of bits.
    pub fn mask(mask: u8, byte: u8, meaning: impl Into<String>) -> Self {
        let (hi, lo) = mask_bounds(mask);
        Self {
            hi,
            lo,
            raw: u64::from((byte & mask) >> lo),
            meaning: meaning.into(),
        }
    }

    /// A one-bit flag, worded as a phrase instead of a `0`/`1`.
    ///
    /// The renderer already prefixes every bit line with `bit N = V ->`, so `when_set`
    /// and `when_clear` should say what the bit asserts about the tile rather than
    /// repeat its value.
    #[must_use]
    pub fn flag(mask: u8, byte: u8, when_set: &str, when_clear: &str) -> Self {
        debug_assert_eq!(mask.count_ones(), 1, "a flag occupies exactly one bit");
        let meaning = if byte & mask == 0 {
            when_clear
        } else {
            when_set
        };
        Self::mask(mask, byte, meaning)
    }

    /// What this field means in prose, without the `bit N = V ->` prefix
    /// [`Display`] puts in front of it.
    #[must_use]
    pub fn meaning(&self) -> &str {
        &self.meaning
    }
}

impl Display for BitField {
    /// `bit 7 = 1 -> meaning`, or `bits 6-4 = 011 -> meaning` for a multi-bit field.
    ///
    /// The value is printed as wide as the field, so the leading zeros of a nibble are
    /// not mistaken for a narrower field. The caller adds any indent and color.
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if self.hi == self.lo {
            write!(f, "bit {}", self.hi)?;
        } else {
            write!(f, "bits {}-{}", self.hi, self.lo)?;
        }
        let width = usize::from(self.hi - self.lo + 1);
        write!(f, " = {:0width$b} -> {}", self.raw, self.meaning)
    }
}

/// Inclusive `(hi, lo)` bit indices spanned by `mask`.
#[expect(
    clippy::cast_possible_truncation,
    reason = "a u8 mask spans at most 8 bits"
)]
fn mask_bounds(mask: u8) -> (u8, u8) {
    debug_assert!(mask != 0, "a bit field needs at least one bit");
    let lo = mask.trailing_zeros() as u8;
    let hi = (u8::BITS - 1 - mask.leading_zeros()) as u8;
    debug_assert_eq!(
        u32::from(hi - lo) + 1,
        mask.count_ones(),
        "a bit field's mask must be one contiguous run of bits"
    );
    (hi, lo)
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
    pub buf_len: usize,
    /// Regions in pre-order (containers before their children).
    pub regions: Vec<Region>,
}
