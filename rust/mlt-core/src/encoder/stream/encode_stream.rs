use std::collections::HashMap;
use std::collections::hash_map::Entry;

use crate::MltResult;
#[cfg(any(test, feature = "__private"))]
use crate::decoder::{DictionaryType, StreamMeta, StreamType};
#[cfg(any(test, feature = "__private"))]
use crate::encoder::EncodedStream;
use crate::errors::AsMltError as _;

/// Deduplicate `values` preserving insertion order.
/// Returns `(unique_strings, per_value_index)` where each entry in `per_value_index` is the
/// index into `unique_strings` for the corresponding input value.
pub(crate) fn dedup_strings<S: AsRef<str>>(values: &[S]) -> MltResult<(Vec<&str>, Vec<u32>)> {
    let mut unique: Vec<&str> = Vec::new();
    let mut index: HashMap<&str, u32> = HashMap::new();
    let mut indices = Vec::with_capacity(values.len());
    for value in values {
        let s = value.as_ref();
        let idx = match index.entry(s) {
            Entry::Occupied(e) => *e.get(),
            Entry::Vacant(e) => {
                let idx = u32::try_from(unique.len()).or_overflow()?;
                unique.push(s);
                *e.insert(idx)
            }
        };
        indices.push(idx);
    }
    Ok((unique, indices))
}

#[cfg(any(test, feature = "__private"))]
impl EncodedStream {
    /// Encodes floating-point values (`f32` / `f64`) into a stream as raw little-endian bytes.
    #[hotpath::measure]
    pub fn encode_floats<T: num_traits::ToBytes>(values: &[T]) -> MltResult<Self> {
        let mut data = Vec::with_capacity(size_of_val(values));
        for v in values {
            data.extend_from_slice(v.to_le_bytes().as_ref());
        }
        let meta = StreamMeta::new_none(StreamType::Data(DictionaryType::None), values.len())?;
        Ok(Self { meta, data })
    }
}
