use std::fmt::Debug;
use std::io::Write;

use integer_encoding::VarIntWriter as _;

use super::encode::{GeometryEncoder, encode_geometry};
use super::model::EncodedGeometry;
use crate::codecs::varint::parse_varint;
use crate::decoder::{
    ColumnType, DictionaryType, GeometryValues, IntEncoding, RawGeometry, RawStream, RawStreamData,
    StreamMeta, StreamType,
};
use crate::utils::{AsUsize as _, BinarySerializer as _, OptSeq, checked_sum2};
use crate::{MltResult, Parser};

impl<'a> RawGeometry<'a> {
    /// Parse encoded geometry from bytes (expects varint stream count + streams).
    /// Reserves decoded memory against the parser's budget.
    pub fn from_bytes(input: &'a [u8], parser: &mut Parser) -> crate::MltRefResult<'a, Self> {
        let (input, stream_count) = parse_varint::<u32>(input)?;
        let stream_count = stream_count.as_usize();
        if stream_count == 0 {
            return Ok((
                input,
                Self {
                    meta: RawStream::new(
                        StreamMeta::new(
                            StreamType::Data(DictionaryType::None),
                            IntEncoding::none(),
                            0,
                        ),
                        RawStreamData::Encoded(&[]),
                    ),
                    items: Vec::new(),
                },
            ));
        }

        let (input, meta) = RawStream::from_bytes(input, parser)?;
        let (input, items) = RawStream::parse_multiple(input, stream_count - 1, parser)?;

        Ok((input, Self { meta, items }))
    }
}

impl EncodedGeometry {
    pub fn write_columns_meta_to<W: Write>(writer: &mut W) -> MltResult<()> {
        ColumnType::Geometry.write_to(writer)?;
        Ok(())
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> MltResult<()> {
        let items_len = u32::try_from(self.items.len())?;
        let items_len = checked_sum2(items_len, 1)?;
        writer.write_varint(items_len)?;
        writer.write_stream(&self.meta)?;
        for item in &self.items {
            writer.write_stream(item)?;
        }
        Ok(())
    }

    pub fn encode(value: &GeometryValues, encoder: GeometryEncoder) -> MltResult<Self> {
        encode_geometry(value, &encoder, None)
    }
}

impl Debug for GeometryValues {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            vector_types,
            geometry_offsets,
            part_offsets,
            ring_offsets,
            index_buffer,
            triangles,
            vertices,
        } = self;
        f.debug_struct("GeometryValues")
            .field("vector_types", &OptSeq(Some(vector_types)))
            .field("geometry_offsets", &OptSeq(geometry_offsets.as_deref()))
            .field("part_offsets", &OptSeq(part_offsets.as_deref()))
            .field("ring_offsets", &OptSeq(ring_offsets.as_deref()))
            .field("index_buffer", &OptSeq(index_buffer.as_deref()))
            .field("triangles", &OptSeq(triangles.as_deref()))
            .field("vertices", &OptSeq(vertices.as_deref()))
            .finish()
    }
}
