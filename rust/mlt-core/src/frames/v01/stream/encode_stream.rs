use std::collections::HashMap;
use std::collections::hash_map::Entry;

use crate::MltResult;
use crate::codecs::bytes::encode_bools_to_bytes;
use crate::codecs::fsst::compress_fsst;
use crate::codecs::rle::encode_byte_rle;
use crate::utils::strings_to_lengths;
use crate::v01::{
    DictionaryType, EncodedPlainData, EncodedStream, EncodedStreamData, EncodedStringsEncoding,
    FsstStrEncoder, IntEncoder, IntEncoding, LengthType, LogicalEncoding, OffsetType,
    PhysicalEncoding, RleMeta, StreamMeta, StreamType,
};

impl EncodedStream {
    /// Creates an empty stream
    #[must_use]
    pub fn empty_without_encoding() -> Self {
        Self {
            meta: StreamMeta::new(
                StreamType::Data(DictionaryType::None),
                IntEncoding::none(),
                0,
            ),
            data: EncodedStreamData::Encoded(Vec::new()),
        }
    }

    #[must_use]
    fn plain(data: Vec<u8>, num_values: u32) -> Self {
        Self::plain_with_type(data, num_values, DictionaryType::None)
    }

    /// Creates a plain stream with values encoded literally
    #[must_use]
    fn plain_with_type(data: Vec<u8>, num_values: u32, dict_type: DictionaryType) -> Self {
        let meta = StreamMeta::new(StreamType::Data(dict_type), IntEncoding::none(), num_values);
        let data = EncodedStreamData::Encoded(data);
        Self { meta, data }
    }

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

