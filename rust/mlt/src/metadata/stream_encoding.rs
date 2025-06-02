use num_enum::TryFromPrimitive;

#[derive(Debug, Clone, TryFromPrimitive)]
#[repr(u8)]
pub enum LengthType {
    VarBinary,
    Geometries,
    Parts,
    Rings,
    Triangles,
    Symbol,
    Dictionary,
}

#[derive(Debug, Clone, TryFromPrimitive)]
#[repr(u8)]
pub enum DictionaryType {
    Single,
    Shared,
    Vertex,
    Morton,
    Fsst,
}

#[derive(Debug, Clone, TryFromPrimitive)]
#[repr(u8)]
pub enum OffsetType {
    Vertex,
    Index,
    String,
    Key,
}

#[derive(Debug, Clone)]
pub enum LogicalStreamType {
    Dictionary(Option<DictionaryType>),
    Offset(OffsetType),
    Length(LengthType),
}

#[derive(Debug, Clone, TryFromPrimitive, PartialEq)]
#[repr(u8)]
pub enum LogicalLevelTechnique {
    None,
    Delta,
    ComponentwiseDelta,
    Rle,
    Morton,
    // Pseudodecimal Encoding of floats -> only for the exponent integer part an additional logical level technique is used.
    // Both exponent and significant parts are encoded with the same physical level technique
    Pde,
}

#[derive(Debug, Clone)]
pub struct Logical {
    pub r#type: Option<LogicalStreamType>,
    pub technique1: Option<LogicalLevelTechnique>,
    pub technique2: Option<LogicalLevelTechnique>,
}

impl Logical {
    pub fn new(
        r#type: Option<LogicalStreamType>,
        technique1: LogicalLevelTechnique,
        technique2: LogicalLevelTechnique,
    ) -> Self {
        Self {
            r#type,
            technique1: Some(technique1),
            technique2: Some(technique2),
        }
    }
}

#[derive(Debug, Clone, TryFromPrimitive, PartialEq)]
#[repr(u8)]
pub enum PhysicalStreamType {
    Present,
    Data,
    Offset,
    Length,
}

#[derive(Debug, Clone, TryFromPrimitive, PartialEq)]
#[repr(u8)]
pub enum PhysicalLevelTechnique {
    None,
    FastPfor,
    Varint,
    Alp,
}

#[derive(Debug, Clone)]
pub struct Physical {
    pub r#type: PhysicalStreamType,
    pub technique: PhysicalLevelTechnique,
}

impl Physical {
    pub fn new(r#type: PhysicalStreamType, technique: PhysicalLevelTechnique) -> Self {
        Self { r#type, technique }
    }
}
