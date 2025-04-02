use thiserror::Error;

pub type MltResult<T> = Result<T, MltError>;

#[derive(Error, Debug)]
pub enum MltError {
    #[error("Unable to parse property: {0}")]
    PropertyParseError(String),
    #[error("Unsupported key value type: {0}")]
    UnsupportedKeyType(String),
    #[error("Failed to read file: {0}")]
    FileReadError(#[from] std::io::Error),
    #[error("Unsupported geometry type: {0}")]
    UnsupportedGeometryType(String),
    #[error("Failed to decode protobuf: {0}")]
    DecodeError(#[from] prost::DecodeError),
}
