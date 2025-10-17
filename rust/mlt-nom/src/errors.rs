use std::convert::Infallible;

use num_enum::TryFromPrimitiveError;

use crate::v01::{GeometryType, LogicalTechnique};

pub type MltRefResult<'a, T> = Result<(&'a [u8], T), MltError>;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum MltError {
    #[error("Integer overflow")]
    IntegerOverflow,
    #[error("multiple ID columns found (only one allowed)")]
    MultipleIdColumns,
    #[error("multiple geometry columns found (only one allowed)")]
    MultipleGeometryColumns,
    #[error("missing geometry column in feature table")]
    MissingGeometry,
    #[error("found {0} bytes after the expected end of layer")]
    TrailingLayerData(usize),
    #[error("layer has zero size")]
    ZeroLayerSize,
    #[error("error 7-bit integer (must be < 128): value={0}")]
    Parsing7BitInt(u8),

    #[error("UTF-8 decode error: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    #[error("num_enum conversion error: {0}")]
    TryFromPrimitive(#[from] TryFromPrimitiveError<GeometryType>),

    #[error("integer conversion error: {0}")]
    TryFromIntError(#[from] std::num::TryFromIntError),

    #[error("duplicate value found where unique required")]
    DuplicateValue,

    #[error("unsupported combination of logical techniques: {0:?} + {1:?}")]
    UnsupportedLogicalTechnique(LogicalTechnique, LogicalTechnique),

    #[error("error parsing column type: code={0}")]
    ParsingColumnType(u8),

    #[error("error parsing physical stream type: code={0}")]
    ParsingPhysicalStreamType(u8),

    #[error("error parsing logical technique: code={0}")]
    ParsingLogicalTechnique(u8),

    #[error("error parsing physical decoder: code={0}")]
    ParsingPhysicalDecoder(u8),

    #[error("error parsing varint")]
    ParsingVarInt,

    #[error("unexpected end of input (unable to take {0} bytes)")]
    UnableToTake(usize),

    // #[error("num_enum conversion error: {0}")]
    // TryFromPrimitive2(#[from] TryFromPrimitiveError<u32>),
    //////////////////////////////////////////////

    // External errors (foreign errors)
    #[error(transparent)]
    Io(#[from] std::io::Error),
    // #[error(transparent)]
    // ProtobufDecode(#[from] prost::DecodeError),
    // #[error(transparent)]
    // RleDecode(#[from] serde_columnar::ColumnarError),

    // Wire/codec decoding (bytes → primitives)
    // #[error("varint decode error: {0:?}")]
    // Varint(#[from] VarintError),
    #[error("buffer underflow: needed {needed} bytes, but only {remaining} remain")]
    BufferUnderflow { needed: usize, remaining: usize },
    #[error("FastPFor decode failed: expected={expected} got={got}")]
    FastPforDecode { expected: usize, got: usize },
    #[error("FastPFor FFI error: {0}")]
    FastPforFfi(String),
    #[error("invalid RLE run length (cannot convert to usize): value={0}")]
    RleRunLenInvalid(i128),

    // Schema & metadata validation
    #[error("missing required field `{0}`")]
    MissingField(&'static str),
    #[error("Invalid PhysicalStreamType: code={0}")]
    InvalidPhysicalStreamType(u8),
    #[error("invalid DictionaryType: code={0}")]
    InvalidDictionaryType(u8),
    #[error("invalid OffsetType: code={0}")]
    InvalidOffsetType(u8),
    #[error("invalid LengthType: code={0}")]
    InvalidLengthType(u8),
    #[error("invalid LogicalLevelTechnique: code={0}")]
    InvalidLogicalLevelTechnique(u8),
    #[error("invalid PhysicalLevelTechnique: code={0}")]
    InvalidPhysicalLevelTechnique(u8),
    #[error("metadata decode error: invalid type={0}")]
    MetaDecodeInvalidType(&'static str),
    #[error("metadata decode error: unsupported type={0}")]
    MetaDecodeUnsupportedType(&'static str),
    #[error("missing required logical metadata: {which}")]
    MissingLogicalMetadata { which: &'static str },

    // Structural constraints (lengths, counts, shapes)
    #[error("{ctx} byte length expected multiple of {multiple_of}, got {got}")]
    InvalidByteMultiple {
        ctx: &'static str,
        multiple_of: usize,
        got: usize,
    },
    #[error("vec2 delta stream size expected to be non-empty and multiple of 2, got {0}")]
    InvalidPairStreamSize(usize),
    #[error("{ctx} expected exactly {expected} values, got {got}")]
    ExpectedValues {
        ctx: &'static str,
        expected: usize,
        got: usize,
    },
    #[error("{ctx} requires at least {min} elements, got {got}")]
    MinLength {
        ctx: &'static str,
        min: usize,
        got: usize,
    },

    // // Technique selection / control flow
    // #[error("unsupported physical technique: {0:?}")]
    // UnsupportedPhysicalTechnique(PhysicalLevelTechnique),
    // #[error("unsupported logical technique: {0:?}")]
    // UnsupportedLogicalTechnique(LogicalLevelTechnique),
    // #[error("partial decode not supported for {0:?}")]
    // PartialDecodeWrongTechnique(LogicalLevelTechnique),

    // Numeric/arithmetics and coordinate errors
    #[error("coordinate {coordinate} too large for i32 (shift={shift})")]
    CoordinateOverflow { coordinate: u32, shift: u32 },
    #[error("subtract overflow: {left_val} - {right_val}")]
    SubtractOverflow { left_val: i32, right_val: i32 },
    #[error("coordinate shift too large for i32: shift={0}")]
    ShiftTooLarge(u32),
    #[error("conversion overflow: {from} -> {to}, value={value}")]
    ConversionOverflow {
        from: &'static str,
        to: &'static str,
        value: u64,
    },

    // Domain-specific lookups (IDs, header vectors)
    #[error("missing infos[{0}]")]
    MissingInfo(usize),

    // Other errors
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),
}

impl From<Infallible> for MltError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}
