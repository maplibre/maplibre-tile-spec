//! Annotating walker for the tag `0x02` (v2) layer body.
//!
//! Unlike v1, a stream's role and value count are not on the wire.
//! role comes from its position, the count from the envelope unless the header carries one.

use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;
use bitvec::view::BitView as _;
use usize_cast::IntoUsize as _;

use super::model::{BitField, BlobInfo, DecodeHint};
use super::walker::Walker;
use crate::codecs::varint::parse_varint;
use crate::decoder::stream::header02;
use crate::decoder::stream::header02::{
    Family, HAS_EXPLICIT_COUNT, StreamCtx02, describe_encoding,
};
use crate::decoder::{
    ColumnType02, DataType02, GeoLayout, LayerLayout, LengthType, Presence02, StreamType,
};
use crate::tile::Extent;
use crate::utils::{parse_string, parse_u8, take};
use crate::wire::{IntEncoding, StreamMeta};
use crate::{MltError, MltResult};

impl<'a> Walker<'a> {
    pub(super) fn walk_layer02(&mut self, input: &'a [u8]) -> MltResult<()> {
        let (input, name) = self.field(input, "name", parse_string, |s| Some(format!("{s:?}")))?;
        if name.is_empty() {
            return Err(MltError::MissingLayerName);
        }
        let (input, extent) = self.field(
            input,
            "extent",
            |i| parse_varint::<u32>(i),
            |v| Some(v.to_string()),
        )?;
        Extent::new(extent)?;
        let (input, feature_count) = self.field(
            input,
            "feature_count",
            |i| parse_varint::<u32>(i),
            |v| Some(v.to_string()),
        )?;

        let (_, layout_byte) = parse_u8(input)?;
        let layout = LayerLayout::parse(layout_byte)?;
        let (mut input, _) = self.byte_field(
            input,
            "layout",
            |b| format!("0x{b:02X} {:?}", layout.geometry),
            layer_layout_bits02,
        )?;

        // The layer's shared presence bitfields, which columns read by index.
        let mut shared = Vec::with_capacity(usize::from(layout.shared_presence));
        if layout.shared_presence > 0 {
            let pi = self.open(input, "shared_presence".to_string());
            for i in 0..layout.shared_presence {
                let bits;
                (input, bits) =
                    self.walk_bitfield02(input, feature_count, &format!("present[{i}]"))?;
                shared.push(bits);
            }
            self.close(pi, input);
        }

        let gi = self.open(input, "geometry".to_string());
        input = self.walk_geometry02(input, layout.geometry, feature_count)?;
        self.close(gi, input);

        let (rest, column_count) = self.field(
            input,
            "column_count",
            |i| parse_varint::<u32>(i),
            |v| Some(v.to_string()),
        )?;
        input = rest;
        // Each column requires at least 1 byte (column type).
        if input.len() < column_count.into_usize() {
            return Err(MltError::BufferUnderflow(column_count, input.len()));
        }

        if column_count > 0 {
            let di = self.open(input, "columns".to_string());
            for i in 0..column_count {
                input = self.walk_column02(input, i, feature_count, &shared)?;
            }
            self.close(di, input);
        }

        // A well-formed layer consumes its whole body; record any trailing bytes.
        if !input.is_empty() {
            self.raw_blob(input, input.len(), "trailing bytes".to_string());
        }
        Ok(())
    }

    /// Mirror `parse_geometry`: the streams the layer layout declares, in order.
    fn walk_geometry02(
        &mut self,
        input: &'a [u8],
        layout: GeoLayout,
        feature_count: u32,
    ) -> MltResult<&'a [u8]> {
        if layout.is_dict() {
            return Err(MltError::NotImplemented("v2 dict geometry layouts"));
        }
        if layout.is_tess() {
            return Err(MltError::NotImplemented("v2 tessellated geometry layouts"));
        }

        let mut input = self.walk_stream02(
            input,
            StreamCtx02::GeomTypes,
            feature_count,
            "types",
            DecodeHint::U32,
        )?;

        let lengths = [
            (
                layout.has_geo_lengths(),
                LengthType::Geometries,
                "geo_lengths",
            ),
            (layout.has_part_lengths(), LengthType::Parts, "part_lengths"),
            (layout.has_ring_lengths(), LengthType::Rings, "ring_lengths"),
        ];
        for (present, length_type, label) in lengths {
            if present {
                let ctx = StreamCtx02::GeomOffsets(length_type);
                input = self.walk_stream02(input, ctx, feature_count, label, DecodeHint::U32)?;
            }
        }

