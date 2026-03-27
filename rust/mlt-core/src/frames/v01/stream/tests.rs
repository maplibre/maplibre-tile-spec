#![cfg(test)]

use proptest::prelude::*;
use rstest::rstest;

use crate::test_helpers::{assert_empty, dec, parser, roundtrip_stream, roundtrip_stream_u32s};
use crate::utils::BinarySerializer as _;
use crate::v01::stream::encoder::IntEncoder;
use crate::v01::{
    DictionaryType, EncodedStream, EncodedStreamData, EncodedStringsEncoding, IntEncoding,
    LengthType, LogicalEncoding, LogicalValue, MortonMeta, OffsetType, PhysicalEncoder,
    PhysicalEncoding, RawFsstData, RawPlainData, RawPresence, RawStream, RawStreamData, RawStrings,
    RawStringsEncoding, RleMeta, StagedStrings, StreamMeta, StreamType,
};

/// Test case for stream decoding tests
#[derive(Debug)]
struct StreamTestCase {
    meta: StreamMeta,
    data: &'static [u8],
    /// Expected contents of the physical decode buffer after `decode_bits_u32`.
    expected_u32_logical_value: Option<Vec<u32>>,
}

/// Generator function that creates a set of test cases for stream decoding
fn generate_stream_test_cases() -> Vec<StreamTestCase> {
    vec![
        // Basic VarInt test case
        StreamTestCase {
            meta: StreamMeta::new(
                StreamType::Data(DictionaryType::None),
                IntEncoding::new(LogicalEncoding::None, PhysicalEncoding::VarInt),
                4,
            ),
            data: &[0x04, 0x03, 0x02, 0x01],
            expected_u32_logical_value: Some(vec![4, 3, 2, 1]),
        },
        // Basic Encoded test case
        StreamTestCase {
            meta: StreamMeta::new(
                StreamType::Data(DictionaryType::None),
                IntEncoding::none(),
                1,
            ),
            data: &[0x04, 0x03, 0x02, 0x01],
            expected_u32_logical_value: Some(vec![0x0102_0304]),
        },
    ]
}

fn create_stream_from_test_case(test_case: &StreamTestCase) -> RawStream<'_> {
    let data = match test_case.meta.encoding.physical {
        PhysicalEncoding::VarInt => RawStreamData::VarInt(test_case.data),
        PhysicalEncoding::None => RawStreamData::Encoded(test_case.data),
        _ => panic!(
            "Unsupported physical encoding in test: {:?}",
            test_case.meta.encoding.physical
        ),
    };
    RawStream::new(test_case.meta, data)
}

#[test]
fn test_decode_bits_u32() {
    let test_cases = generate_stream_test_cases();

    for test_case in test_cases {
        if let Some(expected_buf) = &test_case.expected_u32_logical_value {
            let stream = create_stream_from_test_case(&test_case);
            let mut buf = Vec::new();
            stream
                .decode_bits_u32(&mut buf, &mut dec())
                .expect("Should successfully decode u32 values");
            assert_eq!(
                &buf, expected_buf,
                "Should produce decoded u32 values correctly"
            );
        }
    }
}

fn make_logical_val(logical_encoding: LogicalEncoding, num_values: usize) -> LogicalValue {
    let meta = StreamMeta::new(
        StreamType::Data(DictionaryType::None),
        IntEncoding::new(logical_encoding, PhysicalEncoding::VarInt),
        u32::try_from(num_values).expect("input_data length fits in u32"),
    );
    LogicalValue::new(meta)
}

#[rstest]
// ZigZag pairs: [(0,0),(2,4),(2,4)] -> [(0,0),(1,2),(1,2)]
// Delta: [(0,0),(1,2),(1,2)] -> [(0,0),(1,2),(2,4)]
#[case::componentwise_delta(LogicalEncoding::ComponentwiseDelta, vec![0u32, 0, 2, 4, 2, 4], vec![0i32, 0, 1, 2, 2, 4])]
// ZigZag: [0,1,2,1,2] -> [0,-1,1,-1,1]
// Delta: [0,-1,1,-1,1] -> [0,-1,0,-1,0]
#[case::delta(LogicalEncoding::Delta, vec![0u32, 1, 2, 1, 2], vec![0i32, -1, 0, -1, 0])]
// RLE: [3,2] [0,2] -> [0,0,0,2,2]
// ZigZag: [0,0,0,2,2] -> [0,0,0,1,1]
// Delta: [0,0,0,1,1] -> [0,0,0,1,2]
#[case::delta_rle(LogicalEncoding::DeltaRle(RleMeta { runs: 2, num_rle_values: 5 }), vec![3u32, 2, 0, 2], vec![0i32, 0, 0, 1, 2])]
#[case::delta_empty(LogicalEncoding::Delta, vec![], vec![])]
fn test_decode_i32(
    #[case] logical_encoding: LogicalEncoding,
    #[case] input_data: Vec<u32>,
    #[case] expected: Vec<i32>,
) {
    let result =
        make_logical_val(logical_encoding, input_data.len()).decode_i32(&input_data, &mut dec());
    assert!(result.is_ok(), "should decode successfully");
    assert_eq!(result.unwrap(), expected, "should match expected output");
}

