use crate::utils::AsUsize as _;
use crate::v01::{
    DictionaryType, EncodedFsstData, EncodedStream, EncodedStreamData, FsstStrEncoder, IntEncoding,
    LengthType, RawFsstData, StreamMeta, StreamType,
};
use crate::{Decoder, MltError, MltResult};

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
pub fn decode_fsst(
    raw: RawFsstData<'_>,
    dec: &mut Decoder,
) -> Result<(String, Vec<u32>), MltError> {
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

/// Shared FSST compression kernel: train a compressor on `values`, compress the corpus,
/// and return the 4 streams as an [`EncodedFsstData`].
///
/// 1. Symbol lengths stream (Length, `LengthType::Symbol`)
/// 2. Symbol table data stream (Data, `DictionaryType::Fsst`)
/// 3. Value lengths stream (Length, `LengthType::Dictionary`)
/// 4. Compressed corpus stream (Data, `dict_type`)
///
/// Note: The FSST algorithm implementation may differ from Java's, so the
/// compressed output may not be byte-for-byte identical. Both implementations
/// are semantically compatible and can decode each other's output.
pub fn compress_fsst<S: AsRef<str>>(
    values: &[S],
    encoding: FsstStrEncoder,
    dict_type: DictionaryType,
) -> MltResult<EncodedFsstData> {
    // Build byte slices for training
    let byte_slices: Vec<&[u8]> = values.iter().map(|s| s.as_ref().as_bytes()).collect();
    // Train FSST compressor on the corpus
    let compressor = fsst::Compressor::train(&byte_slices);
    let symbols = compressor.symbol_table();
    let symbol_lengths_u8 = compressor.symbol_lengths();

    // Build concatenated symbol bytes (only the actual bytes for each symbol)
    let mut symbol_bytes = Vec::new();
    for sym in symbols {
        let bytes = sym.to_u64().to_le_bytes();
        let len = sym.len();
        symbol_bytes.extend_from_slice(&bytes[..len]);
    }

    // Convert symbol lengths to u32 for encoding
    let symbol_lengths: Vec<u32> = symbol_lengths_u8
        .iter()
        .take(symbols.len())
        .map(|&l| u32::from(l))
        .collect();

    // Compress all strings and concatenate into a single corpus
    let mut compressed = Vec::new();
    for s in values {
        compressed.extend(compressor.compress(s.as_ref().as_bytes()));
    }

    // Get original string lengths (UTF-8 byte lengths)
    let value_lengths: Vec<u32> = values
        .iter()
        .map(|s| u32::try_from(s.as_ref().len()))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(EncodedFsstData {
        symbol_lengths: EncodedStream::encode_u32s_of_type(
            &symbol_lengths,
            encoding.symbol_lengths,
            StreamType::Length(LengthType::Symbol),
        )?,
        symbol_table: EncodedStream {
            meta: StreamMeta::new(
                StreamType::Data(DictionaryType::Fsst),
                IntEncoding::none(),
                u32::try_from(symbol_lengths.len())?,
            ),
            data: EncodedStreamData::Encoded(symbol_bytes),
        },
        lengths: EncodedStream::encode_u32s_of_type(
            &value_lengths,
            encoding.dict_lengths,
            StreamType::Length(LengthType::Dictionary),
        )?,
        corpus: EncodedStream {
            meta: StreamMeta::new(
                StreamType::Data(dict_type),
                IntEncoding::none(),
                u32::try_from(values.len())?,
            ),
            data: EncodedStreamData::Encoded(compressed),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{assert_empty, dec, parser};
    use crate::utils::BinarySerializer as _;
    use crate::v01::{FsstStrEncoder, IntEncoder, RawFsstData};

    fn roundtrip(values: &[&str]) -> (String, Vec<u32>) {
        let encoding = FsstStrEncoder {
            symbol_lengths: IntEncoder::varint(),
            dict_lengths: IntEncoder::varint(),
        };
        let encoded =
            compress_fsst(values, encoding, DictionaryType::Single).expect("compress_fsst failed");

        // Serialize each of the 4 streams to bytes, then parse them back.
        let streams = encoded.streams();
        let mut buffers: Vec<Vec<u8>> = Vec::new();
        for stream in streams {
            let mut buf = Vec::new();
            buf.write_stream(stream).expect("write_stream failed");
            buffers.push(buf);
        }
        let mut raw_streams = Vec::new();
        for buf in &buffers {
            let (remaining, raw) =
                crate::v01::RawStream::from_bytes(buf, &mut parser()).expect("from_bytes failed");
            assert_empty(remaining);
            raw_streams.push(raw);
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

    #[test]
    fn test_fsst_roundtrip_single() {
        let values = ["hello"];
        let (corpus, lengths) = roundtrip(&values);
        let mut offset = 0;
        for (s, &len) in values.iter().zip(&lengths) {
            let len = len as usize;
            assert_eq!(&corpus[offset..offset + len], *s);
            offset += len;
        }
    }

    #[test]
    fn test_fsst_roundtrip_multiple() {
        let values = ["hello world", "hello rust", "hello fsst", "world"];
        let (corpus, lengths) = roundtrip(&values);
        let mut offset = 0;
        for (s, &len) in values.iter().zip(&lengths) {
            let len = len as usize;
            assert_eq!(&corpus[offset..offset + len], *s);
            offset += len;
        }
    }
}
