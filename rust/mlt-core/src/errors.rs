use std::num::TryFromIntError;

use num_enum::TryFromPrimitiveError;

use crate::decoder::{
    GeometryType, LogicalCombination, LogicalEncoding, LogicalTechnique, PhysicalEncoding,
    StreamType, ValueKind,
};

pub type MltResult<T> = Result<T, MltError>;
pub(crate) type MltRefResult<'a, T> = Result<(&'a [u8], T), MltError>;

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
    #[error("missing layer name")]
    MissingLayerName,
    #[error("invalid extent: {0}")]
    InvalidExtent(u32),
    #[error("missing property name")]
    MissingPropertyName,
    #[error("duplicate column name {name}: the {role} column repeats the {taken_by} column")]
    DuplicateColumnName {
        name: String,
        role: crate::tile::ColumnRole,
        taken_by: crate::tile::ColumnRole,
    },
    #[error("feature property count mismatch: expected {expected}, got {actual}")]
    PropertyLengthMismatch { expected: usize, actual: usize },
    #[error("property {index} kind mismatch: expected {expected:?}, got {actual:?}")]
    PropertyKindMismatch {
        index: usize,
        expected: crate::tile::PropKind,
        actual: crate::tile::PropKind,
    },
    #[error("staged column {column} feature count mismatch: expected {expected}, got {actual}")]
    StagedFeatureCountMismatch {
        column: String,
        expected: usize,
        actual: usize,
    },
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
    #[error("error parsing v2 stream encoding byte: 0x{0:02X}")]
    ParsingEncodingByte(u8),
    #[cfg(feature = "unstable-v2")]
    #[error(
        "a v2 {0:?} stream carries no value count, and nothing in its context implies one: encoding byte 0x{1:02X}"
    )]
    StreamWithoutCount(StreamType, u8),
    #[cfg(feature = "unstable-v2")]
    #[error("error parsing v2 geometry layout: code={0}")]
    ParsingGeoLayout(u8),
    #[cfg(feature = "unstable-v2")]
    #[error(
        "v2 geometry layout {0} gives no per-feature vertex count, so it cannot carry m-values"
    )]
    MValuesNeedVertexCounts(&'static str),
    #[cfg(feature = "unstable-v2")]
    #[error("the v2 layer layout byte claims an m-value section, but it holds no columns")]
    EmptyMValueSection,
    #[cfg(feature = "unstable-v2")]
    #[error(
        "m-value column {name} has no implied count, so its leading stream must write one: encoding byte 0x{byte:02X}"
    )]
    MValueImplicitCount { name: String, byte: u8 },
    #[cfg(feature = "unstable-v2")]
    #[error(
        "m-value column {name} holds {actual} values, but its features have {expected} vertices"
    )]
    MValueColumnLengthMismatch {
        name: String,
        expected: usize,
        actual: usize,
    },
    #[cfg(feature = "unstable-v2")]
    #[error(
        "m-value column {name} holds {actual} values, so a feature's run of {count} at offset {start} is out of range"
    )]
    MValueRunOutOfRange {
        name: String,
        start: usize,
        count: usize,
        actual: usize,
    },
    #[cfg(feature = "unstable-v2")]
    #[error("a feature carries {actual} m-value columns, but the layer has {expected}")]
    MValueColumnCountMismatch { expected: usize, actual: usize },
    #[cfg(feature = "unstable-v2")]
    #[error("m-value key {index} belongs to another layer: this feature has {columns} columns")]
    UnknownMValueKey { index: usize, columns: usize },
    #[cfg(feature = "unstable-v2")]
    #[error("m-value {index} kind mismatch: expected {expected:?}, got {actual:?}")]
    MValueKindMismatch {
        index: usize,
        expected: crate::tile::PropKind,
        actual: crate::tile::PropKind,
    },
    #[cfg(feature = "unstable-v2")]
    #[error("m-value {index} holds {actual} values, but the feature has {expected} vertices")]
    MValueVertexCountMismatch {
        index: usize,
        expected: usize,
        actual: usize,
    },
    #[cfg(feature = "unstable-v2")]
    #[error("m-values are a v2 feature, so layer {0} cannot be written as v1")]
    MValuesNeedV2(String),
    #[cfg(feature = "unstable-v2")]
    #[error("nested properties are a v2 feature, so layer {0} cannot be written as v1")]
    NestedNeedsV2(String),
    #[cfg(feature = "unstable-v2")]
    #[error("duplicate field name in a struct node: {0}")]
    DuplicateFieldName(String),
    #[cfg(feature = "unstable-v2")]
    #[error("a struct node holds no fields")]
    EmptyStructNode,
    #[cfg(feature = "unstable-v2")]
    #[error("nested column {0} has a scalar root, which is an ordinary column")]
    NestedRootIsLeaf(String),
    #[cfg(feature = "unstable-v2")]
    #[error("a nested column is {0} levels deep, more than the 8 the format allows")]
    NestedTooDeep(usize),
    #[cfg(feature = "unstable-v2")]
    #[error("a nested lengths stream sums to {expected}, but the node below it holds {actual}")]
    NestedCountMismatch { expected: u32, actual: u32 },
    #[cfg(feature = "unstable-v2")]
    #[error(
        "nested node {name} has no implied count, so its leading stream must write one: encoding byte 0x{byte:02X}"
    )]
    NestedImplicitCount { name: String, byte: u8 },
    #[cfg(feature = "unstable-v2")]
    #[error("a feature carries {actual} nested columns, but the layer has {expected}")]
    NestedColumnCountMismatch { expected: usize, actual: usize },
    #[cfg(feature = "unstable-v2")]
    #[error("nested key {index} belongs to another layer: this feature has {columns} columns")]
    UnknownNestedKey { index: usize, columns: usize },
    #[cfg(feature = "unstable-v2")]
    #[error("nested column {index} ({name}) was given a value of another shape")]
    NestedValueMismatch { index: usize, name: String },
    #[cfg(feature = "unstable-v2")]
    #[error("nested column {index} was given a value of another shape")]
    NestedShapeMismatch { index: usize },
    #[cfg(feature = "unstable-v2")]
    #[error("nested column {name} runs over {expected} values, but its root holds {actual}")]
    NestedRootCountMismatch {
        name: String,
        expected: u32,
        actual: u32,
    },
    #[cfg(feature = "unstable-v2")]
    #[error(
        "nested node {name} must write its presence as a raw bitmap of one bit per value: encoding byte 0x{byte:02X}"
    )]
    NestedPresenceEncoding { name: String, byte: u8 },
    #[cfg(feature = "unstable-v2")]
    #[error("a nested lengths stream holds {len} lengths, so it has no row {row}")]
    NestedRowOutOfRange { row: usize, len: usize },
    #[cfg(feature = "unstable-v2")]
    #[error("a nested map holds {len} keys, so it has no key for entry {entry}")]
    NestedKeyOutOfRange { entry: usize, len: usize },
    #[cfg(feature = "unstable-v2")]
    #[error("a nested leaf holds {len} values, so it has no value at index {index}")]
    NestedValueOutOfRange { index: usize, len: usize },
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
    #[error("logical encoding {0:?} is not one a {1:?} stream has")]
    LogicalEncodingNotInKind(LogicalCombination, ValueKind),
    #[error("layer has zero size")]
    ZeroLayerSize,
    #[error("bit-packed stream width {0} is outside 1..=32")]
    #[cfg(feature = "unstable-v2")]
    ParsingBitWidth(u32),
    #[error("The encoder used to optimise data is incompatible")]
    BadEncoderDataCombination,
    #[error("StagedLayer::encode_explicit requires Encoder.explicit to be Some(_)")]
    MissingExplicitEncoder,

    // Wire/codec decoding (bytes -> primitives)
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
    #[error("IDs missing for encoding (expected Some IDs, got None)")]
    IdsMissingForEncoding,
    #[error("missing struct encoder for struct")]
    MissingStructEncoderForStruct,
    #[error("previous decode/parsing attempt failed")]
    PriorParseFailure,
    #[error("FSST-compressed data is malformed: {0}")]
    MalformedFsst(&'static str),
    #[cfg(feature = "unstable-v2")]
    #[error("dictionary code {0} is out of range for a dictionary of {1} values")]
    DictionaryCodeOutOfRange(u32, usize),
    #[cfg(feature = "unstable-v2")]
    #[error(
        "front-coded lengths stream holds a prefix and a suffix length per entry, so its {0} values cannot be split in two"
    )]
    FrontCodedOddLengthCount(usize),
    #[cfg(feature = "unstable-v2")]
    #[error(
        "front-coded entry {index} shares {shared} bytes with a predecessor of only {available}"
    )]
    FrontCodedPrefixTooLong {
        index: usize,
        shared: usize,
        available: usize,
    },
    #[cfg(feature = "unstable-v2")]
    #[error(
        "front-coded entry {index} needs {needed} suffix bytes, but {available} remain in the blob"
    )]
    FrontCodedSuffixOutOfBounds {
        index: usize,
        needed: usize,
        available: usize,
    },
    #[cfg(feature = "unstable-v2")]
    #[error("front-coded dictionary leaves {0} suffix bytes after its last entry")]
    FrontCodedTrailingSuffixBytes(usize),
    #[cfg(feature = "unstable-v2")]
    #[error("no ALP parameters return this float column bit-for-bit")]
    NoAlpParameters,
    #[error("invalid ALP parameters: e={0}, f={1}")]
    InvalidAlpParams(u8, u8),
    #[error("presence stream has {0} bits set but {1} values provided")]
    PresenceValueCountMismatch(usize, usize),
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
    #[error("mixed property types are not allowed in column {0} ({1})")]
    MixedPropertyTypes(usize, String),
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
    #[error("Morton stream uses {0} bits, but at most 16 bits are supported")]
    InvalidMortonBits(u32),

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
    #[error("geometry: ring lengths without part lengths")]
    RingLengthsWithoutPartLengths,

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
    #[error("MVT error: {0}")]
    Mvt(#[from] fast_mvt::MvtError),
    #[error("MVT JSON value error: {0}")]
    MvtJsonValue(#[from] fast_mvt::MvtJsonValueError),
}

impl From<MltError> for std::io::Error {
    fn from(value: MltError) -> Self {
        if let MltError::Io(e) = value {
            e
        } else {
            Self::other(value)
        }
    }
}

pub(crate) trait AsMltError<T> {
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
pub(crate) fn fail_if_invalid_stream_size(actual: usize, expected: usize) -> MltResult<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(MltError::InvalidDecodingStreamSize(actual, expected))
    }
}