#[rstest]
#[case::empty(LogicalEncoding::None, vec![], vec![])]
#[case::new_encoded(LogicalEncoding::None, vec![10u32, 20, 30, 40], vec![10u32, 20, 30, 40])]
#[case::rle(LogicalEncoding::Rle(RleMeta { runs: 3, num_rle_values: 6 }), vec![3u32, 2, 1, 10, 20, 30], vec![10u32, 10, 10, 20, 20, 30])]
// ZigZag: [0,2,2,2,2] -> [0,1,1,1,1]
// Delta: [0,1,1,1,1] -> [0,1,2,3,4]
#[case::delta(LogicalEncoding::Delta, vec![0u32, 2, 2, 2, 2], vec![0u32, 1, 2, 3, 4])]
fn test_decode_u32(
    #[case] logical_encoding: LogicalEncoding,
    #[case] input_data: Vec<u32>,
    #[case] expected: Vec<u32>,
) {
    let result =
        make_logical_val(logical_encoding, input_data.len()).decode_u32(&input_data, &mut dec());
    assert!(result.is_ok(), "should decode successfully");
    assert_eq!(result.unwrap(), expected, "should match expected output");
}

#[rstest]
#[case::basic(vec![1, 2, 3, 4, 5, 100, 1000])]
#[case::large(vec![1_000_000; 256])]
#[case::edge_values(vec![0, 1, 2, 4, 8, 16, 1024, 65535, 1_000_000_000, u32::MAX])]
#[case::empty(vec![])]
fn test_fastpfor_roundtrip(#[case] values: Vec<u32>) {
    let encoder = IntEncoder::fastpfor();
    let stream = EncodedStream::encode_u32s(&values, encoder).unwrap();
    let decoded_values = roundtrip_stream_u32s(&stream);
    assert_eq!(decoded_values, values);
}

