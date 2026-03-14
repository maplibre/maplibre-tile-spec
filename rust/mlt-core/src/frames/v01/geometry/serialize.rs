use std::fmt::Debug;
use std::io::Write;

use integer_encoding::VarIntWriter as _;

use crate::MltError;
use crate::decode::Decode as _;
use crate::utils::{AsUsize as _, BinarySerializer as _, OptSeq, checked_sum2, parse_varint};
use crate::v01::{
    ColumnType, DictionaryType, EncodedGeometry, Geometry, GeometryValues, IntEncoding,
    RawGeometry, RawStream, RawStreamData, StreamMeta, StreamType,
};

impl<'a> RawGeometry<'a> {
    /// Parse encoded geometry from bytes (expects varint stream count + streams)
    pub fn parse(input: &'a [u8]) -> crate::MltRefResult<'a, Self> {
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

        let (input, meta) = RawStream::parse(input)?;
        let (input, items) = RawStream::parse_multiple(input, stream_count - 1)?;

        Ok((input, Self { meta, items }))
    }
}

impl TryFrom<RawGeometry<'_>> for GeometryValues {
    type Error = MltError;

    fn try_from(raw: RawGeometry<'_>) -> Result<Self, MltError> {
        GeometryValues::decode(raw)
    }
}

/// Decode from a wire-ready [`EncodedGeometry`], borrowing its byte buffers.
///
/// This conversion is not part of the standard decoding pipeline (`Raw* → Values`),
/// but is provided for round-trip testing and internal use (e.g., `canonicalize_geometry`).
impl TryFrom<EncodedGeometry> for GeometryValues {
    type Error = MltError;

    fn try_from(encoded: EncodedGeometry) -> Result<Self, MltError> {
        let meta = encoded.meta.as_borrowed();
        let items: Vec<_> = encoded.items.iter().map(|s| s.as_borrowed()).collect();
        GeometryValues::decode(RawGeometry { meta, items })
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
    pub fn new_encoded(meta: RawStream<'a>, items: Vec<RawStream<'a>>) -> Self {
        Self::Encoded(RawGeometry { meta, items })
    }

    #[inline]
    pub fn decode(self) -> Result<GeometryValues, MltError> {
        Ok(match self {
            Self::Encoded(v) => GeometryValues::decode(v)?,
            Self::Decoded(v) => v,
        })
    }
}

impl GeometryValues {
    #[must_use]
    pub fn feature_count(&self) -> usize {
        self.vector_types.len()
    }
}
