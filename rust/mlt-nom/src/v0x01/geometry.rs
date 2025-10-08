use borrowme::borrowme;

use crate::MltError;
use crate::utils::SetOptionOnce;
use crate::v0x01::{
    DictionaryType, GeometryType, LengthType, OffsetType, Parsable, PhysicalStreamType, Stream,
    impl_decodable,
};

/// Unparsed geometry data as read directly from the tile
#[borrowme]
#[derive(Debug, PartialEq)]
pub enum Geometry<'a> {
    Raw(RawGeometry<'a>),
    Decoded(DecodedGeometry),
}

impl<'a> Geometry<'a> {
    #[must_use]
    pub fn raw(meta: Stream<'a>, items: Vec<Stream<'a>>) -> Self {
        Self::Raw(RawGeometry { meta, items })
    }
}

impl_decodable!(Geometry<'a>, RawGeometry<'a>, DecodedGeometry);

#[borrowme]
#[derive(Debug, PartialEq)]
pub struct RawGeometry<'a> {
    meta: Stream<'a>,
    items: Vec<Stream<'a>>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct DecodedGeometry {
    // pub vector_type: VectorType,
    // pub vertex_buffer_type: VertexBufferType,
    pub vector_types: Vec<GeometryType>,
    pub geometry_offsets: Option<Vec<u32>>,
    pub part_offsets: Option<Vec<u32>>,
    pub ring_offsets: Option<Vec<u32>>,
    pub vertex_offsets: Option<Vec<u32>>,
    pub index_buffer: Option<Vec<u32>>,
    pub triangles: Option<Vec<u32>>,
    pub vertices: Option<Vec<i32>>,
}

impl<'a> Parsable<'a> for DecodedGeometry {
    type Input = RawGeometry<'a>;

    fn parse(RawGeometry { meta, items }: RawGeometry<'a>) -> Result<Self, MltError> {
        // let vector_type = Self::get_vector_type_int_stream(&meta);
        let vector_types = decode_geometry_types(meta)?;
        let mut geometry_offsets: Option<Vec<u32>> = None;
        let mut part_offsets: Option<Vec<u32>> = None;
        let mut ring_offsets: Option<Vec<u32>> = None;
        let mut vertex_offsets: Option<Vec<u32>> = None;
        let mut index_buffer: Option<Vec<u32>> = None;
        let mut triangles: Option<Vec<u32>> = None;
        let mut vertices: Option<Vec<i32>> = None;

        for stream in items {
            match stream.meta.physical_type {
                PhysicalStreamType::Present => {}
                PhysicalStreamType::Data(v) => match v {
                    DictionaryType::Vertex => {
                        let v = stream.decode_bits_u32()?.decode_i32()?;
                        vertices.set_once(v)?;
                    }
                    _ => todo!("Geometry stream cannot have Data physical type: {v:?}"),
                },
                PhysicalStreamType::Offset(v) => {
                    let target = match v {
                        OffsetType::Vertex => &mut vertex_offsets,
                        OffsetType::Index => &mut index_buffer,
                        _ => todo!("Geometry stream cannot have Offset physical type: {v:?}"),
                    };
                    target.set_once(stream.decode_bits_u32()?.decode_u32()?)?;
                }
                PhysicalStreamType::Length(v) => {
                    let target = match v {
                        LengthType::Geometries => &mut geometry_offsets,
                        LengthType::Parts => &mut part_offsets,
                        LengthType::Rings => &mut ring_offsets,
                        LengthType::Triangles => &mut triangles,
                        _ => todo!("Geometry stream cannot have Length physical type: {v:?}"),
                    };
                    // LogicalStream2<U> -> LogicalStream -> trait LogicalStreamDecoder<T>
                    target.set_once(stream.decode_bits_u32()?.decode_u32()?)?;
                }
            }
        }
        if index_buffer.is_some() && part_offsets.is_none() {
            // Case when the indices of a Polygon outline are not encoded in the data so no
            // topology data are present in the tile
            //
            // return FlatGpuVector::new(vector_types, triangles, index_buffer, vertices);
            todo!("index_buffer.is_some() && part_offsets.is_none() case is not implemented");
        }

        // Use decode_root_length_stream if geometry_offsets is present
        if let Some(offsets) = geometry_offsets.take() {
            geometry_offsets = Some(decode_root_length_stream(
                &vector_types,
                &offsets,
                GeometryType::Polygon,
            ));
        } else if let Some(offsets) = part_offsets.take() {
            if let Some(_ring_offsets) = ring_offsets {
                part_offsets = Some(decode_root_length_stream(
                    &vector_types,
                    &offsets,
                    GeometryType::LineString,
                ));
                // decodeLevel1LengthStream(geometryTypes,
                //                          partOffsets,
                //                          ringOffsetsCopy,
                //                          /*isLineStringPresent=*/true,
                //                          ringOffsets);
                todo!("part_offsets with ring_offsets case is not implemented");
            } else {
                part_offsets = Some(decode_root_length_stream(
                    &vector_types,
                    &offsets,
                    GeometryType::Point,
                ));
            }
        }

        Ok(DecodedGeometry {
            // vector_type,
            // vertex_buffer_type: VertexBufferType::Vec2, // Morton not supported yet
            vector_types,
            geometry_offsets,
            part_offsets,
            ring_offsets,
            vertex_offsets,
            index_buffer,
            triangles,
            vertices,
        })
    }
}

fn decode_geometry_types(meta: Stream) -> Result<Vec<GeometryType>, MltError> {
    // TODO: simplify this, e.g. use u8 or even GeometryType directly rather than going via Vec<u32>
    let vector_types: Vec<u32> = meta.decode_bits_u32()?.decode_u32()?;
    let vector_types: Vec<GeometryType> = vector_types
        .into_iter()
        .map::<Result<GeometryType, MltError>, _>(|v| Ok(u8::try_from(v)?.try_into()?))
        .collect::<Result<_, _>>()?;
    Ok(vector_types)
}
// pub fn get_vector_type_int_stream(metadata: &Stream) -> VectorType {
//     match metadata.stream {
//         StreamType::Rle => {
//             if metadata.data.len() == 1 {
//                 VectorType::Const
//             } else {
//                 VectorType::Flat
//             }
//         }
//         StreamType::DeltaRle if (1..=2).contains(&metadata.data.len()) => VectorType::Sequence,
//         _ => {
//             if metadata.num_values == 1 {
//                 VectorType::Const
//             } else {
//                 VectorType::Flat
//             }
//         }
//     }
// }

/// Handle the parsing of the different topology length buffers separate not generic to reduce the
/// branching and improve the performance
fn decode_root_length_stream(
    geometry_types: &[GeometryType],
    root_length_stream: &[u32],
    buffer_id: GeometryType,
) -> Vec<u32> {
    let mut root_buffer_offsets = Vec::with_capacity(geometry_types.len() + 1);
    root_buffer_offsets.push(0);
    let mut previous_offset = 0_u32;
    let mut root_length_counter = 0_usize;
    for &geom_type in geometry_types {
        let offset = previous_offset
            + if geom_type > buffer_id {
                let val = root_length_stream[root_length_counter];
                root_length_counter += 1;
                val
            } else {
                1
            };
        root_buffer_offsets.push(offset);
        previous_offset = offset;
    }
    root_buffer_offsets
}