        self.walk_stream02(
            input,
            StreamCtx02::GeomVertices,
            feature_count,
            "vertices",
            DecodeHint::I32,
        )
    }

    /// `[u8 type][name?][presence bitfield?][data stream]`.
    ///
    /// `shared` holds the layer's shared presence bitfields, one of which this
    /// column may read instead of storing its own.
    fn walk_column02(
        &mut self,
        input: &'a [u8],
        i: u32,
        feature_count: u32,
        shared: &[&'a BitSlice<u8, Lsb0>],
    ) -> MltResult<&'a [u8]> {
        let ci = self.open(input, format!("column[{i}]"));

        let (_, typ_byte) = parse_u8(input)?;
        let shared_count = u8::try_from(shared.len())?;
        let typ = ColumnType02::parse(typ_byte, shared_count)?;
        let (mut input, _) = self.byte_field(
            input,
            "type",
            |b| format!("0x{b:02X} {:?} {:?}", typ.presence, typ.data),
            move |b| column_type_bits02(b, shared_count),
        )?;

        let mut name_suffix = String::new();
        if typ.data.has_name() {
            let (rest, name) =
                self.field(input, "name", parse_string, |s| Some(format!("{s:?}")))?;
            input = rest;
            name_suffix = format!(" {name:?}");
        }
        let opt = if typ.presence.is_optional() {
            "Opt"
        } else {
            ""
        };
        self.relabel(ci, format!("column[{i}] {opt}{:?}{name_suffix}", typ.data));

        // Presence is a raw LSB0 bitfield, not a stream, and sets the data count.
        // A shared bitfield was already walked at the layer root, so only its
        // popcount is needed here.
        let data_count = match typ.presence {
            Presence02::AllPresent => feature_count,
            Presence02::Inline => {
                let bits;
                (input, bits) = self.walk_bitfield02(input, feature_count, "present")?;
                u32::try_from(bits.count_ones())?
            }
            Presence02::Shared(index) => {
                let bits = shared
                    .get(usize::from(index))
                    .ok_or(MltError::ParsingColumnType(typ_byte))?;
                u32::try_from(bits.count_ones())?
            }
        };

        let ctx = StreamCtx02::Property(typ.data);
        input = self.walk_stream02(input, ctx, data_count, "data", hint_for(typ.data))?;

        self.close(ci, input);
        Ok(input)
    }

    /// Annotate one raw `ceil(feature_count/8)` byte presence bitfield.
    fn walk_bitfield02(
        &mut self,
        input: &'a [u8],
        feature_count: u32,
        label: &str,
    ) -> MltResult<(&'a [u8], &'a BitSlice<u8, Lsb0>)> {
        let (rest, bytes) = take(input, feature_count.div_ceil(8))?;
        self.stream_blob(
            bytes,
            bytes.len(),
            label.to_string(),
            BlobInfo {
                meta: StreamMeta::new(StreamType::Present, IntEncoding::none(), feature_count),
                hint: DecodeHint::PackedBits,
            },
        );
        Ok((
            rest,
            &bytes.view_bits::<Lsb0>()[..feature_count.into_usize()],
        ))
    }

    /// Walk one v2 stream: the annotated header (via the authoritative
    /// [`header02::parse_stream`]) followed by the payload blob.
    ///
    /// `ctx`, `implicit_count`, and `hint` are all supplied by the caller: none of them are on the wire.
    /// `ctx` also names the family the encoding byte's logical field is read against.
    fn walk_stream02(
        &mut self,
        input: &'a [u8],
        ctx: StreamCtx02,
        implicit_count: u32,
        label: &str,
        hint: DecodeHint,
    ) -> MltResult<&'a [u8]> {
        let si = self.open(input, label.to_string());

        // parse -> synthesized meta.
        let (rest, stream) = header02::parse_stream(input, ctx, implicit_count, &mut self.parser)?;

        // Re-walk the consumed header bytes to annotate each field.
        let hi = self.open(input, "header".to_string());
        let (mut c, enc_byte) = self.byte_field(
            input,
            "encoding",
            |b| {
                format!(
                    "0x{b:02X} logical={:?} physical={:?}",
                    stream.meta.encoding.logical, stream.meta.encoding.physical
                )
            },
            |b| encoding_bits02(b, implicit_count, ctx.family()),
        )?;

        if enc_byte & HAS_EXPLICIT_COUNT != 0 {
            (c, _) = self.field(
                c,
                "num_values",
                |i| parse_varint::<u32>(i),
                |v| Some(v.to_string()),
            )?;
        }
        let byte_length;
        (c, byte_length) = self.field(
            c,
            "byte_length",
            |i| parse_varint::<u32>(i),
            |v| Some(v.to_string()),
        )?;
        self.close(hi, c);

        let (after_payload, payload) = take(c, byte_length)?;
        // Consistency guard: the hand re-walk must land exactly on the authoritative tail.
        if self.off(after_payload) != self.off(rest) {
            return Err(MltError::NotImplemented("v2 stream header re-walk desync"));
        }
        self.stream_blob(
            payload,
            payload.len(),
            "data".to_string(),
            BlobInfo {
                meta: stream.meta,
                hint,
            },
        );

        self.close(si, rest);
        Ok(rest)
    }
}