/// Test roundtrip: write -> parse -> equality for stream serialization
#[rstest]
#[case::new_encoded(StreamType::Data(DictionaryType::None), 2, LogicalEncoding::None, PhysicalEncoding::None, vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08], false)]
#[case::new_encoded(StreamType::Data(DictionaryType::None), 2, LogicalEncoding::ComponentwiseDelta, PhysicalEncoding::None, vec![0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00], false)]
#[case::new_encoded(StreamType::Offset(OffsetType::Vertex), 3, LogicalEncoding::None, PhysicalEncoding::None, vec![0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00], false)]
#[case::varint(StreamType::Data(DictionaryType::None), 4, LogicalEncoding::None, PhysicalEncoding::VarInt, vec![0x0A, 0x14, 0x1E, 0x28], false)]
#[case::varint(StreamType::Data(DictionaryType::None), 5, LogicalEncoding::Delta, PhysicalEncoding::VarInt, vec![0x00, 0x02, 0x02, 0x02, 0x02], false)]
#[case::varint(StreamType::Data(DictionaryType::None), 3, LogicalEncoding::PseudoDecimal, PhysicalEncoding::VarInt, vec![0x01, 0x02, 0x03], false)]
#[case::varint(StreamType::Length(LengthType::VarBinary), 3, LogicalEncoding::Delta, PhysicalEncoding::VarInt, vec![0x00, 0x02, 0x02], false)]
#[case::rle(StreamType::Data(DictionaryType::None), 6, LogicalEncoding::Rle(RleMeta { runs: 3, num_rle_values: 6 }), PhysicalEncoding::VarInt, vec![0x03, 0x02, 0x01, 0x0A, 0x14, 0x1E], false)]
#[case::rle(StreamType::Data(DictionaryType::None), 5, LogicalEncoding::DeltaRle(RleMeta { runs: 2, num_rle_values: 5 }), PhysicalEncoding::VarInt, vec![0x03, 0x02, 0x00, 0x02], false)]
#[case::morton(StreamType::Data(DictionaryType::Morton), 4, LogicalEncoding::Morton(MortonMeta { num_bits: 32, coordinate_shift: 0 }), PhysicalEncoding::VarInt, vec![0x01, 0x02, 0x03, 0x04], false)]
#[case::boolean(StreamType::Present, 16, LogicalEncoding::Rle(RleMeta { runs: 2, num_rle_values: 2 }), PhysicalEncoding::VarInt, vec![0xFF, 0x00], true)]
fn test_stream_roundtrip(
    #[case] stream_type: StreamType,
    #[case] num_values: u32,
    #[case] logical_encoding: LogicalEncoding,
    #[case] physical_encoding: PhysicalEncoding,
    #[case] data_bytes: Vec<u8>,
    #[case] is_bool: bool,
) {
    let stream_data = match physical_encoding {
        PhysicalEncoding::None | PhysicalEncoding::FastPFor256 => {
            EncodedStreamData::Encoded(data_bytes)
        }
        PhysicalEncoding::VarInt => EncodedStreamData::VarInt(data_bytes),
        PhysicalEncoding::Alp => panic!("ALP not supported"),
    };
    let stream = EncodedStream {
        meta: StreamMeta::new(
            stream_type,
            IntEncoding::new(logical_encoding, physical_encoding),
            num_values,
        ),
        data: stream_data,
    };

    // Write to buffer
    let mut buffer = Vec::new();
    if is_bool {
        buffer.write_boolean_stream(&stream).unwrap();
    } else {
        buffer.write_stream(&stream).unwrap();
    }

    // Parse back
    let parsed = assert_empty(if is_bool {
        RawStream::parse_bool(&buffer, &mut parser())
    } else {
        RawStream::from_bytes(&buffer, &mut parser())
    });

    assert_eq!(parsed.meta, stream.meta, "metadata mismatch");

    match (&stream.data, &parsed.data) {
        (EncodedStreamData::Encoded(exp), RawStreamData::Encoded(act)) => {
            assert_eq!(exp.as_slice(), *act, "raw data mismatch");
        }
        (EncodedStreamData::VarInt(exp), RawStreamData::VarInt(act)) => {
            assert_eq!(exp.as_slice(), *act, "varint data mismatch");
        }
        _ => panic!("data type mismatch"),
    }
}

/// OOM regression: `VarInt` stream with huge `num_values` but `byte_length=0`.
///
/// `Wire: stream_type=0x00 | enc=0x02(VarInt) | num_values=0xd5_ff_d5_ff_03 | byte_length=0x00`
/// Before the budget fix, `parse_varint_vec` called `Vec::with_capacity(1_073_053_653)` → ~4 GB OOM.
/// Now the memory budget is checked at parse time: `num_values * 8 = ~8 GB > 10 MB limit`.
#[test]
fn test_varint_stream_huge_num_values_empty_data() {
    // enc_byte = 0x02 → logical1=0(None), logical2=0(None), physical=2(VarInt)
    // num_values = 0xd5 0xff 0xd5 0xff 0x03 = 1_073_053_653 (valid u32, 5-byte varint)
    // byte_length = 0x00 → 0 bytes of data
    let wire: &[u8] = &[0x00, 0x02, 0xd5, 0xff, 0xd5, 0xff, 0x03, 0x00];
    // Parsing must fail: budget reserves num_values * 8 ≈ 8 GB which exceeds the 10 MB limit.
    let result = RawStream::from_bytes(wire, &mut parser());
    assert!(
        result.is_err(),
        "parse must fail when num_values * 8 exceeds the memory budget"
    );
}

/// RLE mismatch regression: `num_rle_values` in stream header doesn't equal sum of runs.
///
/// `RleMeta::decode` must return an error instead of allocating based on the
/// header-declared `num_rle_values` when the actual run sum differs.
#[test]
fn test_rle_num_rle_values_mismatch() {
    // runs=1, num_rle_values=u32::MAX (declared), but the single run has value 1.
    // Sum of runs = 1 ≠ u32::MAX → must error before allocating ~16 GB.
    let rle = RleMeta {
        runs: 1,
        num_rle_values: u32::MAX,
    };
    // data = [run_len=1, value=42] (1 run of length 1 with value 42)
    let data = [1u32, 42u32];
    let result = rle.decode::<u32>(&data, &mut dec());
    assert!(
        result.is_err(),
        "must reject mismatched num_rle_values before allocating"
    );
}

fn encoding_no_fastpfor() -> impl Strategy<Value = IntEncoder> {
    any::<IntEncoder>().prop_filter("not fastpfor", |v| v.physical != PhysicalEncoder::FastPFOR)
}

