use crate::decoder::RawFsstData;
use crate::utils::AsUsize as _;
use crate::{Decoder, MltResult};

/// Decode an FSST-compressed byte sequence into the original bytes and value lengths,
/// charging `dec` for the output.
///
/// Takes a [`RawFsstData`] which provides the 4 streams needed for FSST decoding:
/// - `symbol_lengths`: per-symbol byte lengths (decoded as u32 values)
/// - `symbol_table`: concatenated raw symbol bytes (read as raw bytes)
/// - `lengths`: original string byte lengths (decoded as u32 values)
/// - `corpus`: the FSST-encoded payload (read as raw bytes)
///
/// The encoding uses two special cases:
/// - Byte `0xFF` (255): the next byte is a literal — output it verbatim.
/// - Any other byte `idx < symbol_lengths.len()`: expand the symbol at that index.
///
/// Returns `(decompressed_utf8_string, value_lengths)`.
pub fn decode_fsst(raw: RawFsstData<'_>, dec: &mut Decoder) -> MltResult<(String, Vec<u32>)> {
    let RawFsstData {
        symbol_lengths,
        symbol_table,
        lengths,
        corpus,
    } = raw;

    let sym_lens = symbol_lengths.decode_u32s(dec)?;
    let symbols = symbol_table.as_bytes();
    let compressed = corpus.as_bytes();

    // Build symbol offset table from lengths.
    let mut symbol_offsets = vec![0u32; sym_lens.len()];
    for i in 1..sym_lens.len() {
        symbol_offsets[i] = symbol_offsets[i - 1] + sym_lens[i - 1];
    }

    let mut output = Vec::new();
    let mut i = 0;
    while i < compressed.len() {
        let sym_idx = usize::from(compressed[i]);
        if sym_idx == 255 {
            i += 1;
            output.push(compressed[i]);
        } else if sym_idx < sym_lens.len() {
            let len = sym_lens[sym_idx].as_usize();
            let off = symbol_offsets[sym_idx].as_usize();
            output.extend_from_slice(&symbols[off..off + len]);
        }
        i += 1;
    }

    dec.consume_items::<u8>(output.len())?;
    Ok((String::from_utf8(output)?, lengths.decode_u32s(dec)?))
}

/// Raw output from FSST compression (unencoded byte buffers).
///
/// Pass to the string encoder's `write_fsst_data` helper to write these
/// streams to an encoder with a chosen [`FsstStrEncoder`](crate::encoder::FsstStrEncoder).
pub struct FsstRawData {
    /// Per-symbol byte lengths (to be written as `Length(Symbol)` stream).
    pub symbol_lengths: Vec<u32>,
    /// Concatenated raw symbol bytes (to be written as `Data(Fsst)` stream).
    pub symbol_bytes: Vec<u8>,
    /// Per-value byte lengths of the compressed corpus (to be written as `Length(Dictionary)` stream).
    pub value_lengths: Vec<u32>,
    /// FSST-compressed corpus bytes (to be written as `Data(dict_type)` stream).
    pub corpus: Vec<u8>,
}

