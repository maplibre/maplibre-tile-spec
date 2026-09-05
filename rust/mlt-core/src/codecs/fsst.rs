use usize_cast::IntoUsize as _;

use crate::decoder::RawFsstData;
use crate::{Decoder, MltError, MltResult};

/// The corpus byte announcing that the next byte is a literal.
const ESCAPE_MARKER: u8 = 0xFF;

/// Decode an FSST-compressed byte sequence into the original bytes and value lengths,
/// charging `dec` for the output.
///
/// Takes a `RawFsstData` which provides the 4 streams needed for FSST decoding:
/// - `symbol_lengths`: per-symbol byte lengths (decoded as u32 values)
/// - `symbol_table`: concatenated raw symbol bytes (read as raw bytes)
/// - `lengths`: original string byte lengths (decoded as u32 values)
/// - `corpus`: the FSST-encoded payload (read as raw bytes)
///
/// The encoding uses two special cases:
/// - Byte `0xFF` (255): the next byte is a literal - output it verbatim.
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

    let sym_lens = symbol_lengths.decode_ints::<u32>(dec)?;
    let symbols = symbol_table.data;
    let compressed = corpus.data;

    // Split the symbol table into one slice per symbol up front, so the decode loop below is a
    // single lookup per corpus byte.
    let mut symbols_by_index = Vec::with_capacity(sym_lens.len());
    let mut rest = symbols;
    for &len in &sym_lens {
        let Some((symbol, tail)) = rest.split_at_checked(len.into_usize()) else {
            return Err(MltError::MalformedFsst(
                "symbol lengths overrun the symbol table",
            ));
        };
        symbols_by_index.push(symbol);
        rest = tail;
    }

    let mut output = Vec::new();
    let mut corpus_bytes = compressed.iter();
    while let Some(&sym_idx) = corpus_bytes.next() {
        if sym_idx == ESCAPE_MARKER {
            let &escaped = corpus_bytes
                .next()
                .ok_or(MltError::MalformedFsst("corpus ends on an escape marker"))?;
            output.push(escaped);
        } else {
            let Some(symbol) = symbols_by_index.get(usize::from(sym_idx)) else {
                return Err(MltError::MalformedFsst(
                    "corpus references a symbol the symbol table does not have",
                ));
            };
            output.extend_from_slice(symbol);
        }
    }

    dec.consume_items::<u8>(output.len())?;
    Ok((String::from_utf8(output)?, lengths.decode_ints::<u32>(dec)?))
}

/// Raw output from FSST compression (unencoded byte buffers).
///
/// Pass to the string encoder's `write_fsst_data` helper to write these
/// streams directly to an [`Encoder`](crate::encoder::Encoder).
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
/// 4. Compressed corpus (`Data(dict_type)` - supplied by the caller at write time)
///
/// Note: The FSST algorithm implementation may differ from Java's, so the
/// compressed output may not be byte-for-byte identical. Both implementations
/// are semantically compatible and can decode each other's output.
pub fn compress_fsst<S: AsRef<str>>(values: &[S]) -> FsstRawData {
    let byte_slices: Vec<&[u8]> = values.iter().map(|s| s.as_ref().as_bytes()).collect();
    let compressor = fsst::Compressor::train(&byte_slices);
    compress_fsst_with(values, &compressor)
}

