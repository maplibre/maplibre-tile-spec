#![cfg(test)]

use proptest::prelude::*;
use rstest::rstest;

use crate::utils::BinarySerializer as _;
use crate::v01::stream::encoder::IntEncoder;
use crate::v01::stream::logical::LogicalEncoder;
use crate::v01::{
    DictionaryType, EncodedStream, EncodedStreamData, IntEncoding, LengthType, LogicalData,
    LogicalEncoding, LogicalValue, MortonMeta, OffsetType, PhysicalEncoder, PhysicalEncoding,
    RawFsstData, RawPlainData, RawPresence, RawStream, RawStreamData, RawStrings,
    RawStringsEncoding, RleMeta, StagedStrings, StreamMeta, StreamType,
};
use crate::{Decoder, MemBudget};

/// Test case for stream decoding tests
#[derive(Debug)]
struct StreamTestCase {
    meta: StreamMeta,
    data: &'static [u8],
    expected_u32_logical_value: Option<LogicalValue>,
}

fn mem() -> MemBudget {
    MemBudget::default()
}

fn dec() -> Decoder {
    Decoder::default()
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
            expected_u32_logical_value: Some(LogicalValue::new(
                StreamMeta::new(
                    StreamType::Data(DictionaryType::None),
                    IntEncoding::new(LogicalEncoding::None, PhysicalEncoding::VarInt),
                    4,
                ),
                LogicalData::VecU32(vec![4, 3, 2, 1]),
            )),
        },
        // Basic Encoded test case
        StreamTestCase {
            meta: StreamMeta::new(
                StreamType::Data(DictionaryType::None),
                IntEncoding::none(),
                1,
            ),
            data: &[0x04, 0x03, 0x02, 0x01],
            expected_u32_logical_value: Some(LogicalValue::new(
                StreamMeta::new(
                    StreamType::Data(DictionaryType::None),
                    IntEncoding::none(),
                    1,
                ),
                LogicalData::VecU32(vec![0x0102_0304]),
            )),
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
        if let Some(expected_u32_logical_value) = &test_case.expected_u32_logical_value {
            let stream = create_stream_from_test_case(&test_case);
            let result = stream.decode_bits_u32(&mut Decoder::default());
            assert!(result.is_ok(), "Should successfully decode LogicalValue");
            let logical_value = result.unwrap();
            assert_eq!(
                logical_value, *expected_u32_logical_value,
                "Should produce LogicalValue correctly"
            );
        }
    }
}

fn make_logical_val(logical_encoding: LogicalEncoding, input_data: Vec<u32>) -> LogicalValue {
    let meta = StreamMeta::new(
        StreamType::Data(DictionaryType::None),
        IntEncoding::new(logical_encoding, PhysicalEncoding::VarInt),
        u32::try_from(input_data.len()).expect("input_data length fits in u32"),
    );
    let data = LogicalData::VecU32(input_data);
    LogicalValue::new(meta, data)
}

#[rstest]
// ZigZag pairs: [(0,0),(2,4),(2,4)] -> [(0,0),(1,2),(1,2)]
// Delta: [(0,0),(1,2),(1,2)] -> [(0,0),(1,2),(2,4)]
#[case::componentwise_delta(LogicalEncoding::ComponentwiseDelta, vec![0, 0, 2, 4, 2, 4], vec![0, 0, 1, 2, 2, 4])]
// ZigZag: [0,1,2,1,2] -> [0,-1,1,-1,1]
// Delta: [0,-1,1,-1,1] -> [0,-1,0,-1,0]
#[case::delta(LogicalEncoding::Delta, vec![0, 1, 2, 1, 2], vec![0, -1, 0, -1, 0])]
// RLE: [3,2] [0,2] -> [0,0,0,2,2]
// ZigZag: [0,0,0,2,2] -> [0,0,0,1,1]
// Delta: [0,0,0,1,1] -> [0,0,0,1,2]
#[case::delta_rle(LogicalEncoding::DeltaRle(RleMeta { runs: 2, num_rle_values: 5 }), vec![3, 2, 0, 2], vec![0, 0, 0, 1, 2])]
#[case::delta(LogicalEncoding::Delta, vec![], vec![])]
fn test_decode_i32(
    #[case] logical_encoding: LogicalEncoding,
    #[case] input_data: Vec<u32>,
    #[case] expected: Vec<i32>,
) {
    let result = make_logical_val(logical_encoding, input_data).decode_i32(&mut Decoder::default());
    assert!(result.is_ok(), "should decode successfully");
    assert_eq!(result.unwrap(), expected, "should match expected output");
}

