use bytes_varint::VarIntError as BvVarIntError;
use thiserror::Error;

use crate::metadata::stream_encoding::{LogicalLevelTechnique, PhysicalLevelTechnique};

pub type MltResult<T> = Result<T, MltError>;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum MltError {
    //----------------------------------------------------
    // External errors (foreign errors)
    //----------------------------------------------------
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    ProtobufDecode(#[from] prost::DecodeError),

    #[error(transparent)]
    RleDecode(#[from] serde_columnar::ColumnarError),

    //----------------------------------------------------
    // Wire/codec decoding (bytes → primitives)
    //----------------------------------------------------
    #[error("varint decode error: {0:?}")]
    Varint(#[from] VarintError),

    #[error("protobuf decode error at offset={offset} kind={kind:?}")]
    Protobuf { offset: usize, kind: ProtobufError },

    #[error("buffer underflow: needed {needed} bytes, but only {remaining} remain")]
    BufferUnderflow { needed: usize, remaining: usize },

    #[error("FastPFor decode failed: expected={expected} got={got}")]
    FastPforDecode { expected: usize, got: usize },

    #[error("FastPFor FFI error: {0}")]
    FastPforFfi(String),

    #[error("invalid RLE run length (cannot convert to usize): value={0}")]
    RleRunLenInvalid(i128),

    //----------------------------------------------------
    // Schema & metadata validation
    //----------------------------------------------------
    #[error("missing required field `{field}`")]
    MissingField { field: &'static str },

    #[error("invalid discriminant for {kind}: code={code}")]
    InvalidDiscriminant { kind: &'static str, code: u8 },

    #[error("metadata decode error: field={field} kind={kind:?}")]
    MetadataDecode {
        field: &'static str,
        kind: MetadataErrorKind,
    },

    #[error("missing required logical metadata: {which}")]
    MissingLogicalMetadata { which: &'static str },

    //----------------------------------------------------
    // Structural constraints (lengths, counts, shapes)
    //----------------------------------------------------
    #[error("{what} must be a multiple of {multiple_of}, got {got}")]
    InvalidMultiple {
        what: &'static str,
        multiple_of: usize,
        got: usize,
    },

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

    //----------------------------------------------------
    // Technique selection / control flow
    //----------------------------------------------------
    #[error("unsupported technique at {level:?}: {technique:?}")]
    UnsupportedTechnique {
        level: ErrorLevel,
        technique: TechniqueDiscriminant,
    },

    #[error("partial decode not supported for {0:?}")]
    PartialDecodeWrongTechnique(LogicalLevelTechnique),

    //----------------------------------------------------
    // Numeric/arithmetics and coordinate errors
    //----------------------------------------------------
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

    //----------------------------------------------------
    // Domain-specific lookups (IDs, header vectors)
    //----------------------------------------------------
    #[error("missing infos[{0}]")]
    MissingInfo(usize),

    #[error("feature table not found: id={0}")]
    FeatureTableNotFound(u32),
}

//----------------- Support types -----------------
#[derive(Debug)]
pub enum ErrorLevel {
    Physical,
    Logical,
}

#[derive(Debug)]
pub enum TechniqueDiscriminant {
    Physical(PhysicalLevelTechnique),
    Logical(LogicalLevelTechnique),
}

//----------------- Varint failures -----------------
/// Note: `NonCanonical` means the varint was not minimally encoded
/// (it used more bytes than necessary for the value).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum VarintError {
    #[error("unexpected end of input while reading varint")]
    UnexpectedEof,
    #[error("varint too long")]
    TooLong,
    #[error("varint overflowed target integer type")]
    Overflow,
    #[error("varint not in canonical form")]
    NonCanonical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtobufError {
    Truncated,
    Malformed,
    InvalidTag,
    UnexpectedWireType,
    Utf8,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataErrorKind {
    Missing,
    Malformed,
    TypeMismatch,
    OutOfRange,
    Other,
}

//------------------------ Helpers ------------------------
impl MltError {
    #[cold]
    #[inline(never)]
    pub fn protobuf(offset: usize, kind: ProtobufError) -> Self {
        Self::Protobuf { offset, kind }
    }

    #[cold]
    #[inline(never)]
    pub fn unsupported_physical(tech: PhysicalLevelTechnique) -> Self {
        Self::UnsupportedTechnique {
            level: ErrorLevel::Physical,
            technique: TechniqueDiscriminant::Physical(tech),
        }
    }

    #[cold]
    #[inline(never)]
    pub fn unsupported_logical(tech: LogicalLevelTechnique) -> Self {
        Self::UnsupportedTechnique {
            level: ErrorLevel::Logical,
            technique: TechniqueDiscriminant::Logical(tech),
        }
    }

    #[cold]
    #[inline(never)]
    pub fn invalid_byte_multiple(multiple_of: usize, got: usize) -> Self {
        Self::InvalidMultiple {
            what: "byte length",
            multiple_of,
            got,
        }
    }

    #[cold]
    #[inline(never)]
    pub fn invalid_value_multiple(multiple_of: usize, got: usize) -> Self {
        Self::InvalidMultiple {
            what: "value count",
            multiple_of,
            got,
        }
    }
}

//------------------------ Mappers for foreign errors ------------------------
impl From<BvVarIntError> for MltError {
    fn from(e: BvVarIntError) -> Self {
        match e {
            BvVarIntError::BufferUnderflow => MltError::Varint(VarintError::UnexpectedEof),
            BvVarIntError::NumericOverflow => MltError::Varint(VarintError::Overflow),
        }
    }
}

impl From<cxx::Exception> for MltError {
    fn from(e: cxx::Exception) -> Self {
        MltError::FastPforFfi(e.what().to_string())
    }
}
