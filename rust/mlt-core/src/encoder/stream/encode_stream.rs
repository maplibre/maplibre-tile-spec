use std::collections::HashMap;
use std::collections::hash_map::Entry;

use crate::MltResult;
use crate::decoder::{DictionaryType, StreamMeta, StreamType};
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

impl EncodedStream {
    /// Encodes `f32`s into a stream
    #[hotpath::measure]
    pub fn encode_f32(values: &[f32]) -> MltResult<Self> {
        let data = values
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect::<Vec<u8>>();
        let meta = StreamMeta::new_none(StreamType::Data(DictionaryType::None), values.len())?;
        Ok(Self { meta, data })
    }

    /// Encodes `f64`s into a stream
    #[hotpath::measure]
    pub fn encode_f64(values: &[f64]) -> MltResult<Self> {
        let data = values
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<u8>>();
        let meta = StreamMeta::new_none(StreamType::Data(DictionaryType::None), values.len())?;
        Ok(Self { meta, data })
    }
}
