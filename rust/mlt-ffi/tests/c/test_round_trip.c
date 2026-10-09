#include "../../generated/c/ConvertError.h"
#include "../../generated/c/ConvertErrorKind.h"
#include "../../generated/c/MltBuffer.h"
#include "../../generated/c/MltConverter.h"
#include "../../generated/c/MltEncoderOptions.h"
#include "../../generated/c/MltWireVersion.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>

static const uint8_t MVT_FIXTURE[] = { 0x1a, 0x21, 0x0a, 0x05, 0x6c, 0x61, 0x79, 0x65, 0x72, 0x12, 0x0d, 0x08,
    0x01, 0x12, 0x02, 0x00, 0x00, 0x18, 0x01, 0x22, 0x03, 0x09, 0x32, 0x22,
    0x1a, 0x03, 0x6b, 0x65, 0x79, 0x22, 0x02, 0x38, 0x01, 0x78, 0x02 };
static const size_t MVT_FIXTURE_LEN = sizeof(MVT_FIXTURE);

static const uint8_t DECODED_POINT_MVT_WITH_EXPLICIT_EXTENT[] = {
    0x1a, 0x24, 0x0a, 0x05, 0x6c, 0x61, 0x79, 0x65, 0x72, 0x12, 0x0d, 0x08,
    0x01, 0x12, 0x02, 0x00, 0x00, 0x18, 0x01, 0x22, 0x03, 0x09, 0x32, 0x22,
    0x1a, 0x03, 0x6b, 0x65, 0x79, 0x22, 0x02, 0x38, 0x01, 0x28, 0x80, 0x20,
    0x78, 0x02
};

static void v2_encoding_of_a_point_layer_decodes_to_the_same_mvt_as_v1(void)
{
    MltEncoderOptions* v1_options = MltEncoderOptions_new();
    MltEncoderOptions* v2_options = MltEncoderOptions_new();
    MltEncoderOptions_set_wire_version(v2_options, MltWireVersion_V02);
    DiplomatU8View mvt = { .data = MVT_FIXTURE, .len = MVT_FIXTURE_LEN };

    MltBuffer* v1_mlt = MltConverter_mvt_to_mlt(mvt, v1_options).ok;
    MltBuffer* v2_mlt = MltConverter_mvt_to_mlt(mvt, v2_options).ok;
    MltBuffer* v1_mvt = MltConverter_mlt_to_mvt(MltBuffer_as_bytes(v1_mlt)).ok;
    MltBuffer* v2_mvt = MltConverter_mlt_to_mvt(MltBuffer_as_bytes(v2_mlt)).ok;

    DiplomatU8View v1_bytes = MltBuffer_as_bytes(v1_mvt);
    DiplomatU8View v2_bytes = MltBuffer_as_bytes(v2_mvt);
    assert(v1_bytes.len == v2_bytes.len);
    assert(memcmp(v1_bytes.data, v2_bytes.data, v1_bytes.len) == 0);

    MltBuffer_destroy(v2_mvt);
    MltBuffer_destroy(v1_mvt);
    MltBuffer_destroy(v2_mlt);
    MltBuffer_destroy(v1_mlt);
    MltEncoderOptions_destroy(v2_options);
    MltEncoderOptions_destroy(v1_options);
}

static void decoding_garbage_as_mlt_reports_invalid_input_with_the_parser_message(void)
{
    const uint8_t garbage[] = { 0xff };
    DiplomatU8View view = { .data = garbage, .len = sizeof(garbage) };

    ConvertError* error = MltConverter_mlt_to_mvt(view).err;

    assert(ConvertError_kind(error) == ConvertErrorKind_InvalidInput);
    char message[128] = { 0 };
    DiplomatWrite write = diplomat_simple_write(message, sizeof(message) - 1);
    ConvertError_message(error, &write);
    assert(strcmp(message, "buffer underflow: needed 2 bytes, but only 1 remain") == 0);
    ConvertError_destroy(error);
}

static void encoding_garbage_as_mvt_reports_encoding_failed_with_the_parser_message(void)
{
    const uint8_t garbage[] = { 0xff };
    DiplomatU8View view = { .data = garbage, .len = sizeof(garbage) };
    MltEncoderOptions* options = MltEncoderOptions_new();

    ConvertError* error = MltConverter_mvt_to_mlt(view, options).err;

    assert(ConvertError_kind(error) == ConvertErrorKind_EncodingFailed);
    char message[128] = { 0 };
    DiplomatWrite write = diplomat_simple_write(message, sizeof(message) - 1);
    ConvertError_message(error, &write);
    assert(strcmp(message, "MVT error: protobuf decode error: unexpected end of buffer") == 0);
    ConvertError_destroy(error);
    MltEncoderOptions_destroy(options);
}

static void v1_encoding_of_a_point_layer_decodes_to_the_point_mvt_with_an_explicit_extent(void)
{
    MltEncoderOptions* options = MltEncoderOptions_new();
    DiplomatU8View mvt = { .data = MVT_FIXTURE, .len = MVT_FIXTURE_LEN };

    MltBuffer* mlt = MltConverter_mvt_to_mlt(mvt, options).ok;
    MltBuffer* decoded = MltConverter_mlt_to_mvt(MltBuffer_as_bytes(mlt)).ok;

    DiplomatU8View decoded_bytes = MltBuffer_as_bytes(decoded);
    assert(decoded_bytes.len == sizeof(DECODED_POINT_MVT_WITH_EXPLICIT_EXTENT));
    assert(memcmp(decoded_bytes.data, DECODED_POINT_MVT_WITH_EXPLICIT_EXTENT, decoded_bytes.len) == 0);

    MltBuffer_destroy(decoded);
    MltBuffer_destroy(mlt);
    MltEncoderOptions_destroy(options);
}

int main(void)
{
    v1_encoding_of_a_point_layer_decodes_to_the_point_mvt_with_an_explicit_extent();
    v2_encoding_of_a_point_layer_decodes_to_the_same_mvt_as_v1();
    decoding_garbage_as_mlt_reports_invalid_input_with_the_parser_message();
    encoding_garbage_as_mvt_reports_encoding_failed_with_the_parser_message();

    printf("C round-trip test passed\n");
    return 0;
}
