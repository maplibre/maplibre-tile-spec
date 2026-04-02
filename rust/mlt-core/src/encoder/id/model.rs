use crate::encoder::EncodedStream;

/// Wire-ready encoded ID data (owns its byte buffers)
#[derive(Debug, PartialEq, Clone)]
pub struct EncodedId {
    pub(crate) presence: Option<EncodedStream>,
    pub(crate) value: EncodedIdValue,
}

/// Wire-ready encoded ID value, either 32-bit or 64-bit
#[derive(Debug, PartialEq, Clone)]
pub enum EncodedIdValue {
    Id32(EncodedStream),
    Id64(EncodedStream),
}

/// How wide are the IDs
#[derive(Debug, Clone, Copy, PartialEq, strum::EnumIter)]
#[cfg_attr(all(not(test), feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum IdWidth {
    /// 32-bit encoding
    Id32,
    /// 32-bit encoding with nulls
    OptId32,
    /// 64-bit encoding (delta + zigzag + varint)
    Id64,
    /// 64-bit encoding with nulls
    OptId64,
}
