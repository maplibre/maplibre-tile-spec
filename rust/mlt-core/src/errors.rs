use std::convert::Infallible;

use fastpfor::cpp::Exception;
use num_enum::TryFromPrimitiveError;

use crate::v01::{GeometryType, LogicalEncoding, LogicalTechnique, PhysicalEncoding, StreamType};

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
    #[error("error 7-bit integer (must be < 128): value={0}")]
    Parsing7BitInt(u8),
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
    UnableToTake(usize),
    #[error("unexpected stream type {0:?}")]
    UnexpectedStreamType(StreamType),
    #[error("unsupported logical encoding {0:?} for {1}")]
    UnsupportedLogicalEncoding(LogicalEncoding, &'static str),
    #[error("invalid combination of logical encodings: {0:?} + {1:?}")]
    InvalidLogicalEncodings(LogicalTechnique, LogicalTechnique),
    #[error("layer has zero size")]
    ZeroLayerSize,

    // Wire/codec decoding (bytes → primitives)
    #[error("buffer underflow: needed {0} bytes, but only {1} remain")]
    BufferUnderflow(usize, usize),
    #[error("FastPFor decode failed: expected={0} got={1}")]
    FastPforDecode(usize, usize),
    #[error("invalid RLE run length (cannot convert to usize): value={0}")]
    RleRunLenInvalid(i128),

    // Structural constraints (lengths, counts, shapes)
    #[error("geometry requires at least 1 stream, got 0")]
    GeometryWithoutStreams,
    #[error("FastPFor data byte length expected multiple of 4, got {0}")]
    InvalidFastPforByteLength(usize),
    #[error("vec2 delta stream size expected to be non-empty and multiple of 2, got {0}")]
    InvalidPairStreamSize(usize),
    #[error("stream data mismatch: expected {0}, got {1}")]
    StreamDataMismatch(&'static str, &'static str),
    #[error("IDs missing for encoding (expected Some IDs, got None)")]
    IdsMissingForEncoding,
    #[error("presence stream has {0} bits set but {1} values provided")]
    PresenceValueCountMismatch(usize, usize),
    #[error("MVT parse error: {0}")]
    MvtParse(String),
    #[error("need to encode before being able to write")]
    NeedsEncodingBeforeWriting,
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),
    #[error("struct shared dictionary requires at least 2 streams, got {0}")]
    StructSharedDictRequiresStreams(usize),
    #[error("Structs are not allowed to be optional")]
    TriedToEncodeOptionalStruct,
    #[error("struct child data streams expected exactly 1 value, got {0}")]
    UnexpectedStructChildCount(usize),
    #[error("unsupported physical encoding: {0}")]
    UnsupportedPhysicalEncoding(&'static str),
    #[error("unsupported physical encoding: {0:?} for {1}")]
    UnsupportedPhysicalEncodingForType(PhysicalEncoding, &'static str),

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

    // Wrapper errors, using `#[from]` to auto-convert from underlying error types
    #[error("FastPFor FFI error: {0}")]
    FastPforFfi(#[from] Exception),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Serde JSON error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),
    #[error("integer conversion error: {0}")]
    TryFromIntError(#[from] std::num::TryFromIntError),
    #[error("num_enum conversion error: {0}")]
    TryFromPrimitive(#[from] TryFromPrimitiveError<GeometryType>),
    #[error("UTF-8 decode error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
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
            other => std::io::Error::other(other),
        }
    }
}
