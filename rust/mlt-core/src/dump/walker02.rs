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
use crate::decoder::nested::{
    RowShapes, parse_row_shapes, presence_popcount, reject_shaped_child_presence,
};
use crate::decoder::stream::header02;
use crate::decoder::stream::header02::{
    Count02, Family, HAS_EXPLICIT_COUNT, StrLayout, StreamCtx02, describe_encoding,
};
use crate::decoder::{
    Column02, ColumnType02, DataType02, DictionaryType, GeoLayout, Interior02, LayerLayout,
    LengthType, NodeKind02, NodePresence, NodeType02, Presence02, SharedDictKind, StreamType,
    ValuesColumn02,
};
use crate::tile::{Extent, MAX_NESTED_DEPTH};
use crate::utils::{parse_string, parse_u8, take};
use crate::wire::{
    FloatLogical, IntEncoding, LogicalEncoding, StreamMeta, ValueKind, VertexLogical,
};
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

        if layout.m_values {
            let mi = self.open(input, "m_values".to_string());
            input = self.walk_m_values02(input, feature_count, &shared)?;
            self.close(mi, input);
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
        // Every geometry stream is read against the feature count the header gave.
        let count = Count02::Implied(feature_count);
        let (mut input, _) = self.walk_stream02(
            input,
            StreamCtx02::GeomTypes,
            count,
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
                (input, _) = self.walk_stream02(input, ctx, count, label, DecodeHint::U32)?;
            }
        }

        if layout.is_tess() {
            let ctx = StreamCtx02::GeomOffsets(LengthType::Triangles);
            (input, _) = self.walk_stream02(input, ctx, count, "tri_lengths", DecodeHint::U32)?;
            (input, _) = self.walk_stream02(
                input,
                StreamCtx02::GeomIndices,
                count,
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
            count,
            label,
            DecodeHint::I32,
        )?;
        if layout.is_dict() {
            (input, _) = self.walk_stream02(
                input,
                StreamCtx02::GeomVertexOffsets,
                count,
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
        let typ = match Column02::parse(typ_byte, shared_count)? {
            Column02::SharedDict(kind) => {
                let input = self.walk_shared_dict02(input, ci, i, kind, feature_count, shared)?;
                self.close(ci, input);
                return Ok(input);
            }
            Column02::Values(typ) => typ,
        };
        let column = Column {
            region: ci,
            label: &format!("column[{i}]"),
            feature_count,
            shared,
        };
        let (input, presence_count) = self.walk_column_header02(input, column, typ)?;
        let input = match nested_root02(typ.data) {
            // A nested column's body is a tree of nodes rather than a stream set,
            // trailed by the corpora its string leaves index, where any of them do.
            Some(root) => {
                let count = if root.has_lengths() {
                    Count02::Explicit
                } else {
                    Count02::Implied(presence_count)
                };
                // A root has no node type byte, so the shapes bit rides in one of its own.
                let mut input = input;
                let per_row = input.first() == Some(&NodePresence::SHAPES);
                if per_row {
                    (input, _) = self.byte_field(
                        input,
                        "shapes",
                        |b| format!("0x{b:02X} children coded per row"),
                        root_shapes_bits02,
                    )?;
                }
                self.shared_leaves = 0;
                input = self.walk_nested_body02(input, root, count, per_row, "", 1)?;
                if self.shared_leaves > 0 {
                    let corpus_count;
                    (input, corpus_count) =
                        self.field(input, "corpus_count", parse_varint::<u32>, |c| {
                            Some(c.to_string())
                        })?;
                    for c in 0..corpus_count {
                        input = self.walk_nested_corpus02(input, c)?;
                    }
                }
                input
            }
            None => self.walk_value_streams02(input, typ, Count02::Implied(presence_count))?,
        };
        self.close(ci, input);
        Ok(input)
    }

    /// The corpus streams a shared dictionary ends with, in either of its two encodings.
    ///
    /// They mirror a lone string column's dictionary tail. Nothing implies the entry count,
    /// so each stream writes its own.
    fn walk_dict_tail02(
        &mut self,
        mut input: &'a [u8],
        kind: SharedDictKind,
    ) -> MltResult<&'a [u8]> {
        (input, _) = self.walk_stream02(
            input,
            StreamCtx02::StrDictLengths,
            Count02::Explicit,
            "dict_lengths",
            DecodeHint::U32,
        )?;
        if matches!(kind, SharedDictKind::Fsst | SharedDictKind::CorpusFsst) {
            (input, _) = self.walk_stream02(
                input,
                StreamCtx02::StrSymbolLengths,
                Count02::Explicit,
                "symbol_lengths",
                DecodeHint::U32,
            )?;
            (input, _) = self.walk_stream02(
                input,
                StreamCtx02::StrBlob(DictionaryType::Fsst),
                Count02::Explicit,
                "symbol_table",
                DecodeHint::Bytes,
            )?;
        }
        let name = if matches!(kind, SharedDictKind::Fsst | SharedDictKind::CorpusFsst) {
            "corpus"
        } else {
            "dict_values"
        };
        let (input, _) = self.walk_stream02(
            input,
            StreamCtx02::StrBlob(DictionaryType::Shared),
            Count02::Explicit,
            name,
            DecodeHint::Bytes,
        )?;
        Ok(input)
    }

    /// Mirror `parse_nested_corpus`: one corpus a column's string leaves index by position.
    ///
    /// It reads what a shared-dictionary column's corpus reads, and nothing else.
    fn walk_nested_corpus02(&mut self, input: &'a [u8], i: u32) -> MltResult<&'a [u8]> {
        let di = self.open(input, format!("corpus[{i}]"));
        let (_, typ_byte) = parse_u8(input)?;
        let Column02::SharedDict(kind) = Column02::parse(typ_byte, 0)? else {
            return Err(MltError::ParsingColumnType(typ_byte));
        };
        let (mut input, _) = self.byte_field(
            input,
            "type",
            |b| format!("0x{b:02X} {kind:?} SharedDict"),
            shared_dict_type_bits02,
        )?;
        let name;
        (input, name) = self.field(input, "name", parse_string, |s| Some(format!("{s:?}")))?;
        self.relabel(di, format!("corpus[{i}] {name:?}"));
        input = self.walk_dict_tail02(input, kind)?;
        self.close(di, input);
        Ok(input)
    }

    /// Mirror `parse_interior`: the body of one interior node, in wire order.
    ///
    /// `per_row` says the node codes its children's structure as one shape id per row.
    fn walk_nested_body02(
        &mut self,
        input: &'a [u8],
        kind: Interior02,
        count: Count02,
        per_row: bool,
        path: &str,
        depth: usize,
    ) -> MltResult<&'a [u8]> {
        match kind {
            Interior02::Struct => {
                let (mut input, field_count) =
                    self.field(input, "field_count", parse_varint::<u32>, |c| {
                        Some(c.to_string())
                    })?;
                if field_count == 0 {
                    return Err(MltError::EmptyStructNode);
                }
                let mut shapes = None;
                if per_row {
                    let found;
                    (input, found) = self.walk_row_shapes02(input, field_count, count, path)?;
                    shapes = Some(found);
                }
                for i in 0..field_count {
                    // A shape-coded struct holds every field's presence itself, so each
                    // field stores none and its streams run over the rows that hold it.
                    let count = match &shapes {
                        Some(shapes) => {
                            reject_shaped_child_presence(input, path)?;
                            shapes.child_count(i.into_usize(), count)?
                        }
                        None => count,
                    };
                    input = self.walk_node02(input, &format!("field[{i}]"), true, count, depth)?;
                }
                Ok(input)
            }
            Interior02::List => {
                if per_row {
                    return Err(MltError::NestedRowShapeUnsupported {
                        name: path.to_string(),
                        kind: "list",
                    });
                }
                let ctx = StreamCtx02::NestedLengths;
                let (input, _) =
                    self.walk_stream02(input, ctx, count, "lengths", DecodeHint::U32)?;
                self.walk_node02(input, "element", false, Count02::Explicit, depth)
            }
            // Shape-coded, the key stream holds the distinct key list rather than one
            // key per entry, and a row's entry count is its shape's population count.
            Interior02::Map if per_row => {
                let ki = self.open(input, "keys".to_string());
                let (input, keys) = self.walk_strings02(input, Count02::Explicit)?;
                self.close(ki, input);
                let (input, _) = self.walk_row_shapes02(input, keys, count, path)?;
                self.walk_node02(input, "value", false, Count02::Explicit, depth)
            }
            Interior02::Map => {
                let ctx = StreamCtx02::NestedLengths;
                let (input, _) =
                    self.walk_stream02(input, ctx, count, "lengths", DecodeHint::U32)?;
                let ki = self.open(input, "keys".to_string());
                let (input, _) = self.walk_strings02(input, Count02::Explicit)?;
                self.close(ki, input);
                self.walk_node02(input, "value", false, Count02::Explicit, depth)
            }
        }
    }

    /// Mirror `parse_row_shapes`: the key-set table, then one shape id per row.
    ///
    /// The shapes come back because they say how many rows hold each key, which is
    /// what every child's streams are counted against.
    fn walk_row_shapes02(
        &mut self,
        input: &'a [u8],
        keys: u32,
        rows: Count02,
        path: &str,
    ) -> MltResult<(&'a [u8], RowShapes)> {
        let (_, shapes) = parse_row_shapes(input, keys, rows, path, &mut self.parser)?;
        let count = Count02::Explicit;
        let (input, _) = self.walk_stream02(
            input,
            StreamCtx02::NestedShapeTable,
            count,
            "shape_table",
            DecodeHint::PackedBits,
        )?;
        let (input, _) = self.walk_stream02(
            input,
            StreamCtx02::NestedShapeIds,
            count,
            "shape_ids",
            DecodeHint::U32,
        )?;
        Ok((input, shapes))
    }

    /// Mirror `parse_node`: a node's type byte, its name, its presence stream, then its body.
    fn walk_node02(
        &mut self,
        input: &'a [u8],
        label: &str,
        named: bool,
        parent_count: Count02,
        depth: usize,
    ) -> MltResult<&'a [u8]> {
        if depth >= MAX_NESTED_DEPTH {
            return Err(MltError::NestedTooDeep(depth + 1));
        }
        let ni = self.open(input, label.to_string());
        let (_, typ_byte) = parse_u8(input)?;
        let typ = NodeType02::parse(typ_byte)?;
        let (mut input, _) = self.byte_field(
            input,
            "type",
            |b| {
                format!(
                    "0x{b:02X} {:?} {:?}",
                    typ.presence,
                    DataType02::from(typ.data)
                )
            },
            node_type_bits02,
        )?;
        let mut name_suffix = String::new();
        if named {
            let name;
            (input, name) = self.field(input, "name", parse_string, |s| Some(format!("{s:?}")))?;
            name_suffix = format!(" {name:?}");
        }
        self.relabel(
            ni,
            format!("{label} {:?}{name_suffix}", DataType02::from(typ.data)),
        );

        // A list or a map ends the implied counts, its own streams included.
        let node_count = match typ.data.interior() {
            Some(kind) if kind.has_lengths() => Count02::Explicit,
            _ => parent_count,
        };
        let mut present = node_count;
        if typ.presence.has_stream() {
            let ctx = StreamCtx02::NestedPresence;
            let (_, stream) = header02::parse_stream(input, ctx, node_count, &mut self.parser)?;
            let popcount = presence_popcount(&stream)?;
            (input, _) =
                self.walk_stream02(input, ctx, node_count, "present", DecodeHint::PackedBits)?;
            if node_count != Count02::Explicit {
                present = Count02::Implied(popcount);
            }
        }
        // A shared leaf holds no dictionary of its own: the corpus column it indexes,
        // then its codes into that corpus.
        if typ.presence.is_shared() {
            self.shared_leaves += 1;
            let (mut input, _) = self.field(input, "corpus_index", parse_varint::<u32>, |c| {
                Some(c.to_string())
            })?;
            let ctx = StreamCtx02::StrData(StrLayout::Dict);
            (input, _) = self.walk_stream02(input, ctx, present, "codes", DecodeHint::U32)?;
            self.close(ni, input);
            return Ok(input);
        }
        let input = match typ.data {
            NodeKind02::Leaf(values) => {
                if typ.shapes {
                    return Err(MltError::NestedRowShapeUnsupported {
                        name: label.to_string(),
                        kind: "leaf",
                    });
                }
                let typ = ColumnType02::new(Presence02::AllPresent, values.into());
                self.walk_value_streams02(input, typ, present)?
            }
            NodeKind02::Struct => self.walk_nested_body02(
                input,
                Interior02::Struct,
                present,
                typ.shapes,
                label,
                depth + 1,
            )?,
            NodeKind02::List => self.walk_nested_body02(
                input,
                Interior02::List,
                present,
                typ.shapes,
                label,
                depth + 1,
            )?,
            NodeKind02::Map => self.walk_nested_body02(
                input,
                Interior02::Map,
                present,
                typ.shapes,
                label,
                depth + 1,
            )?,
        };
        self.close(ni, input);
        Ok(input)
    }

    /// Mirror `parse_m_values`: the vertex-scoped columns that end a layer body.
    ///
    /// Each reads what a counted column of its data type reads, over the vertex
    /// sequence rather than the features, so none of their counts are implied.
    fn walk_m_values02(
        &mut self,
        input: &'a [u8],
        feature_count: u32,
        shared: &[&'a BitSlice<u8, Lsb0>],
    ) -> MltResult<&'a [u8]> {
        let (mut input, count) = self.field(input, "m_value_count", parse_varint::<u32>, |c| {
            Some(c.to_string())
        })?;
        if count == 0 {
            return Err(MltError::EmptyMValueSection);
        }
        let shared_count = u8::try_from(shared.len())?;
        for i in 0..count {
            let mi = self.open(input, format!("m_value[{i}]"));
            let (_, typ_byte) = parse_u8(input)?;
            let typ = ValuesColumn02::parse_m_value(typ_byte, shared_count)?.into();
            let column = Column {
                region: mi,
                label: &format!("m_value[{i}]"),
                feature_count,
                shared,
            };
            // An m-value column runs over the vertices, whose count only the geometry
            // knows, so its presence popcount says nothing about its streams, which
            // write their own counts.
            let (rest, _) = self.walk_column_header02(input, column, typ)?;
            input = self.walk_value_streams02(rest, typ, Count02::Explicit)?;
            self.close(mi, input);
        }
        Ok(input)
    }

    /// Walk a column's type byte, then what `parse_column_header` reads: its name
    /// and presence bitfield.
    ///
    /// Returns how many values that presence marks, the count a counted column's
    /// data streams are read against.
    fn walk_column_header02(
        &mut self,
        input: &'a [u8],
        column: Column<'_, 'a>,
        typ: ColumnType02,
    ) -> MltResult<(&'a [u8], u32)> {
        let Column {
            region: ci,
            label,
            feature_count,
            shared,
        } = column;
        let shared_count = u8::try_from(shared.len())?;
        let typ_byte = typ.to_byte();
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
        self.relabel(ci, format!("{label} {opt}{:?}{name_suffix}", typ.data));

        // Presence is a raw LSB0 bitfield, not a stream, and counts the data.
        // A shared bitfield was already walked at the layer root, so only its
        // popcount is needed here.
        let presence_count = match typ.presence {
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
        Ok((input, presence_count))
    }

    /// Walk the data streams a column of `typ` holds, read against `count`.
    fn walk_value_streams02(
        &mut self,
        input: &'a [u8],
        typ: ColumnType02,
        count: Count02,
    ) -> MltResult<&'a [u8]> {
        // A string column has a stream set of its own, the rest one data stream.
        if typ.data == DataType02::Str {
            let (input, _) = self.walk_strings02(input, count)?;
            return Ok(input);
        }

        let ctx = StreamCtx02::Property(typ.data);
        let (mut input, meta) =
            self.walk_stream02(input, ctx, count, "data", hint_for(typ.data))?;

        // A dictionary column's data stream holds codes, and the values follow.
        if meta.encoding.logical == LogicalEncoding::Float(FloatLogical::Dict) {
            let ctx = StreamCtx02::PropertyDictionary(typ.data);
            (input, _) = self.walk_stream02(
                input,
                ctx,
                Count02::Implied(meta.num_values),
                "dictionary",
                hint_for(typ.data),
            )?;
        }
        Ok(input)
    }

    /// Mirror `parse_shared_dict02`: the corpus, then the children that index into it.
    fn walk_shared_dict02(
        &mut self,
        input: &'a [u8],
        ci: usize,
        i: u32,
        kind: SharedDictKind,
        feature_count: u32,
        shared: &[&'a BitSlice<u8, Lsb0>],
    ) -> MltResult<&'a [u8]> {
        let (mut input, _) = self.byte_field(
            input,
            "type",
            |b| format!("0x{b:02X} {kind:?} SharedDict"),
            shared_dict_type_bits02,
        )?;

        let name;
        (input, name) = self.field(input, "name", parse_string, |s| Some(format!("{s:?}")))?;
        self.relabel(ci, format!("column[{i}] SharedDict {name:?}"));
        // A corpus-only column has no children; nodes elsewhere index it by column order.
        let mut child_count = 0;
        if !kind.is_corpus_only() {
            (input, child_count) = self.field(input, "child_count", parse_varint::<u32>, |c| {
                Some(c.to_string())
            })?;
        }

        input = self.walk_dict_tail02(input, kind)?;

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
            (input, suffix) =
                self.field(input, "name", parse_string, |s| Some(format!("{s:?}")))?;
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
                Count02::Implied(count),
                "codes",
                DecodeHint::U32,
            )?;
            self.close(cc, input);
        }
        Ok(input)
    }

    /// Mirror `parse_strings`: the leading stream names the layout the rest of the streams follow.
    fn walk_strings02(&mut self, input: &'a [u8], count: Count02) -> MltResult<(&'a [u8], u32)> {
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

        let (mut input, meta) = self.walk_stream02(
            input,
            StreamCtx02::StrData(layout),
            count,
            leading,
            DecodeHint::U32,
        )?;
        // Every stream after the leading one runs over the values it counted.
        let count = Count02::Implied(meta.num_values);
        for &(ctx, label, hint) in rest {
            (input, _) = self.walk_stream02(input, ctx, count, label, hint)?;
        }
        Ok((input, meta.num_values))
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

    /// Walk one v2 stream: the annotated header (via the authoritative
    /// [`header02::parse_stream`]) followed by the payload blob.
    ///
    /// `ctx`, `count`, and `hint` are all supplied by the caller: none of them are on the wire.
    /// `ctx` also names the family the encoding byte's logical field is read against.
    fn walk_stream02(
        &mut self,
        input: &'a [u8],
        ctx: StreamCtx02,
        count: Count02,
        label: &str,
        hint: DecodeHint,
    ) -> MltResult<(&'a [u8], StreamMeta)> {
        let si = self.open(input, label.to_string());

        // parse -> synthesized meta.
        let (rest, stream) = header02::parse_stream(input, ctx, count, &mut self.parser)?;

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
            move |b| encoding_bits02(b, count, family),
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

/// Where a column of values sits and what it reads its presence against.
#[derive(Clone, Copy)]
struct Column<'l, 'a> {
    /// The region the column's fields are annotated into.
    region: usize,
    /// What to call it, which its data type and name are appended to.
    label: &'l str,
    feature_count: u32,
    /// The layer's shared presence bitfields, one of which the column may read.
    shared: &'l [&'a BitSlice<u8, Lsb0>],
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

/// The interior a nested column's root data type names, or [`None`] for a flat column.
fn nested_root02(typ: DataType02) -> Option<Interior02> {
    use DataType02 as D;
    match typ {
        D::Struct => Some(Interior02::Struct),
        D::List => Some(Interior02::List),
        D::Map => Some(Interior02::Map),
        D::Id
        | D::LongId
        | D::Bool
        | D::I8
        | D::U8
        | D::I32
        | D::U32
        | D::I64
        | D::U64
        | D::F32
        | D::F64
        | D::Str => None,
    }
}

/// Bit breakdown of the byte a shape-coded root carries in place of a node type byte.
fn root_shapes_bits02(byte: u8) -> Vec<BitField> {
    vec![BitField {
        hi: 7,
        lo: 0,
        raw: u64::from(byte),
        meaning: "children coded per row".to_string(),
    }]
}

/// Bit breakdown of a nested node's type byte: node presence (7-4), data type (3-0).
fn node_type_bits02(byte: u8) -> Vec<BitField> {
    let (presence, data) = ColumnType02::fields(byte);
    let name_pr = NodePresence::parse(presence).map_or_else(
        || format!("reserved({})", presence >> 4),
        |p| format!("{p:?}"),
    );
    let name_dt = NodeType02::parse(byte).map_or_else(
        |_| format!("reserved({data})"),
        |t| format!("{:?}", DataType02::from(t.data)),
    );
    let shapes = if byte & NodePresence::SHAPES == 0 {
        ""
    } else {
        " + row shapes"
    };
    vec![
        BitField {
            hi: 7,
            lo: 4,
            raw: u64::from(presence >> 4),
            meaning: format!("node presence = {name_pr}{shapes}"),
        },
        BitField {
            hi: 3,
            lo: 0,
            raw: u64::from(data),
            meaning: format!("data type = {name_dt}"),
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
        // A nested column has no data stream of its own, only the tree below it.
        D::Str | D::Struct | D::List | D::Map => DecodeHint::Bytes,
    }
}

/// Bit breakdown of the v2 layer layout byte:
/// - m-value section flag (7),
/// - shared presence bitfield count (6-4),
/// - geometry layout (3-0).
fn layer_layout_bits02(byte: u8) -> Vec<BitField> {
    let (m_values, shared_presence, geometry) = LayerLayout::fields(byte);
    let name_geo = GeoLayout::try_from(geometry)
        .map_or_else(|_| format!("reserved({geometry})"), |g| format!("{g:?}"));
    let m_values = u64::from(m_values != 0);
    vec![
        BitField {
            hi: 7,
            lo: 7,
            raw: m_values,
            meaning: format!("m-value section = {m_values}"),
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
fn encoding_bits02(byte: u8, count: Count02, family: Family) -> Vec<BitField> {
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
        match count {
            Count02::Implied(count) => {
                format!("has_explicit_count = false -> {count} values from context")
            }
            // `parse_stream` has already rejected the stream this would describe.
            Count02::Explicit => {
                "has_explicit_count = false -> nothing implies a count".to_string()
            }
        }
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