/// Shared FSST compression kernel: train a compressor on `values` and compress the corpus.
///
/// Returns [`FsstRawData`] with the four raw byte/int buffers ready to be written to
/// an encoder via the caller's chosen integer encoders.
///
/// Stream order when written:
/// 1. Symbol lengths (`Length(Symbol)`)
/// 2. Symbol table data (`Data(Fsst)`)
/// 3. Value lengths (`Length(Dictionary)`)
/// 4. Compressed corpus (`Data(dict_type)` — supplied by the caller at write time)
///
/// Note: The FSST algorithm implementation may differ from Java's, so the
/// compressed output may not be byte-for-byte identical. Both implementations
/// are semantically compatible and can decode each other's output.
pub fn compress_fsst<S: AsRef<str>>(values: &[S]) -> FsstRawData {
    let byte_slices: Vec<&[u8]> = values.iter().map(|s| s.as_ref().as_bytes()).collect();
    let compressor = fsst::Compressor::train(&byte_slices);
    let symbols = compressor.symbol_table();
    let symbol_lengths_u8 = compressor.symbol_lengths();

    let mut symbol_bytes = Vec::new();
    for sym in symbols {
        let bytes = sym.to_u64().to_le_bytes();
        let len = sym.len();
        symbol_bytes.extend_from_slice(&bytes[..len]);
    }

    let symbol_lengths: Vec<u32> = symbol_lengths_u8
        .iter()
        .take(symbols.len())
        .map(|&l| u32::from(l))
        .collect();

    let value_lengths: Vec<u32> = values
        .iter()
        .map(|s| u32::try_from(s.as_ref().len()).unwrap_or(u32::MAX))
        .collect();

    let mut corpus = Vec::new();
    for s in values {
        corpus.extend(compressor.compress(s.as_ref().as_bytes()));
    }

    FsstRawData {
        symbol_lengths,
        symbol_bytes,
        value_lengths,
        corpus,
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::decoder::{
        DictionaryType, IntEncoding, LengthType, RawFsstData, RawStream, StreamType,
    };
    use crate::encoder::{EncodedStream, EncodedStreamData, IntEncoder};
    use crate::test_helpers::{assert_empty, dec, parser};
    use crate::utils::BinarySerializer as _;

    /// Encode the 4 FSST raw streams to wire bytes and parse them back for decoding.
    fn roundtrip(values: &[&str]) -> (String, Vec<u32>) {
        use crate::decoder::StreamMeta;
        let raw = compress_fsst(values);

        let sym_len_stream = EncodedStream::encode_u32s_of_type(
            &raw.symbol_lengths,
            IntEncoder::varint(),
            StreamType::Length(LengthType::Symbol),
        )
        .unwrap();
        let sym_table_stream = EncodedStream {
            meta: StreamMeta::new(
                StreamType::Data(DictionaryType::Fsst),
                IntEncoding::none(),
                u32::try_from(raw.symbol_lengths.len()).unwrap(),
            ),
            data: EncodedStreamData::Encoded(raw.symbol_bytes.clone()),
        };
        let lengths_stream = EncodedStream::encode_u32s_of_type(
            &raw.value_lengths,
            IntEncoder::varint(),
            StreamType::Length(LengthType::Dictionary),
        )
        .unwrap();
        let corpus_stream = EncodedStream {
            meta: StreamMeta::new(
                StreamType::Data(DictionaryType::Single),
                IntEncoding::none(),
                u32::try_from(values.len()).unwrap(),
            ),
            data: EncodedStreamData::Encoded(raw.corpus.clone()),
        };

        let streams = [
            sym_len_stream,
            sym_table_stream,
            lengths_stream,
            corpus_stream,
        ];
        let mut buffers: Vec<Vec<u8>> = Vec::new();
        for stream in &streams {
            let mut buf = Vec::new();
            buf.write_stream(stream).expect("write_stream failed");
            buffers.push(buf);
        }
        let mut raw_streams = Vec::new();
        for buf in &buffers {
            raw_streams.push(assert_empty(RawStream::from_bytes(buf, &mut parser())));
        }
        let [s0, s1, s2, s3] = raw_streams.try_into().expect("expected 4 streams");
        let raw = RawFsstData::new(s0, s1, s2, s3).expect("RawFsstData::new failed");
        decode_fsst(raw, &mut dec()).expect("decode_fsst failed")
    }

    #[test]
    fn test_fsst_roundtrip_empty() {
        let (corpus, lengths) = roundtrip(&[]);
        assert!(corpus.is_empty());
        assert!(lengths.is_empty());
    }

    #[rstest]
    #[case::longer(&["hello world", "hello rust", "hello fsst", "world"])]
    #[case::short(&["hello"])]
    fn automatic_optimization_roundtrip(#[case] values: &[&str]) {
        let (corpus, lengths) = roundtrip(values);
        let mut offset = 0;
        for (s, &len) in values.iter().zip(&lengths) {
            let len = len as usize;
            assert_eq!(&corpus[offset..offset + len], *s);
            offset += len;
        }
    }
}