/// Decode hint for a column's data stream, keyed by the data type nibble.
fn hint_for(typ: DataType02) -> DecodeHint {
    use DataType02 as D;
    match typ {
        D::Bool => DecodeHint::Bool,
        D::I8 | D::I32 => DecodeHint::I32,
        D::Id | D::U8 | D::U32 => DecodeHint::U32,
        D::I64 => DecodeHint::I64,
        D::LongId | D::U64 => DecodeHint::U64,
        D::F32 => DecodeHint::F32,
        D::F64 => DecodeHint::F64,
    }
}

/// Bit breakdown of the v2 layer layout byte:
/// - reserved (7),
/// - shared presence bitfield count (6-4),
/// - geometry layout (3-0).
fn layer_layout_bits02(byte: u8) -> Vec<BitField> {
    let (reserved, shared_presence, geometry) = LayerLayout::fields(byte);
    let name_geo = GeoLayout::try_from(geometry)
        .map_or_else(|_| format!("reserved({geometry})"), |g| format!("{g:?}"));
    let reserved = u64::from(reserved != 0);
    vec![
        BitField {
            hi: 7,
            lo: 7,
            raw: reserved,
            meaning: format!("reserved = {reserved}"),
        },
        BitField {
            hi: 6,
            lo: 4,
            raw: u64::from(shared_presence),
            meaning: format!("shared presence bitfields = {shared_presence}"),
        },
        BitField {
            hi: 3,
            lo: 0,
            raw: u64::from(geometry),
            meaning: format!("geometry layout = {name_geo}"),
        },
    ]
}

/// Bit breakdown of the v2 column type byte: presence (7-4), data type (3-0).
fn column_type_bits02(byte: u8, shared_count: u8) -> Vec<BitField> {
    let (presence, data) = ColumnType02::fields(byte);
    let name_pr = Presence02::parse(presence, shared_count).map_or_else(
        || format!("reserved({})", presence >> 4),
        |p| format!("{p:?}"),
    );
    let name_dt = DataType02::try_from(data)
        .map_or_else(|_| format!("reserved({data})"), |d| format!("{d:?}"));
    vec![
        BitField {
            hi: 7,
            lo: 4,
            raw: u64::from(presence >> 4),
            meaning: format!("presence = {name_pr}"),
        },
        BitField {
            hi: 3,
            lo: 0,
            raw: u64::from(data),
            meaning: format!("data type = {name_dt}"),
        },
    ]
}

/// Bit breakdown of the v2 encoding byte: explicit-count flag (7), logical (6-4),
/// physical (3-2), extension (1-0).
fn encoding_bits02(byte: u8, implicit_count: u32, family: Family) -> Vec<BitField> {
    let explicit = byte & HAS_EXPLICIT_COUNT != 0;
    let logical = (byte >> 4) & 0x7;
    let physical = (byte >> 2) & 0x3;
    let extension = byte & 0x3;
    let (name_lo, name_ph) = describe_encoding(family, byte);
    let count = if explicit {
        "has_explicit_count = true -> a num_values varint follows".to_string()
    } else {
        format!("has_explicit_count = false -> {implicit_count} values from context")
    };
    vec![
        BitField {
            hi: 7,
            lo: 7,
            raw: u64::from(u8::from(explicit)),
            meaning: count,
        },
        BitField {
            hi: 6,
            lo: 4,
            raw: u64::from(logical),
            meaning: format!("logical = {name_lo} ({family:?} family)"),
        },
        BitField {
            hi: 3,
            lo: 2,
            raw: u64::from(physical),
            meaning: format!("physical = {name_ph}"),
        },
        BitField {
            hi: 1,
            lo: 0,
            raw: u64::from(extension),
            meaning: format!("extension = {extension}"),
        },
    ]
}