/// Helper to encode strings as dictionary and extract offset indices and lengths.
fn encode_dict_and_get_parts(values: &[&str]) -> (Vec<u32>, Vec<u32>) {
    let encoded =
        EncodedStream::encode_strings_dict(values, IntEncoder::varint(), IntEncoder::varint())
            .unwrap();
    let EncodedStringsEncoding::Dictionary {
        plain_data,
        offsets,
    } = encoded
    else {
        panic!("expected Dictionary encoding");
    };
    let offsets = roundtrip_stream_u32s(&offsets);
    let lengths = roundtrip_stream_u32s(&plain_data.lengths);
    (offsets, lengths)
}

/// Helper to serialize streams to bytes.
fn serialize_streams(streams: Vec<&EncodedStream>) -> Vec<Vec<u8>> {
    streams
        .into_iter()
        .map(|s| {
            let mut buf = Vec::new();
            buf.write_stream(s).unwrap();
            buf
        })
        .collect()
}

/// Reconstruct `RawStringsEncoding` from parsed streams based on stream count.
fn streams_to_encoding<'a>(streams: &[RawStream<'a>]) -> RawStringsEncoding<'a> {
    match streams.len() {
        2 => RawStringsEncoding::plain(
            RawPlainData::new(streams[0].clone(), streams[1].clone()).unwrap(),
        ),
        3 => RawStringsEncoding::dictionary(
            RawPlainData::new(streams[0].clone(), streams[2].clone()).unwrap(),
            streams[1].clone(),
        )
        .unwrap(),
        4 => RawStringsEncoding::fsst_plain(
            RawFsstData::new(
                streams[0].clone(),
                streams[1].clone(),
                streams[2].clone(),
                streams[3].clone(),
            )
            .unwrap(),
        ),
        5 => RawStringsEncoding::fsst_dictionary(
            RawFsstData::new(
                streams[0].clone(),
                streams[1].clone(),
                streams[2].clone(),
                streams[3].clone(),
            )
            .unwrap(),
            streams[4].clone(),
        )
        .unwrap(),
        n => panic!("unexpected stream count {n}"),
    }
}

#[rstest]
#[case::with_duplicates(&["apple", "banana", "apple", "cherry", "banana", "apple"], &[0, 1, 0, 2, 1, 0], &[5, 6, 6])]
#[case::all_unique(&["a", "b", "c", "d"], &[0, 1, 2, 3], &[1, 1, 1, 1])]
#[case::all_same(&["same", "same", "same", "same"], &[0, 0, 0, 0], &[4])]
fn test_encode_strings_dict(
    #[case] values: &[&str],
    #[case] expected_offsets: &[u32],
    #[case] expected_lengths: &[u32],
) {
    let (offsets, lengths) = encode_dict_and_get_parts(values);
    assert_eq!(offsets, expected_offsets);
    assert_eq!(lengths, expected_lengths);
}

/// Test roundtrip for `encode_strings_dict`.
#[test]
fn test_strings_dict_roundtrip() {
    let values = vec!["hello", "world", "hello", "rust", "world", "hello"];
    let encoded =
        EncodedStream::encode_strings_dict(&values, IntEncoder::varint(), IntEncoder::varint())
            .unwrap();
    let buffers = serialize_streams(encoded.streams());
    let parsed: Vec<_> = buffers
        .iter()
        .map(|buf| assert_empty(RawStream::from_bytes(buf, &mut parser())))
        .collect();
    let encoding = streams_to_encoding(&parsed);
    let decoded = RawStrings {
        name: "",
        presence: RawPresence(None),
        encoding,
    }
    .decode(&mut dec())
    .unwrap();
    let expected = StagedStrings::from(
        values
            .into_iter()
            .map(|s| Some(s.to_string()))
            .collect::<Vec<_>>(),
    );
    assert_eq!(decoded, expected);
}

