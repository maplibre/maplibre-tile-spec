use std::collections::HashMap;
use std::collections::hash_map::Entry;

#[cfg(test)]
use super::encoder::IntEncoder;
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
        let bytes = encode_bools_to_bytes(values);
        let data = encode_byte_rle(&bytes);
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

    #[cfg(test)]
    pub fn encode_i8s(values: &[i8], encoding: IntEncoder) -> MltResult<Self> {
        let as_i32: Vec<i32> = values.iter().map(|&v| i32::from(v)).collect();
        let (physical_u32s, logical_encoding) = encoding.logical.encode_i32s(&as_i32)?;
        let num_values = u32::try_from(physical_u32s.len())?;
        let (data, physical_encoding) = encoding.physical.encode_u32s(physical_u32s)?;
        Ok(Self {
            meta: StreamMeta::new(
                StreamType::Data(DictionaryType::None),
                IntEncoding::new(logical_encoding, physical_encoding),
                num_values,
            ),
            data,
        })
    }

    #[cfg(test)]
    pub fn encode_u8s(values: &[u8], encoding: IntEncoder) -> MltResult<Self> {
        let as_u32: Vec<u32> = values.iter().map(|&v| u32::from(v)).collect();
        let (physical_u32s, logical_encoding) = encoding.logical.encode_u32s(&as_u32)?;
        let num_values = u32::try_from(physical_u32s.len())?;
        let (data, physical_encoding) = encoding.physical.encode_u32s(physical_u32s)?;
        Ok(Self {
            meta: StreamMeta::new(
                StreamType::Data(DictionaryType::None),
                IntEncoding::new(logical_encoding, physical_encoding),
                num_values,
            ),
            data,
        })
    }

    #[cfg(test)]
    pub fn encode_i32s(values: &[i32], encoding: IntEncoder) -> MltResult<Self> {
        let (physical_u32s, logical_encoding) = encoding.logical.encode_i32s(values)?;
        let num_values = u32::try_from(physical_u32s.len())?;
        let (data, physical_encoding) = encoding.physical.encode_u32s(physical_u32s)?;
        Ok(Self {
            meta: StreamMeta::new(
                StreamType::Data(DictionaryType::None),
                IntEncoding::new(logical_encoding, physical_encoding),
                num_values,
            ),
            data,
        })
    }

    #[cfg(test)]
    pub fn encode_i64s(values: &[i64], encoding: IntEncoder) -> MltResult<Self> {
        let (physical_u64s, logical_encoding) = encoding.logical.encode_i64s(values)?;
        let num_values = u32::try_from(physical_u64s.len())?;
        let (data, physical_encoding) = encoding.physical.encode_u64s(physical_u64s)?;
        Ok(Self {
            meta: StreamMeta::new(
                StreamType::Data(DictionaryType::None),
                IntEncoding::new(logical_encoding, physical_encoding),
                num_values,
            ),
            data,
        })
    }

    #[cfg(test)]
    pub fn encode_u32s(values: &[u32], encoding: IntEncoder) -> MltResult<Self> {
        Self::encode_u32s_of_type(values, encoding, StreamType::Data(DictionaryType::None))
    }

    #[cfg(test)]
    pub fn encode_u32s_of_type(
        values: &[u32],
        encoding: IntEncoder,
        stream_type: StreamType,
    ) -> MltResult<Self> {
        let (physical_u32s, logical_encoding) = encoding.logical.encode_u32s(values)?;
        let num_values = u32::try_from(physical_u32s.len())?;
        let (data, physical_encoding) = encoding.physical.encode_u32s(physical_u32s)?;
        Ok(Self {
            meta: StreamMeta::new(
                stream_type,
                IntEncoding::new(logical_encoding, physical_encoding),
                num_values,
            ),
            data,
        })
    }

    #[cfg(test)]
    pub fn encode_u64s(values: &[u64], encoding: IntEncoder) -> MltResult<Self> {
        let (physical_u64s, logical_encoding) = encoding.logical.encode_u64s(values)?;
        let num_values = u32::try_from(physical_u64s.len())?;
        let (data, physical_encoding) = encoding.physical.encode_u64s(physical_u64s)?;
        Ok(Self {
            meta: StreamMeta::new(
                StreamType::Data(DictionaryType::None),
                IntEncoding::new(logical_encoding, physical_encoding),
                num_values,
            ),
            data,
        })
    }
}
