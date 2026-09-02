//! Stored size of a v2 float column, for choosing between its encodings.

/// The encoding byte every stream header starts with.
pub(crate) const ENCODING_BYTE: usize = 1;

/// Bytes a raw column of `count` elements occupies, header included.
pub(crate) fn raw_stored_bytes<T>(count: usize) -> usize {
    let values = count * size_of::<T>();
    ENCODING_BYTE + varint_len(values) + values
}

/// Bytes a varint-coded payload occupies, including its byte-length varint.
pub(crate) fn data_bytes(values: impl Iterator<Item = u64>) -> usize {
    let data: usize = values.map(varint_len).sum();
    varint_len(data) + data
}

/// Encoded length of a varint, for costing a header that is not written yet.
/// Signed types are measured through their zigzag form, which is how they are written.
pub(crate) fn varint_len(value: impl integer_encoding::VarInt) -> usize {
    value.required_space()
}
