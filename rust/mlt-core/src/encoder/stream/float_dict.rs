//! Dictionary building for float columns.

use crate::codecs::float::FloatValue;
use crate::encoder::stream::float_cost::{ENCODING_BYTE, raw_stored_bytes, varint_len};

/// A float column's distinct values and one code per element.
pub(crate) struct FloatDict<T> {
    /// Distinct values in first-appearance order, so the result does not depend on hash iteration order.
    pub(crate) values: Vec<T>,
    /// One index into `values` per input element.
    pub(crate) codes: Vec<u32>,
}

impl<T: FloatValue> FloatDict<T> {
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
    pub(crate) fn stored_bytes(&self) -> usize {
        let codes: usize = self.codes.iter().copied().map(varint_len).sum();
        let values = size_of_val(self.values.as_slice());
        let codes_stream = ENCODING_BYTE + varint_len(codes) + codes;
        let dict_stream =
            ENCODING_BYTE + varint_len(self.values.len()) + varint_len(values) + values;
        codes_stream + dict_stream
    }
}