/// Like [`compress_fsst`] but reuses an already-trained [`fsst::Compressor`].
pub fn compress_fsst_with<S: AsRef<str>>(
    values: &[S],
    compressor: &fsst::Compressor,
) -> FsstRawData {
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

    // Compress all strings as one concatenated buffer.
    // This allows FSST symbol matches across string boundaries.
    // For example: `"sdfAAAA" + "AAAAyxc"` may now compress more `A`s.
    // The decoder decompresses the full corpus and splits by original (uncompressed) value lengths.
    let concatenated: Vec<u8> = values
        .iter()
        .flat_map(|s| s.as_ref().as_bytes())
        .copied()
        .collect();
    let corpus = compressor.compress(&concatenated);

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
    use crate::MltError;
    use crate::decoder::stream::header01;
    use crate::decoder::{DictionaryType, LengthType, RawFsstData, StreamType, ValueKind};
    use crate::encoder::model::StreamCtx;
    use crate::encoder::{
        Codecs, EncodedStream, Encoder, EncoderConfig, ExplicitEncoder, IntEncoder,
    };
    use crate::test_helpers::{assert_empty, dec, parser};
    use crate::utils::BinarySerializer as _;

    /// The 4 FSST streams as wire bytes, ready to be parsed back.
    fn wire_streams(
        symbol_lengths: &[u32],
        symbol_bytes: &[u8],
        value_lengths: &[u32],
        corpus: &[u8],
    ) -> [Vec<u8>; 4] {
        use crate::decoder::StreamMeta;

        let int_stream = |values: &[u32], ty: StreamType, name: &'static str| {
            let mut enc = Encoder::with_explicit(
                EncoderConfig::default(),
                ExplicitEncoder::all(IntEncoder::varint()),
            );
            let mut codecs = Codecs::default();
            let ctx = StreamCtx::prop(ty, name);
            codecs.write_int_stream(values, &ctx, &mut enc).unwrap();
            enc.data().to_vec()
        };
        let byte_stream = |data: &[u8], ty: StreamType, num_values: usize| {
            let stream = EncodedStream {
                meta: StreamMeta::new_none(ty, ValueKind::Int, num_values).unwrap(),
                data: data.to_vec(),
            };
            let mut buf = Vec::new();
            buf.write_stream(&stream).expect("write_stream failed");
            buf
        };

        [
            int_stream(
                symbol_lengths,
                StreamType::Length(LengthType::Symbol),
                "symbol",
            ),
            byte_stream(
                symbol_bytes,
                StreamType::Data(DictionaryType::Fsst),
                symbol_lengths.len(),
            ),
            int_stream(
                value_lengths,
                StreamType::Length(LengthType::Dictionary),
                "dictionary",
            ),
            byte_stream(
                corpus,
                StreamType::Data(DictionaryType::Single),
                value_lengths.len(),
            ),
        ]
    }

    /// Parse the wire buffers from [`wire_streams`] back into decodable streams.
    fn parse_streams(buffers: &[Vec<u8>; 4]) -> RawFsstData<'_> {
        let mut raw_streams = Vec::new();
        for buf in buffers {
            raw_streams.push(assert_empty(header01::parse_stream(
                buf,
                ValueKind::Int,
                &mut parser(),
            )));
        }
        let [s0, s1, s2, s3] = raw_streams.try_into().expect("expected 4 streams");
        RawFsstData::new(s0, s1, s2, s3).expect("RawFsstData::new failed")
    }

    /// Compress `values`, write them to the wire and decode them back.
    fn roundtrip(values: &[&str]) -> (String, Vec<u32>) {
        let raw = compress_fsst(values);
        let buffers = wire_streams(
            &raw.symbol_lengths,
            &raw.symbol_bytes,
            &raw.value_lengths,
            &raw.corpus,
        );
        decode_fsst(parse_streams(&buffers), &mut dec()).expect("decode_fsst failed")
    }

    /// Decode hand-built streams that no encoder would produce.
    fn decode_malformed(
        symbol_lengths: &[u32],
        symbol_bytes: &[u8],
        value_lengths: &[u32],
        corpus: &[u8],
    ) -> MltError {
        let buffers = wire_streams(symbol_lengths, symbol_bytes, value_lengths, corpus);
        decode_fsst(parse_streams(&buffers), &mut dec())
            .expect_err("expected malformed FSST data to be rejected")
    }

    #[test]
    fn test_fsst_roundtrip_empty() {
        let (corpus, lengths) = roundtrip(&[]);
        assert_eq!(corpus, "");
        assert_eq!(lengths, [] as [u32; 0]);
    }

    #[rstest]
    #[case::longer(&["hello world", "hello rust", "hello fsst", "world"])]
    #[case::short(&["hello"])]
    fn automatic_optimization_roundtrip(#[case] values: &[&str]) {
        let (corpus, lengths) = roundtrip(values);
        let mut offset = 0;
        for (s, &len) in values.iter().zip(&lengths) {
            let len = len.into_usize();
            assert_eq!(&corpus[offset..offset + len], *s);
            offset += len;
        }
    }

    #[rstest]
    #[case::only_byte(&[0xFF])]
    #[case::after_a_symbol(&[0x00, 0xFF])]
    fn corpus_ending_on_an_escape_marker(#[case] corpus: &[u8]) {
        let err = decode_malformed(&[2], b"ab", &[1], corpus);
        assert!(matches!(err, MltError::MalformedFsst(_)), "{err:?}");
    }

    #[test]
    fn symbol_longer_than_the_symbol_table() {
        let err = decode_malformed(&[9], b"ab", &[1], &[0x00]);
        assert!(matches!(err, MltError::MalformedFsst(_)), "{err:?}");
    }

    #[test]
    fn later_symbol_reaching_past_the_symbol_table() {
        let err = decode_malformed(&[2, 2], b"abc", &[1], &[0x01]);
        assert!(matches!(err, MltError::MalformedFsst(_)), "{err:?}");
    }

    #[test]
    fn symbol_lengths_too_large_to_sum() {
        let err = decode_malformed(&[u32::MAX, u32::MAX, 1], b"ab", &[1], &[0x00]);
        assert!(matches!(err, MltError::MalformedFsst(_)), "{err:?}");
    }

    #[test]
    fn symbol_index_with_no_symbol_behind_it() {
        let err = decode_malformed(&[2], b"ab", &[1], &[0x07]);
        assert!(matches!(err, MltError::MalformedFsst(_)), "{err:?}");
    }

    #[test]
    fn escaped_byte_survives_a_valid_corpus() {
        let buffers = wire_streams(&[2], b"ab", &[3], &[0x00, 0xFF, 0x7A]);
        let (corpus, lengths) = decode_fsst(parse_streams(&buffers), &mut dec())
            .expect("valid FSST data should decode");
        assert_eq!(corpus, "abz");
        assert_eq!(lengths, [3]);
    }
}
