//! Shared helpers for unit tests, integration tests, and benchmarks.

use crate::encoder::EncodedStream;
use crate::utils::BinarySerializer as _;
use crate::v01::{Layer01, RawStream};
use crate::{Decoder, Layer, MltRefResult, Parser};

/// Default decoder for decoding in tests.
#[must_use]
pub fn dec() -> Decoder {
    Decoder::default()
}

/// Default parser for parsing in tests.
#[must_use]
pub fn parser() -> Parser {
    Parser::default()
}

pub fn assert_empty<T>(result: MltRefResult<T>) -> T {
    let (remaining, value) = result.unwrap();
    assert!(remaining.is_empty(), "{} bytes remain", remaining.len());
    value
}

#[must_use]
pub fn into_layer01(layer: Layer) -> Layer01 {
    match layer {
        Layer::Tag01(layer01) => layer01,
        Layer::Unknown(_) => panic!("expected Tag01 layer"),
    }
}

#[must_use]
pub fn roundtrip_stream<'a>(buffer: &'a mut Vec<u8>, stream: &EncodedStream) -> RawStream<'a> {
    buffer.clear();
    buffer.write_stream(stream).unwrap();
    assert_empty(RawStream::from_bytes(buffer, &mut parser()))
}

#[must_use]
pub fn roundtrip_stream_u32s(stream: &EncodedStream) -> Vec<u32> {
    let mut buf = Vec::new();
    let parsed_stream = roundtrip_stream(&mut buf, stream);

    let mut decoder = dec();
    let values = parsed_stream.decode_u32s(&mut decoder).unwrap();
    if !values.is_empty() {
        assert!(
            decoder.consumed() > 0,
            "decoder should consume bytes after decode"
        );
    }
    values
}
