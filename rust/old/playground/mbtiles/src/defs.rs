#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum GeomType {
    Unknown = 0,
    Point = 1,
    Linestring = 2,
    Polygon = 3,
}
impl From<&str> for GeomType {
    fn from(value: &str) -> Self {
        match value {
            "Point" => GeomType::Point,
            "Linestring" => GeomType::Linestring,
            "Polygon" => GeomType::Polygon,
            _ => GeomType::Unknown,
        }
    }
}


#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum StreamType_Dictionary {
    NONE = 0,
    SINGLE = 1,
    SHARED = 2,
    VERTEX = 4,
    MORTON = 8,
    FSST = 16,
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum StreamType_OffsetIntoDictionary {
    VERTEX = 0,
    INDEX = 1,
    STRING = 2,
    KEY = 4,
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum StreamType_VariableSizedItems {
    VAR_BINARY = 0,
    GEOMETRIES = 1,
    PARTS = 2,
    RINGS = 4,
    TRIANGLE = 8,
    SYMBOL = 16,
    DICTIONARY = 32,
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum DataType_Vector {
    FLAT = 0,
    CONST = 1,
    FREQUENCY = 2,
    REE = 4,
    DICTIONARY = 8,
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum LogicalLevelCompressionTechnique {
    NONE = 0,
    DELTA = 1,
    COMPONENTWISE_DELTA = 2,
    RLE = 4,
    MORTON = 8,
    PDE = 16,
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum PhysicalLevelCompressionTechnique {
    NONE = 0,
    FAST_PFOR = 1,
    VARINT = 2,
    ALP = 4,
}

#[derive(Debug, Copy, Clone)]
pub struct PhysicalStreamType {
    present: bool,
    data: StreamType_Dictionary,
    offset: StreamType_OffsetIntoDictionary,
    length: StreamType_VariableSizedItems,
}

#[derive(Debug, Copy, Clone)]
pub struct LogicalStreamType {
    dictionary_type: Option<StreamType_Dictionary>,
    offset_type: Option<StreamType_OffsetIntoDictionary>,
    length_type: Option<StreamType_VariableSizedItems>,
}

#[derive(Debug, Copy, Clone)]
pub struct StreamMetaData {
    physical_stream_type: PhysicalStreamType,
    logical_stream_type: LogicalStreamType,
    logical_level_technique1: LogicalLevelCompressionTechnique,
    logical_level_technique2: LogicalLevelCompressionTechnique,
    physical_level_technique: PhysicalLevelCompressionTechnique,
    num_values: u32,
    byte_length: u32,
}

#[derive(Debug)]
pub struct FieldMetadata {
    num_streams: u32,
    vector_type: DataType_Vector,
    stream_meta: Vec<StreamMetaData>,
}

#[derive(Debug)]
pub struct FeatureTableMetadata {
    version: u8,
    id: u32,
    layer_extent: u32,
    max_layer_extent: u32,
    num_features: u32,
    field_metadata: Vec<FieldMetadata>,
}
