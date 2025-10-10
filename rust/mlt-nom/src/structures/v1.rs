use borrowme::borrowme;
use integer_encoding::VarInt;

use crate::MltError::Fail;
use crate::structures::complex_enums::{ColumnStreams, PhysicalStreamType, StreamType};
use crate::structures::enums::{
    ColumnType, DictionaryType, GeometryType, LengthType, LogicalTechnique, PhysicalTechnique,
};
use crate::utils::{all, parse_u7, parse_varint_vec, take};
use crate::{MltError, MltResult, utils};

#[derive(Debug, Clone, PartialEq)]
pub struct Geometry {
    pub vector_type: VectorType,
    pub vector_types: Vec<GeometryType>,
}

impl Geometry {
    fn parse(input: &[u8]) -> MltResult<'_, Self> {
        let (input, stream_count) = parse_u7(input)?;
        let mut vec = Vec::with_capacity(stream_count as usize);
        let (input, meta) = Stream::parse(input)?;
        let vector_type = Self::get_vector_type_int_stream(&meta);
        let vector_types = meta.decode::<u8, GeometryType>()?;
        vec.push(meta);
        let mut geometryOffsets: Option<Vec<u32>> = None;
        let mut partOffsets: Option<Vec<u32>> = None;
        let mut ringOffsets: Option<Vec<u32>> = None;
        let mut vertexOffsets: Option<Vec<u32>> = None;
        let mut indexBuffer: Option<Vec<u32>> = None;
        let mut triangles: Option<Vec<u32>> = None;
        let mut vertices: Option<Vec<i32>> = None;

