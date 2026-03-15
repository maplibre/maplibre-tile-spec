use std::fmt::Debug;
use std::io::Write;

use integer_encoding::VarIntWriter as _;

use crate::utils::{AsUsize as _, BinarySerializer as _, OptSeq, checked_sum2, parse_varint};
use crate::v01::geometry::encode::encode_geometry;
use crate::v01::{
    ColumnType, DictionaryType, EncodedGeometry, Geometry, GeometryEncoder, GeometryValues,
    IntEncoding, RawGeometry, RawStream, RawStreamData, StreamMeta, StreamType,
};
use crate::{MemBudget, MltError};

impl<'a> RawGeometry<'a> {
    /// Parse encoded geometry from bytes (expects varint stream count + streams).
    /// Reserves decoded memory against `budget`.
    pub fn from_bytes(input: &'a [u8], budget: &mut MemBudget) -> crate::MltRefResult<'a, Self> {
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

        let (input, meta) = RawStream::from_bytes(input, budget)?;
        let (input, items) = RawStream::parse_multiple(input, stream_count - 1, budget)?;

        Ok((input, Self { meta, items }))
    }
}

impl EncodedGeometry {
    pub fn write_columns_meta_to<W: Write>(writer: &mut W) -> Result<(), MltError> {
        ColumnType::Geometry.write_to(writer)?;
        Ok(())
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        let items_len = u32::try_from(self.items.len())?;
        let items_len = checked_sum2(items_len, 1)?;
        writer.write_varint(items_len)?;
        writer.write_stream(&self.meta)?;
        for item in &self.items {
            writer.write_stream(item)?;
        }
        Ok(())
    }

    pub(crate) fn encode(
        value: &GeometryValues,
        encoder: GeometryEncoder,
    ) -> Result<Self, MltError> {
        encode_geometry(value, &encoder, None)
    }
}

impl Debug for GeometryValues {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let GeometryValues {
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

impl<'a> Geometry<'a> {
    #[must_use]
    pub fn new_raw(meta: RawStream<'a>, items: Vec<RawStream<'a>>) -> Self {
        Self::Raw(RawGeometry { meta, items })
    }
}

impl GeometryValues {
    #[must_use]
    pub fn feature_count(&self) -> usize {
        self.vector_types.len()
    }
}
