use integer_encoding::VarInt;
use std::convert::TryFrom;
// use borrowme::borrowme;
use crate::MltError::Fail;
use crate::structures::complex_enums::{ColumnStreams, DataRaw, DataVarInt, LogicalStream2, LogicalStreamDecoder, PhysicalStreamType, Stream, StreamData, StreamMeta};
use crate::structures::enums::{
    ColumnType, DictionaryType, GeometryType, LengthType, LogicalTechnique, PhysicalTechnique,
};
use crate::utils::{SetOptionOnce, all, parse_u7, take};
use crate::{MltError, MltRefResult, utils};

#[derive(Debug, Clone, PartialEq)]
pub struct Geometry {
    // pub vector_type: VectorType,
    pub vector_types: Vec<GeometryType>,
    pub geometry_offsets: Option<Vec<u32>>,
    pub part_offsets: Option<Vec<u32>>,
    pub ring_offsets: Option<Vec<u32>>,
    pub vertex_offsets: Option<Vec<u32>>,
    pub index_buffer: Option<Vec<u32>>,
    pub triangles: Option<Vec<u32>>,
    pub vertices: Option<Vec<i32>>,
}

impl Geometry {
    fn parse(input: &[u8]) -> MltRefResult<'_, Self> {
        let (input, stream_count) = parse_u7(input)?;
        // let mut vec = Vec::with_capacity(stream_count as usize);
        let (input, meta) = Stream::parse(input)?;
        // let vector_type = Self::get_vector_type_int_stream(&meta);
        let vector_types = meta.decode::<u8, GeometryType>()?;
        // vec.push(meta);
        let mut geometry_offsets: Option<Vec<u32>> = None;
        let mut part_offsets: Option<Vec<u32>> = None;
        let mut ring_offsets: Option<Vec<u32>> = None;
        let mut vertex_offsets: Option<Vec<u32>> = None;
        let mut index_buffer: Option<Vec<u32>> = None;
        let mut triangles: Option<Vec<u32>> = None;
        let mut vertices: Option<Vec<i32>> = None;

        let mut input = input;
        for _ in 1..stream_count {
            let stream;
            (input, stream) = Stream::parse(input)?;

            match stream.meta.physical_type {
                PhysicalStreamType::Present => {}
                PhysicalStreamType::Data(v) => match v {
                    DictionaryType::Vertex => {
                        let v = stream.decode::<u32, u32>()?.u32()?;
                        vertices.set_once(LogicalStreamDecoder::decode(v)?)?;
                    }
                    _ => panic!("Geometry stream cannot have Data physical type: {v:?}"),
                },
                PhysicalStreamType::Offset(_) => {}
                PhysicalStreamType::Length(v) => match v {
                    LengthType::Geometries => {
                        let v = stream.decode::<u32, u32>()?.u32()?;
                        geometry_offsets.set_once(LogicalStreamDecoder::decode(v)?)?;
                    }
                    _ => panic!("Geometry stream cannot have Length physical type: {v:?}"),
                },
            }
            //         PhysicalStreamType::Data(data) => match data {
            //             DictionaryType::Vertex => {
            //                 if vertices.replace(stream.decode::<u32, i32>()?).is_some() {
            //                     return Err(Fail);
            //                 }
            //             }
            //             _ => panic!("Geometry stream cannot have Data physical type: {data:?}"),
            //         },
            //         _ => panic!("Geometry stream cannot have physical type: {stream:?}"),
            //     }
            //     vec.push(stream);
            // }
            // if let Some(offsets) = geometry_offsets.take() {
            //     geometry_offsets = Some(decode_root_length_stream(
            //         &vector_types,
            //         offsets,
            //         GeometryType::Polygon,
            //     ))
        }
        //
        // columns.push(ColumnStreams::Geometry(vec));
        Ok((
            input,
            Geometry {
                // vector_type,
                vector_types: Default::default(),
                geometry_offsets: Default::default(),
                part_offsets: Default::default(),
                ring_offsets: Default::default(),
                vertex_offsets: Default::default(),
                index_buffer: Default::default(),
                triangles: Default::default(),
                vertices: Default::default(),
            },
        ))
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
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VectorType {
    Flat,
    Const,
    Sequence,
    // Dictionary,
    // FsstDictionary,
}

impl Stream<'_> {
    fn parse(input: &[u8]) -> MltRefResult<'_, Stream<'_>> {
        use {LogicalTechnique as LT, PhysicalTechnique as PT};

        let (input, val) = utils::parse_u8(input)?;
        let physical_type = PhysicalStreamType::from_u8(val).ok_or(Fail)?;

        let (input, val) = utils::parse_u8(input)?;
        let logical_technique1 = LT::try_from(val >> 5).or(Err(Fail))?;
        let logical_technique2 = LT::try_from((val >> 2) & 0x7).or(Err(Fail))?;
        let physical_technique = PT::try_from(val & 0x3).or(Err(Fail))?;

        let (input, num_values) = utils::parse_varint::<usize>(input)?;
        let (input, byte_length) = utils::parse_varint::<usize>(input)?;
        let (input, data) = take(input, byte_length)?;

        let meta = StreamMeta {
            physical_type,
            logical_technique1,
            logical_technique2,
            physical_technique,
            num_values,
        };

        let stream = Stream::new(
            meta,
            match physical_technique {
                PT::None => DataRaw::new(data),
                PT::VarInt => DataVarInt::new(data),
                _ => {
                    panic!(
                        "Unsupported logical/physical technique combination: {physical_technique:?}",
                    )
                }
            },
        );

        Ok((input, stream))
    }

