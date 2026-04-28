use proptest::prelude::*;
use rstest::rstest;

use super::write::{write_i32_stream, write_i64_stream, write_u32_stream, write_u64_stream};
use crate::MltError;
use crate::decoder::{
    DictionaryType, IntEncoding, LengthType, LogicalEncoding, LogicalValue, Morton, OffsetType,
    PhysicalEncoding, RawStream, RleMeta, StreamMeta, StreamType,
};
use crate::encoder::model::StreamCtx;
use crate::encoder::{
    Codecs, EncodedStream, Encoder, ExplicitEncoder, IntEncoder, PhysicalEncoder,
};
use crate::test_helpers::{assert_empty, dec, parser};
use crate::utils::BinarySerializer as _;

const DATA_STREAM: StreamType = StreamType::Data(DictionaryType::None);

fn roundtrip_stream<'a>(buffer: &'a mut Vec<u8>, stream: &EncodedStream) -> RawStream<'a> {
    buffer.clear();
    buffer.write_stream(stream).unwrap();
    assert_empty(RawStream::from_bytes(buffer, &mut parser()))
}

fn roundtrip_stream_u32s(wire: &[u8]) -> Vec<u32> {
    let parsed_stream = assert_empty(RawStream::from_bytes(wire, &mut parser()));

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

fn make_logical_val(logical_encoding: LogicalEncoding, num_values: usize) -> LogicalValue {
    LogicalValue::new(
        StreamMeta::new2(
            StreamType::Data(DictionaryType::None),
            logical_encoding,
            PhysicalEncoding::VarInt,
            num_values,
        )
        .unwrap(),
    )
}

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
    RawStream::new(test_case.meta, test_case.data)
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
    let mut enc = Encoder::with_explicit(
        Encoder::default().cfg,
        ExplicitEncoder::all(IntEncoder::fastpfor()),
    );
    let codecs = &mut Codecs::default();
    let ctx = StreamCtx::prop(DATA_STREAM, "test");
    write_u32_stream(&values, &ctx, &mut enc, codecs).unwrap();
    let decoded_values = roundtrip_stream_u32s(&enc.data);
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
#[case::morton(StreamType::Data(DictionaryType::Morton), 4, LogicalEncoding::Morton(Morton { bits: 16, shift: 0 }), PhysicalEncoding::VarInt, vec![0x01, 0x02, 0x03, 0x04], false)]
#[case::boolean(StreamType::Present, 16, LogicalEncoding::Rle(RleMeta { runs: 2, num_rle_values: 2 }), PhysicalEncoding::VarInt, vec![0xFF, 0x00], true)]
fn test_stream_roundtrip(
    #[case] stream_type: StreamType,
    #[case] num_values: u32,
    #[case] logical_encoding: LogicalEncoding,
    #[case] physical_encoding: PhysicalEncoding,
    #[case] data_bytes: Vec<u8>,
    #[case] is_bool: bool,
) {
    let stream = EncodedStream {
        meta: StreamMeta::new(
            stream_type,
            IntEncoding::new(logical_encoding, physical_encoding),
            num_values,
        ),
        data: data_bytes,
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
    assert_eq!(stream.data.as_slice(), parsed.data, "data mismatch");
}

#[test]
fn test_morton_parse_rejects_too_many_bits() {
    let stream = EncodedStream {
        meta: StreamMeta::new(
            StreamType::Data(DictionaryType::Morton),
            IntEncoding::new(
                LogicalEncoding::Morton(Morton { bits: 17, shift: 0 }),
                PhysicalEncoding::VarInt,
            ),
            1,
        ),
        data: vec![0],
    };
    let mut buffer = Vec::new();
    buffer.write_stream(&stream).unwrap();

    let err = RawStream::from_bytes(&buffer, &mut parser()).unwrap_err();
    assert!(matches!(err, MltError::InvalidMortonBits(17)));
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

/// Deduplicate strings and return (`offset_indices`, `unique_lengths`).
fn dedup_and_get_parts(values: &[&str]) -> (Vec<u32>, Vec<u32>) {
    use crate::encoder::stream::dedup_strings;
    use crate::utils::strings_to_lengths;
    let (unique, offset_indices) = dedup_strings(values).unwrap();
    let lengths = strings_to_lengths(&unique).unwrap();
    (offset_indices, lengths)
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
    let (offsets, lengths) = dedup_and_get_parts(values);
    assert_eq!(offsets, expected_offsets);
    assert_eq!(lengths, expected_lengths);
}

proptest! {
    #[test]
    fn test_i8_roundtrip(
        values in prop::collection::vec(any::<i8>(), 0..100),
        encoding in any::<IntEncoder>(),
    ) {
        let widened: Vec<i32> = values.iter().map(|&v| i32::from(v)).collect();
        let mut enc = Encoder::with_explicit(Encoder::default().cfg, ExplicitEncoder::all(encoding));
        write_i32_stream(&widened, &StreamCtx::prop(DATA_STREAM, "test"), &mut enc, &mut Codecs::default()).unwrap();
        let parsed_stream = assert_empty(RawStream::from_bytes(&enc.data, &mut parser()));
        let decoded_values = parsed_stream.decode_i8s(&mut dec()).unwrap();

        assert_eq!(decoded_values, values);
    }

    #[test]
    fn test_u8_roundtrip(
        values in prop::collection::vec(any::<u8>(), 0..100),
        encoding in any::<IntEncoder>()
    ) {
        let widened: Vec<u32> = values.iter().map(|&v| u32::from(v)).collect();
        let mut enc = Encoder::with_explicit(Encoder::default().cfg, ExplicitEncoder::all(encoding));
        write_u32_stream(&widened, &StreamCtx::prop(DATA_STREAM, "test"), &mut enc, &mut Codecs::default()).unwrap();
        let parsed_stream = assert_empty(RawStream::from_bytes(&enc.data, &mut parser()));
        let decoded_values = parsed_stream.decode_u8s(&mut dec()).unwrap();

        assert_eq!(decoded_values, values);
    }

    #[test]
    fn test_u32_roundtrip(
        values in prop::collection::vec(any::<u32>(), 0..100),
        encoding in any::<IntEncoder>()
    ) {
        let mut enc = Encoder::with_explicit(Encoder::default().cfg, ExplicitEncoder::all(encoding));
        write_u32_stream(&values, &StreamCtx::prop(DATA_STREAM, "test"), &mut enc, &mut Codecs::default()).unwrap();
        let decoded_values = roundtrip_stream_u32s(&enc.data);
        assert_eq!(decoded_values, values);
    }

    #[test]
    fn test_i32_roundtrip(
        values in prop::collection::vec(any::<i32>(), 0..100),
        encoding in any::<IntEncoder>(),
    ) {
        let mut enc = Encoder::with_explicit(Encoder::default().cfg, ExplicitEncoder::all(encoding));
        write_i32_stream(&values, &StreamCtx::prop(DATA_STREAM, "test"), &mut enc, &mut Codecs::default()).unwrap();
        let parsed_stream = assert_empty(RawStream::from_bytes(&enc.data, &mut parser()));
        let decoded_values = parsed_stream.decode_i32s(&mut dec()).unwrap();

        assert_eq!(decoded_values, values);
    }

    #[test]
    fn test_u64_roundtrip(
        values in prop::collection::vec(any::<u64>(), 0..100),
        encoding in encoding_no_fastpfor()
    ) {
        let mut enc = Encoder::with_explicit(Encoder::default().cfg, ExplicitEncoder::all(encoding));
        write_u64_stream(&values, &StreamCtx::prop(DATA_STREAM, "test"), &mut enc, &mut Codecs::default()).unwrap();
        let parsed_stream = assert_empty(RawStream::from_bytes(&enc.data, &mut parser()));
        let decoded_values = parsed_stream.decode_u64s(&mut dec()).unwrap();

        assert_eq!(decoded_values, values);
    }

    #[test]
    fn test_i64_roundtrip(
        values in prop::collection::vec(any::<i64>(), 0..100),
        encoding in encoding_no_fastpfor()
    ) {
        let mut enc = Encoder::with_explicit(Encoder::default().cfg, ExplicitEncoder::all(encoding));
        write_i64_stream(&values, &StreamCtx::prop(DATA_STREAM, "test"), &mut enc, &mut Codecs::default()).unwrap();
        let parsed_stream = assert_empty(RawStream::from_bytes(&enc.data, &mut parser()));
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
}
