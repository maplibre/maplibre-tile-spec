use thiserror::Error;

pub type MltResult<T> = Result<T, MltError>;

#[derive(Error, Debug)]
pub enum MltError {
    #[error("Unable to parse property: {0}")]
    PropertyParseError(String),
    #[error("Unsupported key value type: {0}")]
    UnsupportedKeyType(String),
}
