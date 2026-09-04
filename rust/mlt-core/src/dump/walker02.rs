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
    Family, HAS_EXPLICIT_COUNT, StrLayout, StreamCtx02, describe_encoding,
};
use crate::decoder::{
    ColumnType02, DataType02, DictionaryType, GeoLayout, LayerLayout, LengthType, Presence02,
    SharedDictKind, StreamType,
};
use crate::tile::Extent;
use crate::utils::{parse_u8, take};
use crate::wire::{
    FloatLogical, IntEncoding, LogicalEncoding, StreamMeta, ValueKind, VertexLogical,
};
use crate::{MltError, MltResult};

impl<'a> Walker<'a> {
    pub(super) fn walk_layer02(&mut self, input: &'a [u8]) -> MltResult<()> {
        let (input, name) = self.name_field(input, "name")?;
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
        let (mut input, _) = self.walk_stream02(
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
                (input, _) =
                    self.walk_stream02(input, ctx, feature_count, label, DecodeHint::U32)?;
            }
        }

        if layout.is_tess() {
            let ctx = StreamCtx02::GeomOffsets(LengthType::Triangles);
            (input, _) =
                self.walk_stream02(input, ctx, feature_count, "tri_lengths", DecodeHint::U32)?;
            (input, _) = self.walk_stream02(
                input,
                StreamCtx02::GeomIndices,
                feature_count,
                "tri_indexes",
                DecodeHint::U32,
            )?;
        }

        let label = if layout.is_dict() {
            "vertex_dict"
        } else {
            "vertices"
        };
        (input, _) = self.walk_stream02(
            input,
            StreamCtx02::GeomVertices,
            feature_count,
            label,
            DecodeHint::I32,
        )?;
        if layout.is_dict() {
            (input, _) = self.walk_stream02(
                input,
                StreamCtx02::GeomVertexOffsets,
                feature_count,
                "vertex_offsets",
                DecodeHint::U32,
            )?;
        }
        Ok(input)
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
        // A shared dictionary spends its presence nibble on the corpus encoding, so it walks
        // its own way rather than through the presence-carrying column shape below.
        if ColumnType02::fields(typ_byte).1 == DataType02::SharedDict as u8 {
            let input = self.walk_shared_dict02(input, ci, i, feature_count, shared)?;
            self.close(ci, input);
            return Ok(input);
        }
        let typ = ColumnType02::parse(typ_byte, shared_count)?;
        let (mut input, _) = self.byte_field(
            input,
            "type",
            |b| format!("0x{b:02X} {:?} {:?}", typ.presence, typ.data),
            move |b| column_type_bits02(b, shared_count),
        )?;

        let mut name_suffix = String::new();
        if typ.data.has_name() {
            let (rest, name) = self.name_field(input, "name")?;
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

        // A string column has a stream set of its own, the rest one data stream.
        if typ.data == DataType02::Str {
            input = self.walk_strings02(input, data_count)?;
            self.close(ci, input);
            return Ok(input);
        }

        let ctx = StreamCtx02::Property(typ.data);
        let meta;
        (input, meta) = self.walk_stream02(input, ctx, data_count, "data", hint_for(typ.data))?;

        // A dictionary column's data stream holds codes, and the values follow.
        if meta.encoding.logical == LogicalEncoding::Float(FloatLogical::Dict) {
            let ctx = StreamCtx02::PropertyDictionary(typ.data);
            (input, _) = self.walk_stream02(
                input,
                ctx,
                meta.num_values,
                "dictionary",
                hint_for(typ.data),
            )?;
        }

        self.close(ci, input);
        Ok(input)
    }

