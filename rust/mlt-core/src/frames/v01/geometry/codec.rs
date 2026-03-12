use crate::MltError::{self, NotImplemented};
use crate::decode::{FromEncoded, impl_decodable};
use crate::encode::{FromDecoded, impl_encodable};
use crate::utils::{AsUsize as _, SetOptionOnce as _};
use crate::v01::geometry::decode::{
    decode_geometry_types, decode_level1_length_stream,
    decode_level1_without_ring_buffer_length_stream, decode_level2_length_stream,
    decode_root_length_stream,
};
use crate::v01::geometry::encode::encode_geometry;
use crate::v01::{
    DecodedGeometry, DictionaryType, EncodedGeometry, Geometry, GeometryEncoder, GeometryType,
    LengthType, OffsetType, OwnedEncodedGeometry, OwnedGeometry, StreamType,
};

impl_decodable!(Geometry<'a>, EncodedGeometry<'a>, DecodedGeometry);
impl_encodable!(OwnedGeometry, DecodedGeometry, OwnedEncodedGeometry);

impl FromDecoded<'_> for OwnedEncodedGeometry {
    type Input = DecodedGeometry;
    type Encoder = GeometryEncoder;

    fn from_decoded(decoded: &Self::Input, encoder: Self::Encoder) -> Result<Self, MltError> {
        encode_geometry(decoded, &encoder, None)
    }
}

impl<'a> FromEncoded<'a> for DecodedGeometry {
    type Input = EncodedGeometry<'a>;

    fn from_encoded(
        EncodedGeometry { meta, items }: EncodedGeometry<'a>,
    ) -> Result<Self, MltError> {
        let vector_types = decode_geometry_types(&meta)?;
        let mut geometry_offsets: Option<Vec<u32>> = None;
        let mut part_offsets: Option<Vec<u32>> = None;
        let mut ring_offsets: Option<Vec<u32>> = None;
        let mut vertex_offsets: Option<Vec<u32>> = None;
        let mut index_buffer: Option<Vec<u32>> = None;
        let mut triangles: Option<Vec<u32>> = None;
        let mut vertices: Option<Vec<i32>> = None;

        for stream in items {
            match stream.meta.stream_type {
                StreamType::Present => {}
                StreamType::Data(v) => match v {
                    DictionaryType::Vertex | DictionaryType::Morton => {
                        let v = stream.decode_bits_u32()?.decode_i32()?;
                        vertices.set_once(v)?;
                    }
                    _ => Err(MltError::UnexpectedStreamType(stream.meta.stream_type))?,
                },
                StreamType::Offset(v) => {
                    let target = match v {
                        OffsetType::Vertex => &mut vertex_offsets,
                        OffsetType::Index => &mut index_buffer,
                        _ => Err(MltError::UnexpectedStreamType(stream.meta.stream_type))?,
                    };
                    target.set_once(FromEncoded::from_encoded(stream)?)?;
                }
                StreamType::Length(v) => {
                    let target = match v {
                        LengthType::Geometries => &mut geometry_offsets,
                        LengthType::Parts => &mut part_offsets,
                        LengthType::Rings => &mut ring_offsets,
                        LengthType::Triangles => &mut triangles,
                        _ => Err(MltError::UnexpectedStreamType(stream.meta.stream_type))?,
                    };
                    // LogicalStream2<U> -> LogicalStream -> trait LogicalStreamEncoding<T>
                    target.set_once(FromEncoded::from_encoded(stream)?)?;
                }
            }
        }

        if index_buffer.is_some() && part_offsets.is_none() {
            // Case when the indices of a Polygon outline are not encoded in the data so no
            // topology data are present in the tile
            //
            // return FlatGpuVector::new(vector_types, triangles, index_buffer, vertices);
            return Err(NotImplemented(
                "index_buffer.is_some() && part_offsets.is_none() case",
            ));
        }

        // Use decode_root_length_stream if geometry_offsets is present
        if let Some(offsets) = geometry_offsets.take() {
            geometry_offsets = Some(decode_root_length_stream(
                &vector_types,
                &offsets,
                GeometryType::Polygon,
            ));
            if let Some(part_offsets_copy) = part_offsets.take() {
                if let Some(ring_offsets_copy) = ring_offsets.take() {
                    part_offsets = Some(decode_level1_length_stream(
                        &vector_types,
                        geometry_offsets.as_ref().unwrap(),
                        &part_offsets_copy,
                        false, // isLineStringPresent
                    ));
                    ring_offsets = Some(decode_level2_length_stream(
                        &vector_types,
                        geometry_offsets.as_ref().unwrap(),
                        part_offsets.as_ref().unwrap(),
                        &ring_offsets_copy,
                    ));
                } else {
                    part_offsets = Some(decode_level1_without_ring_buffer_length_stream(
                        &vector_types,
                        geometry_offsets.as_ref().unwrap(),
                        &part_offsets_copy,
                    ));
                }
            }
        } else if let Some(offsets) = part_offsets.take() {
            if let Some(ring_offsets_copy) = ring_offsets.take() {
                let is_line_string_present = vector_types.iter().any(|t| t.is_linestring());
                part_offsets = Some(decode_root_length_stream(
                    &vector_types,
                    &offsets,
                    GeometryType::LineString,
                ));
                ring_offsets = Some(decode_level1_length_stream(
                    &vector_types,
                    part_offsets.as_ref().unwrap(),
                    &ring_offsets_copy,
                    is_line_string_present,
                ));
            } else {
                part_offsets = Some(decode_root_length_stream(
                    &vector_types,
                    &offsets,
                    GeometryType::Point,
                ));
            }
        }

        // Case when the indices of a Polygon outline are encoded in the tile
        // This is handled by including index_buffer in the DecodedGeometry

        // Expand vertex dictionary:
        // If a vertex offset stream was present,
        // - `vertices` holds only the unique dictionary entries and
        // - `vertex_offsets` holds per-vertex indices into it.
        //
        // Expand them into a single flat (x, y) sequence so that `DecodedGeometry` always
        // represents fully decoded data, regardless of the encoding that was used.
        if let Some(offsets) = vertex_offsets.take()
            && let Some(dict) = vertices.as_deref()
        {
            vertices = Some(
                offsets
                    .iter()
                    .flat_map(|&i| {
                        let i = i.as_usize();
                        [dict[i * 2], dict[i * 2 + 1]]
                    })
                    .collect(),
            );
        }

        Ok(DecodedGeometry {
            vector_types,
            geometry_offsets,
            part_offsets,
            ring_offsets,
            index_buffer,
            triangles,
            vertices,
        })
    }
}