        let mut input = input;
        for _ in 1..stream_count {
            let stream;
            (input, stream) = Stream::parse(input)?;
            match stream.physical_type {
                PhysicalStreamType::Data(data) => match data {
                    DictionaryType::Vertex => match stream.logical_type {
                        StreamType::VarInt => {
                            if vertices.is_some() {
                                return Err(Fail);
                            }
                            vertices = Some(stream.decode::<u32, i32>()?);
                        }
                        _ => panic!(
                            "Geometry stream cannot have Data logical type: {:?}",
                            stream.logical_type
                        ),
                    },
                    _ => panic!("Geometry stream cannot have Data physical type: {data:?}"),
                },
                PhysicalStreamType::Length(len) => match len {
                    LengthType::Geometries => {
                        if geometryOffsets.is_some() {
                            return Err(Fail);
                        }
                        geometryOffsets = Some(stream.decode::<u32, u32>()?);
                    }
                    _ => panic!("Geometry stream cannot have Length physical type: {len:?}"),
                },
                _ => panic!("Geometry stream cannot have physical type: {stream:?}"),
            }
            vec.push(stream);
        }
        // columns.push(ColumnStreams::Geometry(vec));
        Ok((
            input,
            Geometry {
                vector_type,
                vector_types,
            },
        ))
    }

    pub fn get_vector_type_int_stream(metadata: &Stream) -> VectorType {
        match metadata.logical_type {
            StreamType::Rle => {
                if metadata.data.len() == 1 {
                    VectorType::Const
                } else {
                    VectorType::Flat
                }
            }
            StreamType::DeltaRle if (1..=2).contains(&metadata.data.len()) => VectorType::Sequence,
            _ => {
                if metadata.num_values == 1 {
                    VectorType::Const
                } else {
                    VectorType::Flat
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VectorType {
    Flat,
    Const,
    Sequence,
    // Dictionary,
    // FsstDictionary,
}

/// MVT-compatible feature table data
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct Stream<'a> {
    pub physical_type: PhysicalStreamType,
    pub logical_type: StreamType,
    pub num_values: usize,
    #[borrowme(borrow_with = Vec::as_slice)]
    pub data: &'a [u8],
}

impl Stream<'_> {
    fn parse(input: &[u8]) -> MltResult<'_, Stream<'_>> {
        use {LogicalTechnique as LT, PhysicalTechnique as PT};

        let (input, val) = utils::parse_u8(input)?;
        let physical_type = PhysicalStreamType::from_u8(val).ok_or(Fail)?;

        let (input, val) = utils::parse_u8(input)?;
        let logical_technique1 = LT::try_from(val >> 5).or(Err(Fail))?;
        let logical_technique2 = LT::try_from((val >> 2) & 0x7).or(Err(Fail))?;
        let physical_technique = PT::try_from(val & 0x3).or(Err(Fail))?;
        let logical_type = match (logical_technique1, logical_technique2, physical_technique) {
            (LT::ComponentwiseDelta, LT::None, PT::VarInt) => StreamType::ComponentwiseDeltaVarInt,
            (LT::Delta, LT::ComponentwiseDelta, PT::VarInt) => StreamType::DeltaCompDeltaAlp,
            (LT::Delta, LT::Morton, PT::VarInt) => StreamType::DetaMortonVarInt,
            (LT::Delta, LT::None, PT::FastPFOR) => StreamType::DeltaFastPFOR,
            (LT::Delta, LT::None, PT::None) => StreamType::DeltaVarInt,
            (LT::Delta, LT::None, PT::VarInt) => StreamType::DeltaNoneVarInt,
            (LT::Delta, LT::PseudoDecimal, PT::Alp) => StreamType::DeltaPseudoDecimalAlp,
            (LT::Delta, LT::PseudoDecimal, PT::None) => StreamType::DeltaPseudoDecimal,
            (LT::Delta, LT::PseudoDecimal, PT::VarInt) => StreamType::DeltaPseudoDecimalVarInt,
            (LT::None, LT::ComponentwiseDelta, PT::Alp) => StreamType::NoneCompDeltaAlp,
            (LT::None, LT::Delta, PT::Alp) => StreamType::NoneDeltaAlp,
            (LT::None, LT::Rle, PT::VarInt) => StreamType::NoneRleVarInt,
            (LT::Delta, LT::Morton, PT::None) => StreamType::DeltaMorton,
            (LT::Morton, LT::Rle, PT::FastPFOR) => StreamType::MortonRleFastPFOR,
            (LT::None, LT::ComponentwiseDelta, PT::None) => StreamType::NoneCompDeltaNone,
            (LT::None, LT::Delta, PT::FastPFOR) => StreamType::NoneDeltaFastPFOR,
            (LT::None, LT::Delta, PT::None) => StreamType::NoneDelta,
            (LT::None, LT::Delta, PT::VarInt) => StreamType::NoneDeltaVarInt,
            (LT::None, LT::Morton, PT::Alp) => StreamType::NoneMortonAlp,
            (LT::None, LT::Morton, PT::FastPFOR) => StreamType::NoneMortonFastPFOR,
            (LT::None, LT::Morton, PT::None) => StreamType::NoneMorton,
            (LT::None, LT::None, PT::Alp) => StreamType::Alp,
            (LT::None, LT::None, PT::FastPFOR) => StreamType::NoneFastPFOR,
            (LT::None, LT::None, PT::None) => StreamType::None,
            (LT::None, LT::None, PT::VarInt) => StreamType::VarInt,
            (LT::None, LT::PseudoDecimal, PT::Alp) => StreamType::NonePseudoDecimalAlp,
            (LT::None, LT::PseudoDecimal, PT::None) => StreamType::NonePseudoDecimal,
            (LT::None, LT::Rle, PT::FastPFOR) => StreamType::NoneRleFastPFOR,
            (LT::None, LT::Rle, PT::None) => StreamType::NoneRle,
            (LT::PseudoDecimal, LT::None, PT::None) => StreamType::PseudoDecimal,
            (LT::Rle, LT::None, PT::None) => StreamType::Rle,
            (LT::Rle, LT::None, PT::VarInt) => StreamType::RleVarInt,
            _ => panic!(
                "Unsupported logical/physical technique combination: {:?}, {:?}, {:?}",
                logical_technique1, logical_technique2, physical_technique
            ), // return Err(Fail),
        };

        let (input, num_values) = utils::parse_varint::<usize>(input)?;
        let (input, byte_length) = utils::parse_varint::<usize>(input)?;
        let (input, data) = take(input, byte_length)?;

        Ok((
            input,
            Stream {
                physical_type,
                logical_type,
                num_values,
                data,
            },
        ))
    }

    pub fn decode<'a, T, U>(&'_ self) -> Result<Vec<U>, MltError>
    where
        T: VarInt,
        U: TryFrom<T>,
        MltError: From<<U as TryFrom<T>>::Error>,
    {
        match self.logical_type {
            StreamType::VarInt => {
                // LogicalLevelTechnique::NONE
                all(parse_varint_vec::<T, U>(self.data, self.num_values)?)
            }
            _ => panic!("Unsupported physical type: {:?}", self.logical_type),
        }
    }

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
#[borrowme]
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

pub fn parse_pair(input: &[u8]) -> MltResult<'_, (Stream<'_>, Stream<'_>)> {
    let (input, opt) = Stream::parse(input)?;
    let (input, val) = Stream::parse(input)?;
    Ok((input, (opt, val)))
}

/// Column definition
#[borrowme]
#[derive(Debug, PartialEq)]
pub struct Column<'a> {
    pub typ: ColumnType,
    pub name: Option<&'a str>,
}

impl Column<'_> {
    /// Parse a single column definition
    fn parse(input: &[u8]) -> MltResult<'_, Column<'_>> {
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