    /// Mirror `parse_shared_dict02`: the corpus, then the children that index into it.
    fn walk_shared_dict02(
        &mut self,
        input: &'a [u8],
        ci: usize,
        i: u32,
        feature_count: u32,
        shared: &[&'a BitSlice<u8, Lsb0>],
    ) -> MltResult<&'a [u8]> {
        let (_, typ_byte) = parse_u8(input)?;
        let kind = SharedDictKind::parse(ColumnType02::fields(typ_byte).0)
            .ok_or(MltError::ParsingColumnType(typ_byte))?;
        let (mut input, _) = self.byte_field(
            input,
            "type",
            |b| format!("0x{b:02X} {kind:?} SharedDict"),
            shared_dict_type_bits02,
        )?;

        let name;
        (input, name) = self.name_field(input, "name")?;
        self.relabel(ci, format!("column[{i}] SharedDict {name:?}"));
        let child_count;
        (input, child_count) = self.field(input, "child_count", parse_varint::<u32>, |c| {
            Some(c.to_string())
        })?;

        // The corpus streams mirror a lone string column's dictionary tail.
        match kind {
            SharedDictKind::Plain => {
                (input, _) = self.walk_stream02(
                    input,
                    StreamCtx02::StrDictLengths,
                    0,
                    "dict_lengths",
                    DecodeHint::U32,
                )?;
                (input, _) = self.walk_stream02(
                    input,
                    StreamCtx02::StrBlob(DictionaryType::Shared),
                    0,
                    "dict_values",
                    DecodeHint::Bytes,
                )?;
            }
            SharedDictKind::Fsst => {
                (input, _) = self.walk_stream02(
                    input,
                    StreamCtx02::StrDictLengths,
                    0,
                    "dict_lengths",
                    DecodeHint::U32,
                )?;
                (input, _) = self.walk_stream02(
                    input,
                    StreamCtx02::StrSymbolLengths,
                    0,
                    "symbol_lengths",
                    DecodeHint::U32,
                )?;
                (input, _) = self.walk_stream02(
                    input,
                    StreamCtx02::StrBlob(DictionaryType::Fsst),
                    0,
                    "symbol_table",
                    DecodeHint::Bytes,
                )?;
                (input, _) = self.walk_stream02(
                    input,
                    StreamCtx02::StrBlob(DictionaryType::Shared),
                    0,
                    "corpus",
                    DecodeHint::Bytes,
                )?;
            }
        }

        let shared_count = u8::try_from(shared.len())?;
        for child in 0..child_count {
            let cc = self.open(input, format!("child[{child}]"));
            let (_, child_byte) = parse_u8(input)?;
            let child_typ = ColumnType02::parse(child_byte, shared_count)?;
            (input, _) = self.byte_field(
                input,
                "type",
                |b| format!("0x{b:02X} {:?} {:?}", child_typ.presence, child_typ.data),
                move |b| column_type_bits02(b, shared_count),
            )?;
            let suffix;
            (input, suffix) = self.name_field(input, "name")?;
            self.relabel(cc, format!("child[{child}] {suffix:?}"));

            let count = match child_typ.presence {
                Presence02::AllPresent => feature_count,
                Presence02::Inline => {
                    let bits;
                    (input, bits) = self.walk_bitfield02(input, feature_count, "present")?;
                    u32::try_from(bits.count_ones())?
                }
                Presence02::Shared(index) => {
                    let bits = shared
                        .get(usize::from(index))
                        .ok_or(MltError::ParsingColumnType(child_byte))?;
                    u32::try_from(bits.count_ones())?
                }
            };
            (input, _) = self.walk_stream02(
                input,
                StreamCtx02::StrData(StrLayout::Dict),
                count,
                "codes",
                DecodeHint::U32,
            )?;
            self.close(cc, input);
        }
        Ok(input)
    }

    /// Mirror `parse_strings`: the leading stream names the layout the rest of the streams follow.
    fn walk_strings02(&mut self, input: &'a [u8], count: u32) -> MltResult<&'a [u8]> {
        /// One string stream: what it holds, what to call it, and how to read its payload.
        type Stream = (StreamCtx02, &'static str, DecodeHint);
        const DICT_LENGTHS: Stream = (StreamCtx02::StrDictLengths, "dict_lengths", DecodeHint::U32);
        const SYMBOL_LENGTHS: Stream = (
            StreamCtx02::StrSymbolLengths,
            "symbol_lengths",
            DecodeHint::U32,
        );
        const SYMBOL_TABLE: Stream = (
            StreamCtx02::StrBlob(DictionaryType::Fsst),
            "symbol_table",
            DecodeHint::Bytes,
        );
        const CORPUS: Stream = (
            StreamCtx02::StrBlob(DictionaryType::Single),
            "corpus",
            DecodeHint::Bytes,
        );
        const VALUES: Stream = (
            StreamCtx02::StrBlob(DictionaryType::None),
            "values",
            DecodeHint::Bytes,
        );
        const DICT_VALUES: Stream = (
            StreamCtx02::StrBlob(DictionaryType::Single),
            "dict_values",
            DecodeHint::Bytes,
        );

        let (_, enc_byte) = parse_u8(input)?;
        let layout = StrLayout::from_bits(enc_byte);
        let leading = match layout {
            StrLayout::Plain | StrLayout::Fsst => "lengths",
            StrLayout::Dict | StrLayout::FsstDict => "codes",
        };
        let rest: &[Stream] = match layout {
            StrLayout::Plain => &[VALUES],
            StrLayout::Dict => &[DICT_LENGTHS, DICT_VALUES],
            StrLayout::Fsst => &[SYMBOL_LENGTHS, SYMBOL_TABLE, CORPUS],
            StrLayout::FsstDict => &[DICT_LENGTHS, SYMBOL_LENGTHS, SYMBOL_TABLE, CORPUS],
        };

        let (mut input, _) = self.walk_stream02(
            input,
            StreamCtx02::StrData(layout),
            count,
            leading,
            DecodeHint::U32,
        )?;
        for &(ctx, label, hint) in rest {
            (input, _) = self.walk_stream02(input, ctx, count, label, hint)?;
        }
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
                meta: StreamMeta::new(
                    StreamType::Present,
                    IntEncoding::none(ValueKind::Bool),
                    feature_count,
                ),
                hint: DecodeHint::PackedBits,
            },
        );
        Ok((
            rest,
            &bytes.view_bits::<Lsb0>()[..feature_count.into_usize()],
        ))
    }

