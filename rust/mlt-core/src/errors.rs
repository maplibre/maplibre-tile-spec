use std::convert::Infallible;
use std::num::TryFromIntError;

use num_enum::TryFromPrimitiveError;

use crate::decoder::{
    GeometryType, LogicalEncoding, LogicalTechnique, PhysicalEncoding, StreamType,
};
use crate::utils::AsUsize;

pub type MltResult<T> = Result<T, MltError>;
pub type MltRefResult<'a, T> = Result<(&'a [u8], T), MltError>;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum MltError {
    #[error("cannot decode {0} as {1}")]
    DataWidthMismatch(&'static str, &'static str),
    #[error("dictionary index {0} out of bounds (len={1})")]
    DictIndexOutOfBounds(u32, usize),
    #[error("duplicate value found where unique required")]
    DuplicateValue,
    #[error("Integer overflow")]
    IntegerOverflow,
    #[error("missing geometry column in feature table")]
    MissingGeometry,
    #[error("missing string stream: {0}")]
    MissingStringStream(&'static str),
    #[error("multiple geometry columns found (only one allowed)")]
    MultipleGeometryColumns,
    #[error("multiple ID columns found (only one allowed)")]
    MultipleIdColumns,
    #[error("varint uses more bytes than necessary (non-canonical encoding)")]
    NonCanonicalVarInt,
    #[error("{0} is not decoded")]
    NotDecoded(&'static str),
    #[error("decoded data is not in encoded form")]
    NotEncoded,
    #[error("error parsing column type: code={0}")]
    ParsingColumnType(u8),
    #[error("error parsing logical technique: code={0}")]
    ParsingLogicalTechnique(u8),
    #[error("error parsing physical encoding: code={0}")]
    ParsingPhysicalEncoding(u8),
    #[error("error parsing stream type: code={0}")]
    ParsingStreamType(u8),
    #[error("found {0} bytes after the expected end of layer")]
    TrailingLayerData(usize),
    #[error("unexpected end of input (unable to take {0} bytes)")]
    UnableToTake(u32),
    #[error("unexpected stream type {0:?}")]
    UnexpectedStreamType(StreamType),
    #[error("unexpected stream type {0:?}, expected {1} for {2}")]
    UnexpectedStreamType2(StreamType, &'static str, &'static str),
    #[error("unsupported logical encoding {0:?} for {1}")]
    UnsupportedLogicalEncoding(LogicalEncoding, &'static str),
    #[error("invalid combination of logical encodings: {0:?} + {1:?}")]
    InvalidLogicalEncodings(LogicalTechnique, LogicalTechnique),
    #[error("layer has zero size")]
    ZeroLayerSize,
    #[error("The encoder used to optimise data is incompatible")]
    BadEncoderDataCombination,
    #[error("StagedLayer01::encode_explicit requires Encoder.explicit to be Some(_)")]
    MissingExplicitEncoder,

    // Wire/codec decoding (bytes → primitives)
    #[error("buffer underflow: needed {0} bytes, but only {1} remain")]
    BufferUnderflow(u32, usize),
    #[error("FastPFor decode failed: expected={0} got={1}")]
    FastPforDecode(u32, usize),
    #[error("invalid RLE run length (cannot convert to usize): value={0}")]
    RleRunLenInvalid(i128),

    // Structural constraints (lengths, counts, shapes)
    #[error("geometry requires at least 1 stream, got 0")]
    GeometryWithoutStreams,
    #[error("FastPFor data byte length expected multiple of 4, got {0}")]
    InvalidFastPforByteLength(usize),
    #[error("vec2 delta stream size expected to be non-empty and multiple of 2, got {0}")]
    InvalidPairStreamSize(usize),
    #[error("decodable stream size expected {1}, got {0}")]
    InvalidDecodingStreamSize(usize, usize),
    #[error("stream data mismatch: expected {0}, got {1}")]
    StreamDataMismatch(&'static str, &'static str),
    #[error("IDs missing for encoding (expected Some IDs, got None)")]
    IdsMissingForEncoding,
    #[error("missing struct encoder for struct")]
    MissingStructEncoderForStruct,
    #[error("previous decode/parsing attempt failed")]
    PriorParseFailure,
    #[error("presence stream has {0} bits set but {1} values provided")]
    PresenceValueCountMismatch(usize, usize),
    #[error("MVT parse error: {0}")]
    MvtParse(String),
    #[error("need to encode before being able to write")]
    NeedsEncodingBeforeWriting,
    #[error("memory limit exceeded: limit={limit}, used={used}, requested={requested}")]
    MemoryLimitExceeded {
        limit: u32,
        used: u32,
        requested: u32,
    },
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),
    #[error("unsupported property value and encoder combination: {0:?} + {1:?}")]
    UnsupportedPropertyEncoderCombination(&'static str, &'static str),
    #[error("shared dictionary requires at least 2 streams, got {0}")]
    SharedDictRequiresStreams(usize),
    #[error("unsupported string stream count (expected between 2 and 5): {0}")]
    UnsupportedStringStreamCount(usize),
    #[error("Structs are not allowed to be optional")]
    TriedToEncodeOptionalStruct,
    #[error(
        "encoding instruction count mismatch: expected {input_len} instructions for {input_len} properties, got {config_len}"
    )]
    EncodingInstructionCountMismatch { input_len: usize, config_len: usize },
    #[error("struct child data streams expected exactly 1 value, got {0}")]
    UnexpectedStructChildCount(u32),
    // Note that {expected}+1 is allowed for the legacy Java encoder bug
    #[error("SharedDict stream count is {actual}, expected {expected}")]
    InvalidSharedDictStreamCount { actual: u32, expected: u32 },
    #[error("unsupported physical encoding: {0}")]
    UnsupportedPhysicalEncoding(&'static str),
    #[error("unsupported physical encoding: {0:?} for {1}")]
    UnsupportedPhysicalEncodingForType(PhysicalEncoding, &'static str),
    #[error(
        "Extent {extent} cannot be encoded to morton due to morton allowing max. 16 bits, but {required_bits} would be required"
    )]
    VertexMortonNotCompatibleWithExtent { extent: u32, required_bits: u32 },

    // Geometry decode errors (field = variable name, geom_type for context)
    #[error("MVT error: {0}")]
    BadMvtGeometry(&'static str),
    #[error("geometry[{0}]: index out of bounds")]
    GeometryIndexOutOfBounds(usize),
    #[error("geometry[{index}]: {field}[{idx}] out of bounds (len={len})")]
    GeometryOutOfBounds {
        index: usize,
        field: &'static str,
        idx: usize,
        len: usize,
    },
    #[error("geometry[{index}]: vertex {vertex} out of bounds (count={count})")]
    GeometryVertexOutOfBounds {
        index: usize,
        vertex: usize,
        count: usize,
    },
    #[error("geometry[{0}]: {1} requires geometry_offsets")]
    NoGeometryOffsets(usize, GeometryType),
    #[error("geometry[{0}]: {1} requires part_offsets")]
    NoPartOffsets(usize, GeometryType),
    #[error("geometry[{0}]: {1} requires ring_offsets")]
    NoRingOffsets(usize, GeometryType),
    #[error("geometry[{0}]: unexpected offset combination for {1}")]
    UnexpectedOffsetCombination(usize, GeometryType),

    #[error("FastPFor error: {0}")]
    FastPfor(#[from] fastpfor::FastPForError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Serde JSON error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),
    #[error("integer conversion error: {0}")]
    TryFromIntError(#[from] TryFromIntError),
    #[error("num_enum conversion error: {0}")]
    TryFromPrimitive(#[from] TryFromPrimitiveError<GeometryType>),
    #[error("UTF-8 decode error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("UTF-8 decode error: {0}")]
    FromUtf8(#[from] std::string::FromUtf8Error),
}

impl From<Infallible> for MltError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

impl From<MltError> for std::io::Error {
    fn from(value: MltError) -> Self {
        match value {
            MltError::Io(e) => e,
            other => Self::other(other),
        }
    }
}

pub trait AsMltError<T> {
    fn or_overflow(&self) -> MltResult<T>;
}

impl<T: Copy> AsMltError<T> for Option<T> {
    #[inline]
    fn or_overflow(&self) -> MltResult<T> {
        self.ok_or(MltError::IntegerOverflow)
    }
}

impl AsMltError<u32> for Result<u32, TryFromIntError> {
    #[inline]
    fn or_overflow(&self) -> MltResult<u32> {
        self.map_err(|_| MltError::IntegerOverflow)
    }
}

#[inline]
pub fn fail_if_invalid_stream_size<T: AsUsize>(actual: T, expected: T) -> MltResult<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(MltError::InvalidDecodingStreamSize(
            actual.as_usize(),
            expected.as_usize(),
        ))
    }
}
