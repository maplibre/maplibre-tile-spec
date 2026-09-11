//! Dictionary building for float columns.

/// A float whose bit pattern is its dictionary key.
/// `-0.0` must stay a distinct entry from `0.0`, and a NaN is equal to no float at all, itself included.
pub(crate) trait FloatBits: Copy {
    type Bits: Copy + Eq + std::hash::Hash;

    #[cfg_attr(
        not(feature = "unstable-v2"),
        expect(dead_code, reason = "only the v2 float dictionary keys on bits")
    )]
    fn key(self) -> Self::Bits;
}

impl FloatBits for f32 {
    type Bits = u32;

    fn key(self) -> u32 {
        self.to_bits()
    }
}

impl FloatBits for f64 {
    type Bits = u64;

    fn key(self) -> u64 {
        self.to_bits()
    }
}

/// A float column's distinct values and one code per element.
#[cfg(feature = "unstable-v2")]
pub(crate) struct FloatDict<T> {
    /// Distinct values in first-appearance order, so the result does not depend on hash iteration order.
    pub(crate) values: Vec<T>,
    /// One index into `values` per input element.
    pub(crate) codes: Vec<u32>,
}

#[cfg(feature = "unstable-v2")]
impl<T: FloatBits> FloatDict<T> {
    /// Build a dictionary for `values`, or [`None`] when it would not be smaller.
    /// Both layouts are costed as stored bytes, headers included, assuming nothing about transport compression.
    pub(crate) fn worth_building(values: &[T]) -> Option<Self> {
        let dict = Self::build(values)?;
        (dict.stored_bytes() < raw_stored_bytes::<T>(values.len())).then_some(dict)
    }

    /// The dictionary, or [`None`] if every element is distinct.
    fn build(values: &[T]) -> Option<Self> {
        let mut seen = std::collections::HashMap::new();
        let mut dict = Self {
            values: Vec::new(),
            codes: Vec::with_capacity(values.len()),
        };
        for &value in values {
            let next = u32::try_from(dict.values.len()).ok()?;
            let code = *seen.entry(value.key()).or_insert_with(|| {
                dict.values.push(value);
                next
            });
            dict.codes.push(code);
        }
        (dict.values.len() < values.len()).then_some(dict)
    }

    /// Bytes the codes and dictionary streams occupy, headers included.
    /// The codes count is implied by context, while the dictionary's always needs its own varint.
    fn stored_bytes(&self) -> usize {
        let codes: usize = self.codes.iter().copied().map(varint_len).sum();
        let values = size_of_val(self.values.as_slice());
        let codes_stream = ENCODING_BYTE + varint_len(codes) + codes;
        let dict_stream =
            ENCODING_BYTE + varint_len(self.values.len()) + varint_len(values) + values;
        codes_stream + dict_stream
    }
}

/// The encoding byte every stream header starts with.
#[cfg(feature = "unstable-v2")]
const ENCODING_BYTE: usize = 1;

/// Bytes a raw column of `count` elements occupies, header included.
#[cfg(feature = "unstable-v2")]
fn raw_stored_bytes<T>(count: usize) -> usize {
    let values = count * size_of::<T>();
    ENCODING_BYTE + varint_len(values) + values
}

/// Encoded length of a varint, for costing a header that is not written yet.
#[cfg(feature = "unstable-v2")]
fn varint_len(value: impl TryInto<u64>) -> usize {
    let value: u64 = value.try_into().unwrap_or(u64::MAX);
    integer_encoding::VarInt::required_space(value)
}
