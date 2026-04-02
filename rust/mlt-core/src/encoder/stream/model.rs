use crate::v01::StreamMeta;

/// Owned variant of [`RawStream`](crate::v01::RawStream).
#[derive(Debug, PartialEq, Clone)]
pub struct EncodedStream {
    pub meta: StreamMeta,
    pub(crate) data: EncodedStreamData,
}

#[derive(PartialEq, Clone)]
pub enum EncodedStreamData {
    VarInt(Vec<u8>),
    Encoded(Vec<u8>),
}
