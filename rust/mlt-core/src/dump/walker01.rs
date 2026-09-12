//! Annotating walker for the tag `0x01` layer body.

use usize_cast::IntoUsize as _;

use super::model::{BitField, BlobInfo, DecodeHint};
use super::walker::Walker;
use crate::codecs::varint::parse_varint;
use crate::decoder::stream::header01::{PhysicalField, parse_stream_meta};
use crate::decoder::{Column, ColumnType, DictionaryType, StreamType};
use crate::utils::{parse_string, take};
use crate::wire::{
    BoolLogical, IntLogical, LogicalEncoding, LogicalTechnique, StreamMeta, ValueKind,
    VertexLogical,
};
use crate::{MltError, MltResult};

impl<'a> Walker<'a> {
    /// Mirror [`crate::decoder::Layer01::from_bytes`].
    /// `body` must be consumed fully.
    pub(super) fn walk_layer01(&mut self, input: &'a [u8]) -> MltResult<()> {
        let (input, _name) = self.field(input, "name", parse_string, |s| Some(format!("{s:?}")))?;
        let (input, _extent) = self.field(
            input,
            "extent",
            |i| parse_varint::<u32>(i),
            |v| Some(v.to_string()),
        )?;
        let (input, column_count) = self.field(
            input,
            "column_count",
            |i| parse_varint::<u32>(i),
            |v| Some(v.to_string()),
        )?;

        let (mut input, columns) = self.walk_schema(input, column_count)?;

        if !columns.is_empty() {
            let di = self.open(input, "column data".to_string());
            for (ci, col) in columns.iter().enumerate() {
                input = self.walk_column_data(input, ci, col)?;
            }
            self.close(di, input);
        }

        // A well-formed layer consumes its whole body; record any trailing bytes.
        if !input.is_empty() {
            self.raw_blob(input, input.len(), "trailing bytes".to_string());
        }
        Ok(())
    }

    /// Mirror `parse_columns_meta`: `column_count` column definitions.
    fn walk_schema(
        &mut self,
        mut input: &'a [u8],
        column_count: u32,
    ) -> MltResult<(&'a [u8], Vec<Column<'a>>)> {
        let si = self.open(input, "schema".to_string());
        if input.len() < column_count.into_usize() {
            return Err(MltError::BufferUnderflow(column_count, input.len()));
        }
        let mut cols = Vec::with_capacity(column_count.into_usize());
        for i in 0..column_count {
            let (rest, col) = self.walk_column_def(input, i)?;
            input = rest;
            cols.push(col);
        }
        self.close(si, input);
        Ok((input, cols))
    }

    /// Mirror `Column::from_bytes` (plus inline `SharedDict` children), split into
    /// `[type u8][optional name]` (and child defs).
    fn walk_column_def(&mut self, input: &'a [u8], i: u32) -> MltResult<(&'a [u8], Column<'a>)> {
        let ci = self.open(input, format!("column[{i}]"));

        // Column-type byte, with the optional-flag bit broken out.
        let (after_ty, typ) = ColumnType::from_bytes(input)?;
        let byte = typ as u8;
        let bits = vec![
            BitField {
                hi: 7,
                lo: 1,
                raw: u64::from(byte >> 1),
                meaning: format!("base type = {typ:?}"),
            },
            BitField {
                hi: 0,
                lo: 0,
                raw: u64::from(byte & 1),
                meaning: format!("optional = {}", typ.is_optional()),
            },
        ];
        self.leaf_bits(
            input,
            after_ty,
            "type".to_string(),
            Some(format!("0x{byte:02X} {typ:?}")),
            bits,
        );
        let mut input = after_ty;

        let name = if typ.has_name() {
            let (rest, name) =
                self.field(input, "name", parse_string, |s| Some(format!("{s:?}")))?;
            input = rest;
            Some(name)
        } else {
            None
        };

        let mut children = Vec::new();
        if typ == ColumnType::SharedDict {
            let (rest, child_count) = self.field(
                input,
                "child_count",
                |i| parse_varint::<u32>(i),
                |v| Some(v.to_string()),
            )?;
            input = rest;
            if input.len() < child_count.into_usize() {
                return Err(MltError::BufferUnderflow(child_count, input.len()));
            }
            children.reserve(child_count.into_usize());
            for j in 0..child_count {
                let (rest, child) = self.walk_column_def(input, j)?;
                input = rest;
                children.push(child);
            }
        }

        self.close(ci, input);
        Ok((
            input,
            Column {
                typ,
                name,
                children,
            },
        ))
    }