#[rstest]
#[case::empty(LogicalEncoding::None, vec![], vec![])]
#[case::new_encoded(LogicalEncoding::None, vec![10, 20, 30, 40], vec![10, 20, 30, 40])]
#[case::rle(LogicalEncoding::Rle(RleMeta { runs: 3, num_rle_values: 6 }), vec![3, 2, 1, 10, 20, 30], vec![10, 10, 10, 20, 20, 30])]
// ZigZag: [0,2,2,2,2] -> [0,1,1,1,1]
// Delta: [0,1,1,1,1] -> [0,1,2,3,4]
#[case::delta(LogicalEncoding::Delta, vec![0, 2, 2, 2, 2], vec![0, 1, 2, 3, 4])]
fn test_decode_u32(
    #[case] logical_encoding: LogicalEncoding,
    #[case] input_data: Vec<u32>,
    #[case] expected: Vec<u32>,
) {
    let result = make_logical_val(logical_encoding, input_data).decode_u32(&mut Decoder::default());
    assert!(result.is_ok(), "should decode successfully");
    assert_eq!(result.unwrap(), expected, "should match expected output");
}

#[rstest]
#[case::basic(vec![1, 2, 3, 4, 5, 100, 1000])]
#[case::large(vec![1_000_000; 256])]
#[case::edge_values(vec![0, 1, 2, 4, 8, 16, 1024, 65535, 1_000_000_000, u32::MAX])]
#[case::empty(vec![])]
fn test_fastpfor_roundtrip(#[case] values: Vec<u32>) {
    let encoder = IntEncoder::new(LogicalEncoder::None, PhysicalEncoder::FastPFOR);
    let owned_stream = EncodedStream::encode_u32s(&values, encoder).unwrap();

    let mut buffer = Vec::new();
    buffer.write_stream(&owned_stream).unwrap();

    let (remaining, parsed_stream) =
        RawStream::from_bytes(&buffer, &mut MemBudget::default()).unwrap();
    assert!(remaining.is_empty());

    let decoded_values = parsed_stream
        .decode_bits_u32(&mut Decoder::default())
        .unwrap()
        .decode_u32(&mut Decoder::default())
        .unwrap();

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
        PhysicalEncoding::None | PhysicalEncoding::FastPFOR => {
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
    let (remaining, parsed) = if is_bool {
        RawStream::parse_bool(&buffer, &mut MemBudget::default()).unwrap()
    } else {
        RawStream::from_bytes(&buffer, &mut MemBudget::default()).unwrap()
    };

    assert!(remaining.is_empty(), "{} bytes remain", remaining.len());
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
/// Before the fix, `parse_varint_vec` called `Vec::with_capacity(1_073_053_653)` → ~4 GB OOM.
#[test]
fn test_varint_stream_huge_num_values_empty_data() {
    // enc_byte = 0x02 → logical1=0(None), logical2=0(None), physical=2(VarInt)
    // num_values = 0xd5 0xff 0xd5 0xff 0x03 = 1_073_053_653 (valid u32, 5-byte varint)
    // byte_length = 0x00 → 0 bytes of data
    let wire: &[u8] = &[0x00, 0x02, 0xd5, 0xff, 0xd5, 0xff, 0x03, 0x00];
    let (remaining, stream) =
        RawStream::from_bytes(wire, &mut MemBudget::default()).expect("parse must succeed");
    assert!(remaining.is_empty());
    assert_eq!(stream.meta.num_values, 1_073_053_653);
    // Decoding must return an error, not OOM or panic.
    let result = stream.decode_bits_u32(&mut Decoder::default());
    assert!(
        result.is_err(),
        "decode must fail on empty data with huge num_values"
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
    let result = rle.decode::<u32>(&data, &mut Decoder::default());
    assert!(
        result.is_err(),
        "must reject mismatched num_rle_values before allocating"
    );
}

fn encoding_no_fastpfor() -> impl Strategy<Value = IntEncoder> {
    any::<IntEncoder>().prop_filter("not fastpfor", |v| v.physical != PhysicalEncoder::FastPFOR)
}

proptest! {
    #[test]
    fn test_i8_roundtrip(
        values in prop::collection::vec(any::<i8>(), 0..100),
        encoding in any::<IntEncoder>(),
    ) {
        let owned_stream = EncodedStream::encode_i8s(&values, encoding).unwrap();

        let mut buffer = Vec::new();
        buffer.write_stream(&owned_stream).unwrap();

        let (remaining, parsed_stream) = RawStream::from_bytes(&buffer, &mut MemBudget::default()).unwrap();
        assert!(remaining.is_empty());

        let decoded_values = parsed_stream.decode_i8s(&mut Decoder::default()).unwrap();
        assert_eq!(decoded_values, values);
    }

    #[test]
    fn test_u8_roundtrip(
        values in prop::collection::vec(any::<u8>(), 0..100),
        encoding in any::<IntEncoder>()
    ) {
        let owned_stream = EncodedStream::encode_u8s(&values, encoding).unwrap();

        let mut buffer = Vec::new();
        buffer.write_stream(&owned_stream).unwrap();

        let (remaining, parsed_stream) = RawStream::from_bytes(&buffer, &mut MemBudget::default()).unwrap();
        assert!(remaining.is_empty());

        let decoded_values = parsed_stream.decode_u8s(&mut Decoder::default()).unwrap();
        assert_eq!(decoded_values, values);
    }

    #[test]
    fn test_u32_roundtrip(
        values in prop::collection::vec(any::<u32>(), 0..100),
        encoding in any::<IntEncoder>()
    ) {
        let owned_stream = EncodedStream::encode_u32s(&values, encoding).unwrap();

        let mut buffer = Vec::new();
        buffer.write_stream(&owned_stream).unwrap();

        let (remaining, parsed_stream) = RawStream::from_bytes(&buffer, &mut mem()).unwrap();
        assert!(remaining.is_empty());

        let decoded_values = parsed_stream.decode_bits_u32(&mut dec()).unwrap().decode_u32(&mut dec()).unwrap();

        assert_eq!(decoded_values, values);
    }

    #[test]
    fn test_i32_roundtrip(
        values in prop::collection::vec(any::<i32>(), 0..100),
        encoding in any::<IntEncoder>(),
    ) {
        let owned_stream = EncodedStream::encode_i32s(&values, encoding).unwrap();

        let mut buffer = Vec::new();
        buffer.write_stream(&owned_stream).unwrap();

        let (remaining, parsed_stream) = RawStream::from_bytes(&buffer, &mut mem()).unwrap();
        assert!(remaining.is_empty());

        let decoded_values = parsed_stream.decode_bits_u32(&mut dec()).unwrap().decode_i32(&mut dec()).unwrap();

        assert_eq!(decoded_values, values);
    }

    #[test]
    fn test_u64_roundtrip(
        values in prop::collection::vec(any::<u64>(), 0..100),
        encoding in encoding_no_fastpfor()
    ) {
        let owned_stream = EncodedStream::encode_u64s(&values, encoding).unwrap();

        let mut buffer = Vec::new();
        buffer.write_stream(&owned_stream).unwrap();

        let (remaining, parsed_stream) = RawStream::from_bytes(&buffer, &mut mem()).unwrap();
        assert!(remaining.is_empty());

        let decoded_values = parsed_stream.decode_u64s(&mut dec()).unwrap();

        assert_eq!(decoded_values, values);
    }

    #[test]
    fn test_i64_roundtrip(
        values in prop::collection::vec(any::<i64>(), 0..100),
        encoding in encoding_no_fastpfor()
    ) {
        let owned_stream = EncodedStream::encode_i64s(&values, encoding).unwrap();

        let mut buffer = Vec::new();
        buffer.write_stream(&owned_stream).unwrap();

        let (remaining, parsed_stream) = RawStream::from_bytes(&buffer, &mut mem()).unwrap();
        assert!(remaining.is_empty());

        let decoded_values = parsed_stream.decode_i64s(&mut dec()).unwrap();

        assert_eq!(decoded_values, values);
    }

    #[test]
    fn test_f32_roundtrip(values in prop::collection::vec(any::<f32>(), 0..100)) {
        let owned_stream = EncodedStream::encode_f32(&values).unwrap();

        let mut buffer = Vec::new();
        buffer.write_stream(&owned_stream).unwrap();

        let (remaining, parsed_stream) = RawStream::from_bytes(&buffer, &mut mem()).unwrap();
        assert!(remaining.is_empty());

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

        let mut buffer = Vec::new();
        buffer.write_stream(&owned_stream).unwrap();

        let (remaining, parsed_stream) = RawStream::from_bytes(&buffer, &mut mem()).unwrap();
        assert!(remaining.is_empty());

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
        let owned_streams = encoded.streams();

        let mut buffers: Vec<Vec<u8>> = Vec::new();
        for owned_stream in owned_streams {
            let mut buffer = Vec::new();
            buffer.write_stream(owned_stream).unwrap();
            buffers.push(buffer);
        }

        let mut parsed_streams = Vec::new();
        for buffer in &buffers {
            let (remaining, parsed_stream) = RawStream::from_bytes(buffer, &mut mem()).unwrap();
            assert!(remaining.is_empty());
            parsed_streams.push(parsed_stream);
        }

        let strings_encoding = match parsed_streams.len() {
            2 => RawStringsEncoding::plain(RawPlainData::new(parsed_streams[0].clone(), parsed_streams[1].clone()).unwrap()),
            3 => RawStringsEncoding::dictionary(
                RawPlainData::new(parsed_streams[0].clone(), parsed_streams[2].clone()).unwrap(),
                parsed_streams[1].clone(),
            ).unwrap(),
            4 => RawStringsEncoding::fsst_plain(
                RawFsstData::new(
                    parsed_streams[0].clone(),
                    parsed_streams[1].clone(),
                    parsed_streams[2].clone(),
                    parsed_streams[3].clone(),
                ).unwrap()
            ),
            5 => RawStringsEncoding::fsst_dictionary(
                RawFsstData::new(
                    parsed_streams[0].clone(),
                    parsed_streams[1].clone(),
                    parsed_streams[2].clone(),
                    parsed_streams[3].clone()
                ).unwrap(),
                parsed_streams[4].clone(),
            ).unwrap(),
            n => panic!("unexpected stream count {n}"),
        };
        let decoded_values = RawStrings { name: "", presence: RawPresence(None), encoding: strings_encoding }.decode(&mut dec()).unwrap();
        let expected = StagedStrings::from(values.into_iter().map(Some).collect::<Vec<_>>());
        assert_eq!(decoded_values, expected);
    }
}
