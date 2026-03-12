use std::fmt::Debug;
use std::io::Write;

use integer_encoding::VarIntWriter as _;

use crate::MltError;
use crate::decode::Decode as _;
use crate::utils::{AsUsize as _, BinarySerializer as _, OptSeq, checked_sum2, parse_varint};
use crate::v01::{
    ColumnType, DecodedGeometry, DictionaryType, EncodedGeometry, Geometry, IntEncoding,
    OwnedEncodedGeometry, OwnedGeometry, Stream, StreamData, StreamMeta, StreamType,
};

impl OwnedGeometry {
    #[doc(hidden)]
    pub fn write_columns_meta_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        match self {
            Self::Encoded(_) => OwnedEncodedGeometry::write_columns_meta_to(writer),
            Self::Decoded(_) => Err(MltError::NeedsEncodingBeforeWriting),
        }
    }

    #[doc(hidden)]
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
        match self {
            Self::Encoded(r) => r.write_to(writer),
            Self::Decoded(_) => Err(MltError::NeedsEncodingBeforeWriting),
        }
    }
}

impl<'a> EncodedGeometry<'a> {
    /// Parse encoded geometry from bytes (expects varint stream count + streams)
    pub fn parse(input: &'a [u8]) -> crate::MltRefResult<'a, Self> {
        let (input, stream_count) = parse_varint::<u32>(input)?;
        let stream_count = stream_count.as_usize();
        if stream_count == 0 {
            return Ok((
                input,
                Self {
                    meta: Stream::new(
                        StreamMeta::new(
                            StreamType::Data(DictionaryType::None),
                            IntEncoding::none(),
                            0,
                        ),
                        StreamData::Encoded(&[]),
                    ),
                    items: Vec::new(),
                },
            ));
        }

        let (input, meta) = Stream::parse(input)?;
        let (input, items) = Stream::parse_multiple(input, stream_count - 1)?;

        Ok((input, Self { meta, items }))
    }
}

impl OwnedEncodedGeometry {
    /// Decode this encoded geometry into its decoded form.
    pub fn decode(&self) -> Result<DecodedGeometry, MltError> {
        DecodedGeometry::decode(borrowme::borrow(self))
    }

    pub(crate) fn write_columns_meta_to<W: Write>(writer: &mut W) -> Result<(), MltError> {
        ColumnType::Geometry.write_to(writer)?;
        Ok(())
    }

    pub(crate) fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), MltError> {
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

impl Debug for DecodedGeometry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let DecodedGeometry {
            vector_types,
            geometry_offsets,
            part_offsets,
            ring_offsets,
            index_buffer,
            triangles,
            vertices,
        } = self;
        f.debug_struct("DecodedGeometry")
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
    pub fn new_encoded(meta: Stream<'a>, items: Vec<Stream<'a>>) -> Self {
        Self::Encoded(EncodedGeometry { meta, items })
    }

    #[inline]
    pub fn decode(self) -> Result<DecodedGeometry, MltError> {
        Ok(match self {
            Self::Encoded(v) => DecodedGeometry::decode(v)?,
            Self::Decoded(v) => v,
        })
    }
}

impl DecodedGeometry {
    #[must_use]
    pub fn feature_count(&self) -> usize {
        self.vector_types.len()
    }
}
