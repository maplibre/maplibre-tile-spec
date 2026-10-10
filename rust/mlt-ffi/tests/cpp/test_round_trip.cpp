#include "../../generated/cpp/ConvertError.hpp"
#include "../../generated/cpp/ConvertErrorKind.hpp"
#include "../../generated/cpp/MltBuffer.hpp"
#include "../../generated/cpp/MltConverter.hpp"
#include "../../generated/cpp/MltEncoderOptions.hpp"
#include "../../generated/cpp/MltWireVersion.hpp"
#include <cassert>
#include <cstdint>
#include <cstdio>
#include <vector>

static const uint8_t MVT_FIXTURE[] = { 0x1a, 0x21, 0x0a, 0x05, 0x6c, 0x61, 0x79, 0x65, 0x72, 0x12, 0x0d, 0x08,
    0x01, 0x12, 0x02, 0x00, 0x00, 0x18, 0x01, 0x22, 0x03, 0x09, 0x32, 0x22,
    0x1a, 0x03, 0x6b, 0x65, 0x79, 0x22, 0x02, 0x38, 0x01, 0x78, 0x02 };

static const uint8_t DECODED_POINT_MVT_WITH_EXPLICIT_EXTENT[] = {
    0x1a, 0x24, 0x0a, 0x05, 0x6c, 0x61, 0x79, 0x65, 0x72, 0x12, 0x0d, 0x08, 0x01, 0x12, 0x02, 0x00, 0x00, 0x18, 0x01,
    0x22, 0x03, 0x09, 0x32, 0x22, 0x1a, 0x03, 0x6b, 0x65, 0x79, 0x22, 0x02, 0x38, 0x01, 0x28, 0x80, 0x20, 0x78, 0x02
};

static void v2_encoding_of_a_point_layer_decodes_to_the_same_mvt_as_v1()
{
    auto v1_options = MltEncoderOptions::new_();
    auto v2_options = MltEncoderOptions::new_();
    v2_options->set_wire_version(MltWireVersion::V02);
    diplomat::span<const uint8_t> mvt(MVT_FIXTURE, sizeof(MVT_FIXTURE));

    auto v1_mlt = MltConverter::mvt_to_mlt(mvt, *v1_options).ok().value();
    auto v2_mlt = MltConverter::mvt_to_mlt(mvt, *v2_options).ok().value();
    auto v1_mvt = MltConverter::mlt_to_mvt(v1_mlt->as_bytes()).ok().value();
    auto v2_mvt = MltConverter::mlt_to_mvt(v2_mlt->as_bytes()).ok().value();

    auto v1_bytes = v1_mvt->as_bytes();
    auto v2_bytes = v2_mvt->as_bytes();
    assert(std::vector<uint8_t>(v1_bytes.begin(), v1_bytes.end()) == std::vector<uint8_t>(v2_bytes.begin(), v2_bytes.end()));
}

static void decoding_garbage_as_mlt_reports_invalid_input_with_the_parser_message()
{
    const uint8_t garbage[] = { 0xff };

    auto error = MltConverter::mlt_to_mvt(diplomat::span<const uint8_t>(garbage, 1)).err().value();

    assert(error->kind() == ConvertErrorKind::InvalidInput);
    assert(error->message() == "buffer underflow: needed 2 bytes, but only 1 remain");
}

static void encoding_garbage_as_mvt_reports_encoding_failed_with_the_parser_message()
{
    const uint8_t garbage[] = { 0xff };
    auto options = MltEncoderOptions::new_();

    auto error = MltConverter::mvt_to_mlt(diplomat::span<const uint8_t>(garbage, 1), *options).err().value();

    assert(error->kind() == ConvertErrorKind::EncodingFailed);
    assert(error->message() == "MVT error: protobuf decode error: unexpected end of buffer");
}

static void v1_encoding_of_a_point_layer_decodes_to_the_point_mvt_with_an_explicit_extent()
{
    auto options = MltEncoderOptions::new_();
    diplomat::span<const uint8_t> mvt(MVT_FIXTURE, sizeof(MVT_FIXTURE));

    auto mlt = MltConverter::mvt_to_mlt(mvt, *options).ok().value();
    auto decoded = MltConverter::mlt_to_mvt(mlt->as_bytes()).ok().value();

    auto decoded_bytes = decoded->as_bytes();
    assert(std::vector<uint8_t>(decoded_bytes.begin(), decoded_bytes.end()) == std::vector<uint8_t>(std::begin(DECODED_POINT_MVT_WITH_EXPLICIT_EXTENT), std::end(DECODED_POINT_MVT_WITH_EXPLICIT_EXTENT)));
}

int main()
{
    v1_encoding_of_a_point_layer_decodes_to_the_point_mvt_with_an_explicit_extent();
    v2_encoding_of_a_point_layer_decodes_to_the_same_mvt_as_v1();
    decoding_garbage_as_mlt_reports_invalid_input_with_the_parser_message();
    encoding_garbage_as_mvt_reports_encoding_failed_with_the_parser_message();

    printf("C++ round-trip test passed\n");
    return 0;
}