    fn walk_column_data(
        &mut self,
        input: &'a [u8],
        ci: usize,
        col: &Column<'a>,
    ) -> MltResult<&'a [u8]> {
        use ColumnType as C;
        let typ = col.typ;
        let name_suffix = col.name.map(|n| format!(" {n:?}")).unwrap_or_default();
        let gi = self.open(input, format!("column[{ci}] {typ:?}{name_suffix}"));

        let mut input = input;
        match typ {
            C::Id | C::OptId => {
                input = self.walk_optional(input, typ)?;
                input = self
                    .walk_stream(input, ValueKind::Int, "id", |_| DecodeHint::U32)?
                    .0;
            }
            C::LongId | C::OptLongId => {
                input = self.walk_optional(input, typ)?;
                input = self
                    .walk_stream(input, ValueKind::Int, "id", |_| DecodeHint::U64)?
                    .0;
            }
            C::Geometry => {
                input = self.walk_geometry(input)?;
            }
            C::Bool | C::OptBool => {
                input = self.walk_optional(input, typ)?;
                input = self
                    .walk_stream(input, ValueKind::Bool, "data", |_| DecodeHint::Bool)?
                    .0;
            }
            C::I8 | C::OptI8 | C::I32 | C::OptI32 => {
                input = self.walk_optional(input, typ)?;
                input = self
                    .walk_stream(input, ValueKind::Int, "data", |_| DecodeHint::I32)?
                    .0;
            }
            C::U8 | C::OptU8 | C::U32 | C::OptU32 => {
                input = self.walk_optional(input, typ)?;
                input = self
                    .walk_stream(input, ValueKind::Int, "data", |_| DecodeHint::U32)?
                    .0;
            }
            C::I64 | C::OptI64 => {
                input = self.walk_optional(input, typ)?;
                input = self
                    .walk_stream(input, ValueKind::Int, "data", |_| DecodeHint::I64)?
                    .0;
            }
            C::U64 | C::OptU64 => {
                input = self.walk_optional(input, typ)?;
                input = self
                    .walk_stream(input, ValueKind::Int, "data", |_| DecodeHint::U64)?
                    .0;
            }
            C::F32 | C::OptF32 => {
                input = self.walk_optional(input, typ)?;
                input = self
                    .walk_stream(input, ValueKind::Float, "data", |_| DecodeHint::F32)?
                    .0;
            }
            C::F64 | C::OptF64 => {
                input = self.walk_optional(input, typ)?;
                input = self
                    .walk_stream(input, ValueKind::Float, "data", |_| DecodeHint::F64)?
                    .0;
            }
            C::Str | C::OptStr => {
                input = self.walk_str(input, typ)?;
            }
            C::SharedDict => {
                input = self.walk_shared_dict(input, col)?;
            }
        }