        Ok(Self::plain(data, num_values))
    }

    /// Encodes `f64`s into a stream
    pub fn encode_f64(values: &[f64]) -> MltResult<Self> {
        let num_values = u32::try_from(values.len())?;
        let data = values
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<u8>>();

        Ok(Self::plain(data, num_values))
    }

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

    pub fn encode_u32s(values: &[u32], encoding: IntEncoder) -> MltResult<Self> {
        Self::encode_u32s_of_type(values, encoding, StreamType::Data(DictionaryType::None))
    }

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

    /// Encode a sequence of strings into a length stream and a data stream.
    pub fn encode_strings_with_type<S: AsRef<str>>(
        values: &[S],
        length_encoding: IntEncoder,
        length_type: LengthType,
        dict_type: DictionaryType,
    ) -> MltResult<EncodedStringsEncoding> {
        let lengths = strings_to_lengths(values)?;
        let data: Vec<u8> = values
            .iter()
            .flat_map(|s| s.as_ref().as_bytes().iter().copied())
            .collect();
        Ok(EncodedStringsEncoding::Plain(EncodedPlainData::new(
            Self::encode_u32s_of_type(&lengths, length_encoding, StreamType::Length(length_type))?,
            Self::plain_with_type(data, u32::try_from(values.len())?, dict_type),
        )?))
    }

    /// Encode a sequence of strings using FSST compression.
    ///
    /// Produces 5 streams:
    /// 1. Symbol lengths stream (Length, `LengthType::Symbol`)
    /// 2. Symbol table data stream (Data, `DictionaryType::Fsst`)
    /// 3. Value lengths stream (Length, `LengthType::Dictionary`)
    /// 4. Compressed corpus stream (Data, `dict_type`)
    /// 5. Offset indices stream (Offset, `OffsetType::String`)
    ///
    /// The dictionary type of the compressed corpus stream is determined by the
    /// `dict_type` argument passed to this function.
    ///
    /// Note: The FSST algorithm implementation may differ from Java's, so the
    /// compressed output may not be byte-for-byte identical. Both implementations
    /// are semantically compatible and can decode each other's output.
    pub fn encode_strings_fsst_with_type<S: AsRef<str>>(
        values: &[S],
        encoding: FsstStrEncoder,
        dict_type: DictionaryType,
    ) -> MltResult<EncodedStringsEncoding> {
        let fsst_data = compress_fsst(values, encoding, dict_type)?;
        let value_cnt = u32::try_from(values.len())?;
        let offsets = (0..value_cnt).collect::<Vec<_>>();
        let offsets = Self::encode_u32s_of_type(
            &offsets,
            encoding.dict_lengths,
            StreamType::Offset(OffsetType::String),
        )?;
        Ok(EncodedStringsEncoding::FsstDictionary { fsst_data, offsets })
    }

    /// Deduplicate a slice of strings, returning the unique strings and per-value indices.
    fn dedup_strings<S: AsRef<str>>(values: &[S]) -> (Vec<String>, Vec<u32>) {
        let mut unique: Vec<String> = Vec::new();
        let mut index: HashMap<String, u32> = HashMap::new();
        let mut indices = Vec::with_capacity(values.len());
        for s in values.iter().map(|s| s.as_ref().to_owned()) {
            let idx = match index.entry(s.clone()) {
                Entry::Occupied(e) => *e.get(),
                Entry::Vacant(e) => {
                    let idx =
                        u32::try_from(unique.len()).expect("unique string count exceeds u32::MAX");
                    e.insert(idx);
                    unique.push(s);
                    idx
                }
            };
            indices.push(idx);
        }
        (unique, indices)
    }

    /// Encode a deduplicated plain dictionary: unique strings + per-feature offset indices.
    pub fn encode_strings_dict<S: AsRef<str>>(
        values: &[S],
        length_encoding: IntEncoder,
        offsets_encoding: IntEncoder,
    ) -> MltResult<EncodedStringsEncoding> {
        let (unique, offset_indices) = Self::dedup_strings(values);
        let unique_refs: Vec<&str> = unique.iter().map(String::as_str).collect();
        let lengths = strings_to_lengths(&unique_refs)?;
        let data: Vec<u8> = unique_refs
            .iter()
            .flat_map(|s| s.as_bytes().iter().copied())
            .collect();
        let plain_data = EncodedPlainData::new(
            Self::encode_u32s_of_type(
                &lengths,
                length_encoding,
                StreamType::Length(LengthType::Dictionary),
            )?,
            Self::plain_with_type(
                data,
                u32::try_from(unique_refs.len())?,
                DictionaryType::Single,
            ),
        )?;
        let offsets = Self::encode_u32s_of_type(
            &offset_indices,
            offsets_encoding,
            StreamType::Offset(OffsetType::String),
        )?;
        Ok(EncodedStringsEncoding::Dictionary {
            plain_data,
            offsets,
        })
    }

    /// Encode a deduplicated FSST dictionary: FSST-compressed unique strings + per-feature offsets.
    pub fn encode_strings_fsst_dict<S: AsRef<str>>(
        values: &[S],
        encoding: FsstStrEncoder,
        offsets_encoding: IntEncoder,
    ) -> MltResult<EncodedStringsEncoding> {
        let (unique, offset_indices) = Self::dedup_strings(values);
        let unique_refs: Vec<&str> = unique.iter().map(String::as_str).collect();
        let fsst_data = compress_fsst(&unique_refs, encoding, DictionaryType::Single)?;
        let offsets = Self::encode_u32s_of_type(
            &offset_indices,
            offsets_encoding,
            StreamType::Offset(OffsetType::String),
        )?;
        Ok(EncodedStringsEncoding::FsstDictionary { fsst_data, offsets })
    }

    /// Encode strings with FSST (4 streams, no offset). For shared dictionary struct columns;
    /// each child has its own offset stream.
    pub fn encode_strings_fsst_plain_with_type<S: AsRef<str>>(
        values: &[S],
        encoding: FsstStrEncoder,
        dict_type: DictionaryType,
    ) -> MltResult<EncodedStringsEncoding> {
        Ok(EncodedStringsEncoding::FsstPlain(compress_fsst(
            values, encoding, dict_type,
        )?))
    }
}