    pub fn decode<T, U>(self) -> Result<LogicalStream2<U>, MltError>
    where
        T: VarInt,
        U: TryFrom<T>,
        MltError: From<<U as TryFrom<T>>::Error>,
    {
        let value = match self.data {
            StreamData::VarInt(data) => all(utils::parse_varint_vec::<T, U>(
                data.data,
                self.meta.num_values,
            )?),
            // StreamData::Raw(data) => {
            //     // let physical_decode = all(parse_varint_vec::<T, U>(self.data, self.num_values)?)?;
            //     // decode_componentwise_delta_vec2s(physical_decode.as_slice())
            // }
            _ => panic!("Unsupported physical type: {:?}", self.data)
        }?;
        Ok(LogicalStream2::new(self.meta, value))
        // Ok(match self.meta.logical_technique1 {
        //     LogicalTechnique::None => {
        //         LogicalStream2::new(self.meta, value)
        //     }
        //     LogicalTechnique::ComponentwiseDelta => {
        //         LogicalStream2::new(self.meta, value)
        //     }
        //     _ => panic!(
        //         "Unsupported logical technique for decoding: {:?}",
        //         self.meta.logical_technique1
        //     ),
        // })
    }

    // pub fn decode<'a, T, U>(&'_ self) -> Result<Vec<U>, MltError>
    // where
    //     T: VarInt,
    //     U: TryFrom<T>, // + ZigZag,
    //     MltError: From<<U as TryFrom<T>>::Error>,
    // {
    //     match &self.stream {
    //         StreamType::VarInt(data) => all(parse_varint_vec::<T, U>(data, self.num_values)?),
    //         StreamType::ComponentwiseDeltaVarInt(data) => {
    //             // let physical_decode = all(parse_varint_vec::<T, U>(self.data, self.num_values)?)?;
    //             todo!();
    //             // decode_componentwise_delta_vec2s(physical_decode.as_slice())
    //         }
    //         _ => panic!("Unsupported physical type: {:?}", self.stream),
    //     }
    // }

    // pub fn decode2<'a>(&'_ self) -> MltResult<'_, Vec<u32>> {
    //     match self.physical_type {
    //         PhysicalStreamType::Present => {
    //             todo!()
    //         }
    //         PhysicalStreamType::Data(_v) => parse_varint_vec::<u32, u32>(&[], self.num_values),
    //         PhysicalStreamType::Offset(_v) => {
    //             todo!()
    //         }
    //         PhysicalStreamType::Length(_v) => {
    //             todo!()
    //         }
    //     }
    // }
}

/// MVT-compatible feature table data
//#[borrowme]
#[derive(Debug, PartialEq)]
pub struct FeatureTable<'a> {
    pub name: &'a str,
    pub extent: u32,
    pub columns: Vec<ColumnStreams<'a>>,
}