    /// Walk a name field, which the tile's name table may hold instead of the layer.
    fn name_field(&mut self, input: &'a [u8], label: &str) -> MltResult<(&'a [u8], &'a str)> {
        let names = self.names.clone();
        self.field(
            input,
            label,
            |i| crate::decoder::parse_name02(i, names.as_deref()),
            |s| Some(format!("{s:?}")),
        )
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
    ) -> MltResult<(&'a [u8], StreamMeta)> {
        let si = self.open(input, label.to_string());

        // parse -> synthesized meta.
        let (rest, stream) = header02::parse_stream(input, ctx, implicit_count, &mut self.parser)?;

        // Re-walk the consumed header bytes to annotate each field.
        let hi = self.open(input, "header".to_string());
        let family = ctx.family();
        let (mut c, enc_byte) = self.byte_field(
            input,
            "encoding",
            |b| {
                let (logical, physical) = describe_encoding(family, b);
                format!("0x{b:02X} logical={logical} physical={physical}")
            },
            move |b| encoding_bits02(b, implicit_count, family),
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
        // ALP's parameters ride in the header, after the byte length.
        if matches!(
            stream.meta.encoding.logical,
            LogicalEncoding::Float(FloatLogical::Alp(_))
        ) {
            for name in ["alp_e", "alp_f"] {
                (c, _) = self.field(c, name, |i| parse_varint::<u8>(i), |v| Some(v.to_string()))?;
            }
            (c, _) = self.field(
                c,
                "alp_base",
                |i| parse_varint::<i64>(i),
                |v| Some(v.to_string()),
            )?;
        }
        // So do the Morton grid's, for a vertex dictionary keyed by Morton code.
        if matches!(
            stream.meta.encoding.logical,
            LogicalEncoding::Vertex(VertexLogical::MortonDelta(_))
        ) {
            for name in ["morton_bits", "morton_shift"] {
                (c, _) =
                    self.field(c, name, |i| parse_varint::<u32>(i), |v| Some(v.to_string()))?;
            }
        }
        self.close(hi, c);

        let (after_payload, payload) = take(c, byte_length)?;
        // Consistency guard: the hand re-walk must land exactly on the authoritative tail.
        if self.off(after_payload) != self.off(rest) {
            return Err(MltError::NotImplemented("v2 stream header re-walk desync"));
        }
        // A dictionary's codes and ALP's integers are integer streams, whatever the column's type is.
        let hint = match stream.meta.encoding.logical {
            LogicalEncoding::Float(FloatLogical::Dict) => DecodeHint::U32,
            LogicalEncoding::Float(FloatLogical::Alp(params)) => DecodeHint::Alp(params),
            LogicalEncoding::Float(FloatLogical::None)
            | LogicalEncoding::Int(_)
            | LogicalEncoding::Bool(_)
            | LogicalEncoding::Vertex(_) => hint,
        };
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
        Ok((rest, stream.meta))
    }
}

/// Bit breakdown of a shared-dictionary column's type byte, whose high nibble names the
/// corpus encoding rather than presence.
fn shared_dict_type_bits02(byte: u8) -> Vec<BitField> {
    let (kind, data) = ColumnType02::fields(byte);
    vec![
        BitField {
            hi: 7,
            lo: 4,
            raw: u64::from(kind >> 4),
            meaning: SharedDictKind::parse(kind).map_or_else(
                || "corpus = reserved".to_string(),
                |k| format!("corpus = {k:?}"),
            ),
        },
        BitField {
            hi: 3,
            lo: 0,
            raw: u64::from(data),
            meaning: "data type = SharedDict".to_string(),
        },
    ]
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
        D::Str | D::SharedDict => DecodeHint::Bytes,
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
    let family_name: &'static str = family.into();
    let count = if family == Family::Bytes {
        "has_explicit_count = false -> a blob's byte length is its value count".to_string()
    } else if explicit {
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
            meaning: format!("logical = {name_lo}, numbered for {family_name}"),
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
            meaning: match family {
                Family::Str(layout) => format!("string layout = {layout:?}"),
                Family::Int | Family::Bool | Family::Float | Family::Vertex | Family::Bytes => {
                    format!("extension = {extension}")
                }
            },
        },
    ]
}