proptest! {
    #[test]
    fn test_i8_roundtrip(
        values in prop::collection::vec(any::<i8>(), 0..100),
        encoding in any::<IntEncoder>(),
    ) {
        let owned_stream = EncodedStream::encode_i8s(&values, encoding).unwrap();

        let mut buf = Vec::new();
        let parsed_stream = roundtrip_stream(&mut buf, &owned_stream);
        let decoded_values = parsed_stream.decode_i8s(&mut dec()).unwrap();

        assert_eq!(decoded_values, values);
    }

    #[test]
    fn test_u8_roundtrip(
        values in prop::collection::vec(any::<u8>(), 0..100),
        encoding in any::<IntEncoder>()
    ) {
        let owned_stream = EncodedStream::encode_u8s(&values, encoding).unwrap();

        let mut buf = Vec::new();
        let parsed_stream = roundtrip_stream(&mut buf, &owned_stream);
        let decoded_values = parsed_stream.decode_u8s(&mut dec()).unwrap();

        assert_eq!(decoded_values, values);
    }

    #[test]
    fn test_u32_roundtrip(
        values in prop::collection::vec(any::<u32>(), 0..100),
        encoding in any::<IntEncoder>()
    ) {
        let owned_stream = EncodedStream::encode_u32s(&values, encoding).unwrap();
        let decoded_values = roundtrip_stream_u32s(&owned_stream);
        assert_eq!(decoded_values, values);
    }

    #[test]
    fn test_i32_roundtrip(
        values in prop::collection::vec(any::<i32>(), 0..100),
        encoding in any::<IntEncoder>(),
    ) {
        let owned_stream = EncodedStream::encode_i32s(&values, encoding).unwrap();

        let mut buf = Vec::new();
        let parsed_stream = roundtrip_stream(&mut buf, &owned_stream);
        let decoded_values = parsed_stream.decode_i32s(&mut dec()).unwrap();

        assert_eq!(decoded_values, values);
    }

    #[test]
    fn test_u64_roundtrip(
        values in prop::collection::vec(any::<u64>(), 0..100),
        encoding in encoding_no_fastpfor()
    ) {
        let owned_stream = EncodedStream::encode_u64s(&values, encoding).unwrap();

        let mut buf = Vec::new();
        let parsed_stream = roundtrip_stream(&mut buf, &owned_stream);
        let decoded_values = parsed_stream.decode_u64s(&mut dec()).unwrap();

        assert_eq!(decoded_values, values);
    }

    #[test]
    fn test_i64_roundtrip(
        values in prop::collection::vec(any::<i64>(), 0..100),
        encoding in encoding_no_fastpfor()
    ) {
        let owned_stream = EncodedStream::encode_i64s(&values, encoding).unwrap();

        let mut buf = Vec::new();
        let parsed_stream = roundtrip_stream(&mut buf, &owned_stream);
        let decoded_values = parsed_stream.decode_i64s(&mut dec()).unwrap();

        assert_eq!(decoded_values, values);
    }

    #[test]
    fn test_f32_roundtrip(values in prop::collection::vec(any::<f32>(), 0..100)) {
        let owned_stream = EncodedStream::encode_f32(&values).unwrap();

        let mut buf = Vec::new();
        let parsed_stream = roundtrip_stream(&mut buf, &owned_stream);
        let decoded_values = parsed_stream.decode_f32s(&mut dec()).unwrap();

        assert_eq!(decoded_values.len(), values.len());
        for (v1, v2) in decoded_values.iter().zip(values.iter()) {
            assert_eq!(
                v1.to_bits(),
                v2.to_bits(),
                "despite being semantically equal, the values are not actually equal"
            );
        }
    }

    #[test]
    fn test_f64_roundtrip(values in prop::collection::vec(any::<f64>(), 0..100)) {
        let owned_stream = EncodedStream::encode_f64(&values).unwrap();

        let mut buf = Vec::new();
        let parsed_stream = roundtrip_stream(&mut buf, &owned_stream);
        let decoded_values = parsed_stream.decode_f64s(&mut dec()).unwrap();

        assert_eq!(decoded_values.len(), values.len());
        for (v1, v2) in decoded_values.iter().zip(values.iter()) {
            assert_eq!(
                v1.to_bits(),
                v2.to_bits(),
                "despite being semantically equal, the values are not actually equal"
            );
        }
    }

    #[test]
    fn test_string_roundtrip(
        values in prop::collection::vec(any::<String>(), 0..100),
        encoding in any::<IntEncoder>(),
    ) {
        let encoded = EncodedStream::encode_strings_with_type(&values, encoding, LengthType::VarBinary, DictionaryType::None).unwrap();
        let buffers = serialize_streams(encoded.streams());
        let parsed: Vec<_> = buffers.iter().map(|buf| assert_empty(RawStream::from_bytes(buf, &mut parser()))).collect();
        let str_encoding = streams_to_encoding(&parsed);
        let decoded = RawStrings { name: "", presence: RawPresence(None), encoding: str_encoding }.decode(&mut dec()).unwrap();
        let expected = StagedStrings::from(values.into_iter().map(Some).collect::<Vec<_>>());
        assert_eq!(decoded, expected);
    }
}