        self.close(gi, input);
        Ok(input)
    }

    /// Mirror `parse_optional`: a boolean presence stream iff the column is optional.
    fn walk_optional(&mut self, input: &'a [u8], typ: ColumnType) -> MltResult<&'a [u8]> {
        if typ.is_optional() {
            Ok(self
                .walk_stream(input, ValueKind::Bool, "present", |_| DecodeHint::Presence)?
                .0)
        } else {
            Ok(input)
        }
    }

    /// Mirror `parse_geometry_column`: `[varint stream_count]` + meta stream + rest.
    fn walk_geometry(&mut self, input: &'a [u8]) -> MltResult<&'a [u8]> {
        let (mut input, stream_count) = self.field(
            input,
            "stream_count",
            |i| parse_varint::<u32>(i),
            |v| Some(v.to_string()),
        )?;
        if stream_count == 0 {
            return Err(MltError::GeometryWithoutStreams);
        }
        input = self
            .walk_stream(input, ValueKind::Int, "meta", geom_hint)?
            .0;
        for j in 0..stream_count - 1 {
            input = self
                .walk_stream(input, ValueKind::Int, &format!("stream[{j}]"), geom_hint)?
                .0;
        }
        Ok(input)
    }

    /// Mirror `parse_str_column`: `[varint stream_count]`, optional presence, then
    /// 2-5 data/offset/length streams.
    fn walk_str(&mut self, input: &'a [u8], typ: ColumnType) -> MltResult<&'a [u8]> {
        let (mut input, stream_count) = self.field(
            input,
            "stream_count",
            |i| parse_varint::<u32>(i),
            |v| Some(v.to_string()),
        )?;
        let mut remaining = stream_count.into_usize();
        if typ.is_optional() {
            if remaining == 0 {
                return Err(MltError::UnsupportedStringStreamCount(remaining));
            }
            input = self
                .walk_stream(input, ValueKind::Bool, "present", |_| DecodeHint::Presence)?
                .0;
            remaining -= 1;
        }
        for j in 0..remaining {
            input = self
                .walk_stream(input, ValueKind::Int, &format!("stream[{j}]"), auto_hint)?
                .0;
        }
        Ok(input)
    }

    /// Mirror `parse_shared_dict_column` + `parse_shared_dict_children`.
    fn walk_shared_dict(&mut self, input: &'a [u8], col: &Column<'a>) -> MltResult<&'a [u8]> {
        let (mut input, _stream_count) = self.field(
            input,
            "stream_count",
            |i| parse_varint::<u32>(i),
            |v| Some(v.to_string()),
        )?;

        // Dictionary streams: read until the DATA(Single|Shared) stream.
        let mut taken = 0usize;
        loop {
            let (rest, meta) = self.walk_stream(
                input,
                ValueKind::Int,
                &format!("dict_stream[{taken}]"),
                auto_hint,
            )?;
            input = rest;
            taken += 1;
            if matches!(
                meta.stream_type,
                StreamType::Data(DictionaryType::Single | DictionaryType::Shared)
            ) {
                break;
            }
            if taken >= 5 {
                return Err(MltError::UnsupportedStringStreamCount(taken + 1));
            }
        }

        // Children: each `[varint stream_count][optional present][data stream]`.
        for (j, child) in col.children.iter().enumerate() {
            let cci = self.open(input, format!("child[{j}] {:?}", child.typ));
            let (rest, _sc) = self.field(
                input,
                "stream_count",
                |i| parse_varint::<u32>(i),
                |v| Some(v.to_string()),
            )?;
            input = rest;
            if child.typ.is_optional() {
                input = self
                    .walk_stream(input, ValueKind::Bool, "present", |_| DecodeHint::Presence)?
                    .0;
            }
            input = self
                .walk_stream(input, ValueKind::Int, "data", auto_hint)?
                .0;
            self.close(cci, input);
        }
        Ok(input)
    }

    /// Walk one stream: the annotated header (via the authoritative
    /// [`parse_stream_meta`]) followed by the payload blob.
    fn walk_stream(
        &mut self,
        input: &'a [u8],
        kind: ValueKind,
        label: &str,
        hint: impl FnOnce(StreamType) -> DecodeHint,
    ) -> MltResult<(&'a [u8], StreamMeta)> {
        let si = self.open(input, label);
        let is_bool = kind == ValueKind::Bool;

        // Authoritative parse - drives advancement and gives us `meta`/`byte_length`.
        let (after_hdr, (meta, byte_length)) =
            parse_stream_meta(input, kind, is_bool, &mut self.parser)?;

        // Re-walk the consumed header bytes to annotate each field.
        let hi = self.open(input, "header");
        let mut c = input;

        (c, _) = self.byte_field(
            c,
            "stream_type",
            |b| format!("0x{b:02X} {:?}", meta.stream_type),
            |b| stream_type_bits(meta.stream_type, b),
        )?;
        (c, _) = self.byte_field(
            c,
            "encoding",
            |b| {
                format!(
                    "0x{b:02X} logical={:?} physical={:?}",
                    meta.encoding.logical, meta.encoding.physical
                )
            },
            encoding_bits,
        )?;

        (c, _) = self.field(
            c,
            "num_values",
            |i| parse_varint::<u32>(i),
            |v| Some(v.to_string()),
        )?;
        (c, _) = self.field(
            c,
            "byte_length",
            |i| parse_varint::<u32>(i),
            |v| Some(v.to_string()),
        )?;

        match meta.encoding.logical {
            LogicalEncoding::Int(IntLogical::Rle(_) | IntLogical::DeltaRle(_))
            | LogicalEncoding::Bool(BoolLogical::ByteRle(_))
                if !is_bool =>
            {
                (c, _) = self.field(
                    c,
                    "runs",
                    |i| parse_varint::<u32>(i),
                    |v| Some(v.to_string()),
                )?;
                (c, _) = self.field(
                    c,
                    "num_rle_values",
                    |i| parse_varint::<u32>(i),
                    |v| Some(v.to_string()),
                )?;
            }
            LogicalEncoding::Vertex(
                VertexLogical::Morton(_)
                | VertexLogical::MortonDelta(_)
                | VertexLogical::MortonRle(_),
            ) => {
                (c, _) = self.field(
                    c,
                    "bits",
                    |i| parse_varint::<u32>(i),
                    |v| Some(v.to_string()),
                )?;
                (c, _) = self.field(
                    c,
                    "shift",
                    |i| parse_varint::<u32>(i),
                    |v| Some(v.to_string()),
                )?;
            }
            LogicalEncoding::Int(_)
            | LogicalEncoding::Bool(_)
            | LogicalEncoding::Float(_)
            | LogicalEncoding::Vertex(
                VertexLogical::None | VertexLogical::Delta | VertexLogical::ComponentwiseDelta,
            ) => {}
        }
        self.close(hi, c);

        // Consistency guard: the hand re-walk must land exactly on the authoritative tail.
        if self.off(c) != self.off(after_hdr) {
            return Err(MltError::NotImplemented("stream header re-walk desync"));
        }

        let (rest, payload) = take(after_hdr, byte_length)?;
        self.stream_blob(
            payload,
            payload.len(),
            "data",
            BlobInfo {
                meta,
                hint: hint(meta.stream_type),
            },
        );

        self.close(si, rest);
        Ok((rest, meta))
    }
}

/// Decode hint for opaque string / dictionary streams, keyed by stream type.
fn auto_hint(st: StreamType) -> DecodeHint {
    match st {
        StreamType::Present => DecodeHint::Presence,
        StreamType::Offset(_) | StreamType::Length(_) => DecodeHint::U32,
        StreamType::Data(_) => DecodeHint::Bytes,
    }
}

/// Decode hint for geometry streams: vertex/data are signed (zigzag / componentwise
/// / morton), while offsets and lengths are unsigned counts.
fn geom_hint(st: StreamType) -> DecodeHint {
    match st {
        StreamType::Present => DecodeHint::Presence,
        StreamType::Offset(_) | StreamType::Length(_) => DecodeHint::U32,
        StreamType::Data(_) => DecodeHint::I32,
    }
}

/// Bit breakdown of the `stream_type` byte: category nibble + subtype nibble.
fn stream_type_bits(st: StreamType, byte: u8) -> Vec<BitField> {
    let category = match st {
        StreamType::Present => "Present",
        StreamType::Data(_) => "Data",
        StreamType::Offset(_) => "Offset",
        StreamType::Length(_) => "Length",
    };
    let subtype = match st {
        StreamType::Present => "-".to_string(),
        StreamType::Data(d) => format!("{d:?}"),
        StreamType::Offset(o) => format!("{o:?}"),
        StreamType::Length(l) => format!("{l:?}"),
    };
    vec![
        BitField {
            hi: 7,
            lo: 4,
            raw: u64::from(byte >> 4),
            meaning: format!("category = {category}"),
        },
        BitField {
            hi: 3,
            lo: 0,
            raw: u64::from(byte & 0x0F),
            meaning: format!("subtype = {subtype}"),
        },
    ]
}

/// Bit breakdown of the `encoding` byte: logical1 (7-5), logical2 (4-2), physical (1-0).
fn encoding_bits(byte: u8) -> Vec<BitField> {
    let l1 = byte >> 5;
    let l2 = (byte >> 2) & 0x7;
    let ph = byte & 0x3;
    // The `LogicalTechnique` repr is pre-shifted into the logical1 field, so name both from there.
    let name_lt = |v: u8| {
        LogicalTechnique::try_from(v << 5)
            .map_or_else(|_| format!("invalid({v})"), |t| format!("{t:?}"))
    };
    let name_ph = PhysicalField::parse(byte).map_or_else(
        |_| format!("invalid({ph})"),
        |p| <&str>::from(p).to_string(),
    );
    vec![
        BitField {
            hi: 7,
            lo: 5,
            raw: u64::from(l1),
            meaning: format!("logical1 = {}", name_lt(l1)),
        },
        BitField {
            hi: 4,
            lo: 2,
            raw: u64::from(l2),
            meaning: format!("logical2 = {}", name_lt(l2)),
        },
        BitField {
            hi: 1,
            lo: 0,
            raw: u64::from(ph),
            meaning: format!("physical = {name_ph}"),
        },
    ]
}