impl FeatureTable<'_> {
    /// Parse `FeatureTable` V1 metadata
    pub fn parse(mut input: &[u8]) -> Result<FeatureTable<'_>, MltError> {
        let name;
        let extent;
        let column_count;

        (input, name) = utils::parse_string(input)?;
        (input, extent) = utils::parse_varint::<u32>(input)?;
        (input, column_count) = utils::parse_varint::<usize>(input)?;

        let mut col_info = Vec::with_capacity(column_count);
        for _ in 0..column_count {
            let typ;
            (input, typ) = Column::parse(input)?;
            col_info.push(typ);
        }

        let mut columns = Vec::with_capacity(col_info.len() - 1);
        for info in col_info {
            let opt;
            let val;
            let nam = info.name.unwrap_or("");
            match info.typ {
                ColumnType::Id => {
                    (input, val) = Stream::parse(input)?;
                    columns.push(ColumnStreams::Id(val));
                }
                ColumnType::OptId => {
                    (input, (opt, val)) = parse_pair(input)?;
                    columns.push(ColumnStreams::OptId(opt, val));
                }
                ColumnType::LongId => {
                    (input, val) = Stream::parse(input)?;
                    columns.push(ColumnStreams::LongId(val));
                }
                ColumnType::OptLongId => {
                    (input, (opt, val)) = parse_pair(input)?;
                    columns.push(ColumnStreams::OptLongId(opt, val));
                }
                ColumnType::Geometry => {
                    // Geometry columns do not have a name
                    let val;
                    (input, val) = Geometry::parse(input)?;
                    columns.push(ColumnStreams::Geometry(val));
                }
                ColumnType::Bool => {
                    (input, val) = Stream::parse(input)?;
                    columns.push(ColumnStreams::Bool(nam, val));
                }
                ColumnType::OptBool => {
                    (input, (opt, val)) = parse_pair(input)?;
                    columns.push(ColumnStreams::OptBool(nam, opt, val));
                }
                ColumnType::I8 => {
                    (input, val) = Stream::parse(input)?;
                    columns.push(ColumnStreams::I8(nam, val));
                }
                ColumnType::OptI8 => {
                    (input, (opt, val)) = parse_pair(input)?;
                    columns.push(ColumnStreams::OptI8(nam, opt, val));
                }
                ColumnType::U8 => {
                    (input, val) = Stream::parse(input)?;
                    columns.push(ColumnStreams::U8(nam, val));
                }
                ColumnType::OptU8 => {
                    (input, (opt, val)) = parse_pair(input)?;
                    columns.push(ColumnStreams::OptU8(nam, opt, val));
                }
                ColumnType::I32 => {
                    (input, val) = Stream::parse(input)?;
                    columns.push(ColumnStreams::I32(nam, val));
                }
                ColumnType::OptI32 => {
                    (input, (opt, val)) = parse_pair(input)?;
                    columns.push(ColumnStreams::OptI32(nam, opt, val));
                }
                ColumnType::U32 => {
                    (input, val) = Stream::parse(input)?;
                    columns.push(ColumnStreams::U32(nam, val));
                }
                ColumnType::OptU32 => {
                    (input, (opt, val)) = parse_pair(input)?;
                    columns.push(ColumnStreams::OptU32(nam, opt, val));
                }
                ColumnType::I64 => {
                    (input, val) = Stream::parse(input)?;
                    columns.push(ColumnStreams::I64(nam, val));
                }
                ColumnType::OptI64 => {
                    (input, (opt, val)) = parse_pair(input)?;
                    columns.push(ColumnStreams::OptI64(nam, opt, val));
                }
                ColumnType::U64 => {
                    (input, val) = Stream::parse(input)?;
                    columns.push(ColumnStreams::U64(nam, val));
                }
                ColumnType::OptU64 => {
                    (input, (opt, val)) = parse_pair(input)?;
                    columns.push(ColumnStreams::OptU64(nam, opt, val));
                }
                ColumnType::F32 => {
                    (input, val) = Stream::parse(input)?;
                    columns.push(ColumnStreams::F32(nam, val));
                }
                ColumnType::OptF32 => {
                    (input, (opt, val)) = parse_pair(input)?;
                    columns.push(ColumnStreams::OptF32(nam, opt, val));
                }
                ColumnType::F64 => {
                    (input, val) = Stream::parse(input)?;
                    columns.push(ColumnStreams::F64(nam, val));
                }
                ColumnType::OptF64 => {
                    (input, (opt, val)) = parse_pair(input)?;
                    columns.push(ColumnStreams::OptF64(nam, opt, val));
                }
                ColumnType::Str => {
                    (input, val) = Stream::parse(input)?;
                    columns.push(ColumnStreams::Str(nam, val));
                }
                ColumnType::OptStr => {
                    (input, (opt, val)) = parse_pair(input)?;
                    columns.push(ColumnStreams::OptStr(nam, opt, val));
                }
                ColumnType::Struct => {}
            }
        }
        if input.is_empty() {
            Ok(FeatureTable {
                name,
                extent,
                columns,
            })
        } else {
            Err(Fail)
        }
    }
}

pub fn parse_pair(input: &[u8]) -> MltRefResult<'_, (Stream<'_>, Stream<'_>)> {
    let (input, opt) = Stream::parse(input)?;
    let (input, val) = Stream::parse(input)?;
    Ok((input, (opt, val)))
}

/// Column definition
//#[borrowme]
#[derive(Debug, PartialEq)]
pub struct Column<'a> {
    pub typ: ColumnType,
    pub name: Option<&'a str>,
}

impl Column<'_> {
    /// Parse a single column definition
    fn parse(input: &[u8]) -> MltRefResult<'_, Column<'_>> {
        let (mut input, typ) = ColumnType::parse(input)?;
        let name = if typ.has_name() {
            let pair = utils::parse_string(input)?;
            input = pair.0;
            Some(pair.1)
        } else {
            None
        };
        Ok((input, Column { typ, name }))
    }
}

///Handle the parsing of the different topology length buffers separate not generic to reduce the
///branching and improve the performance
pub fn decode_root_length_stream(
    geometry_types: &[GeometryType],
    root_length_stream: Vec<u32>,
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
