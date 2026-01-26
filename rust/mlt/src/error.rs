use bytes_varint::VarIntError as BvVarIntError;
use fastpfor::cpp::Exception;

use crate::metadata::stream_encoding::{LogicalLevelTechnique, PhysicalLevelTechnique};

pub type MltResult<T> = Result<T, MltError>;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum MltError {
    // External errors (foreign errors)
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    ProtobufDecode(#[from] prost::DecodeError),
    #[error(transparent)]
    RleDecode(#[from] serde_columnar::ColumnarError),

    // Wire/codec decoding (bytes → primitives)
    #[error("varint decode error: {0:?}")]
    Varint(#[from] VarintError),
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

    // Technique selection / control flow
    #[error("unsupported physical technique: {0:?}")]
    UnsupportedPhysicalTechnique(PhysicalLevelTechnique),
    #[error("unsupported logical technique: {0:?}")]
    UnsupportedLogicalTechnique(LogicalLevelTechnique),
    #[error("partial decode not supported for {0:?}")]
    PartialDecodeWrongTechnique(LogicalLevelTechnique),

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
    #[error("feature table not found: id={0}")]
    FeatureTableNotFound(u32),

    // Other errors
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),
}

/// Varint failures
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum VarintError {
    #[error("unexpected end of input while reading varint")]
    UnexpectedEof,
    #[error("varint overflowed target integer type")]
    Overflow,
}

/// Mappers for foreign errors
impl From<BvVarIntError> for MltError {
    fn from(e: BvVarIntError) -> Self {
        match e {
            BvVarIntError::BufferUnderflow => MltError::Varint(VarintError::UnexpectedEof),
            BvVarIntError::NumericOverflow => MltError::Varint(VarintError::Overflow),
        }
    }
}

impl From<Exception> for MltError {
    fn from(e: Exception) -> Self {
        MltError::FastPforFfi(e.what().to_string())
    }
}
