pub type MvtResult<T> = Result<T, MvtError>;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum MvtError {
    #[error("protobuf decode error: {0}")]
    Decode(#[from] buffa::DecodeError),

    #[error("protobuf encode error: {0}")]
    Encode(#[from] buffa::EncodeError),

    #[error("duplicate layer name: {0}")]
    DuplicateLayer(String),

    #[error("missing required layer name")]
    MissingLayerName,

    #[error("invalid extent 0")]
    InvalidExtent,

    #[error("unsupported layer version {version} for layer {layer}")]
    UnsupportedVersion { layer: String, version: u32 },

    #[error("invalid feature tags length: {0}")]
    InvalidTagsLength(usize),

    #[error("invalid key index {0}")]
    InvalidKeyIndex(u32),

    #[error("invalid value index {0}")]
    InvalidValueIndex(u32),

    #[error("invalid value field {0}")]
    InvalidValueField(u32),

    #[error("invalid geometry command stream")]
    InvalidGeometry,

    #[error("unsupported geometry type: {0}")]
    UnsupportedGeometry(&'static str),

    #[error("command count exceeds supported range: {0}")]
    CommandCount(u32),

    #[error("tile index exceeds supported range: {0}")]
    IndexOverflow(usize),
}
