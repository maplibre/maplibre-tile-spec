use std::collections::HashMap;
use std::collections::hash_map::Entry;

use crate::MltResult;
use crate::codecs::bytes::encode_bools_to_bytes;
use crate::codecs::rle::encode_byte_rle;
use crate::decoder::{
    DictionaryType, IntEncoding, LogicalEncoding, PhysicalEncoding, RleMeta, StreamMeta, StreamType,
};
use crate::encoder::{EncodedStream, EncodedStreamData};
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
    /// Encode a boolean data stream: byte-RLE <- packed bitmap <- `Vec<bool>`
    /// Boolean streams always use byte-RLE encoding with `LogicalEncoding::Rle` metadata.
    /// The `RleMeta` values are computed by readers from the stream itself.
    #[hotpath::measure]
    pub fn encode_bools(values: &[bool]) -> MltResult<Self> {
        Self::encode_bools_with_type(values, StreamType::Data(DictionaryType::None))
    }

    /// Encode a presence/nullability stream
    ///
    /// Identical to [`Self::encode_bools`] except the stream type is [`StreamType::Present`]
    pub fn encode_presence(values: &[bool]) -> MltResult<Self> {
        Self::encode_bools_with_type(values, StreamType::Present)
    }

    /// Encode a boolean data stream: byte-RLE <- packed bitmap <- `Vec<bool>`
    fn encode_bools_with_type(values: &[bool], stream_type: StreamType) -> MltResult<Self> {
        let num_values = u32::try_from(values.len())?;
        let mut bools_bytes = Vec::new();
        encode_bools_to_bytes(values, &mut bools_bytes);
        let mut data = Vec::new();
        encode_byte_rle(&bools_bytes, &mut data);
        let runs = num_values.div_ceil(8);
        let num_rle_values = u32::try_from(data.len())?;
        let meta = StreamMeta::new(
            stream_type,
            IntEncoding::new(
                LogicalEncoding::Rle(RleMeta {
                    runs,
                    num_rle_values,
                }),
                PhysicalEncoding::None,
            ),
            num_values,
        );
        Ok(Self {
            meta,
            data: EncodedStreamData::Encoded(data),
        })
    }

    /// Encodes `f32`s into a stream
    #[hotpath::measure]
    pub fn encode_f32(values: &[f32]) -> MltResult<Self> {
        let num_values = u32::try_from(values.len())?;
        let data = values
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect::<Vec<u8>>();
        let meta = StreamMeta::new(
            StreamType::Data(DictionaryType::None),
            IntEncoding::none(),
            num_values,
        );
        Ok(Self {
            meta,
            data: EncodedStreamData::Encoded(data),
        })
    }

    /// Encodes `f64`s into a stream
    #[hotpath::measure]
    pub fn encode_f64(values: &[f64]) -> MltResult<Self> {
        let num_values = u32::try_from(values.len())?;
        let data = values
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<u8>>();
        let meta = StreamMeta::new(
            StreamType::Data(DictionaryType::None),
            IntEncoding::none(),
            num_values,
        );
        Ok(Self {
            meta,
            data: EncodedStreamData::Encoded(data),
        })
    }
}
