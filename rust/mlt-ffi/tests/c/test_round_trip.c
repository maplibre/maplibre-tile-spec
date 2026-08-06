// Round-trip test: MVT -> MLT -> MVT via the C diplomat bindings.
#include "../../generated/c/MltBuffer.h"
#include "../../generated/c/MltConverter.h"
#include "../../generated/c/MltEncoderOptions.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>

// test/fixtures/simple/point-boolean.mvt (35 bytes)
static const uint8_t MVT_FIXTURE[] = { 0x1a, 0x21, 0x0a, 0x05, 0x6c, 0x61, 0x79, 0x65, 0x72, 0x12, 0x0d, 0x08,
    0x01, 0x12, 0x02, 0x00, 0x00, 0x18, 0x01, 0x22, 0x03, 0x09, 0x32, 0x22,
    0x1a, 0x03, 0x6b, 0x65, 0x79, 0x22, 0x02, 0x38, 0x01, 0x78, 0x02 };
static const size_t MVT_FIXTURE_LEN = sizeof(MVT_FIXTURE);

int main(void)
{
    // 1. Create default encoder options
    MltEncoderOptions* opts = MltEncoderOptions_new();
    assert(opts != NULL);

    // 2. MVT -> MLT
    DiplomatU8View mvt_view = { .data = MVT_FIXTURE, .len = MVT_FIXTURE_LEN };
    MltConverter_mvt_to_mlt_result mlt_res = MltConverter_mvt_to_mlt(mvt_view, opts);
    assert(mlt_res.is_ok);
    MltBuffer* mlt_buf = mlt_res.ok;
    assert(MltBuffer_len(mlt_buf) > 0);

    // 3. MLT -> MVT
    DiplomatU8View mlt_view = MltBuffer_as_bytes(mlt_buf);
    MltConverter_mlt_to_mvt_result mvt_res = MltConverter_mlt_to_mvt(mlt_view);
    assert(mvt_res.is_ok);
    MltBuffer* mvt_buf = mvt_res.ok;
    assert(MltBuffer_len(mvt_buf) > 0);

    // 4. Verify round-trip produced valid MVT (non-empty, starts with protobuf tag)
    DiplomatU8View mvt_out = MltBuffer_as_bytes(mvt_buf);
    assert(mvt_out.len > 0);
    assert(mvt_out.data[0] == 0x1a); // protobuf field 3 (layers), wire type 2

    // Cleanup
    MltBuffer_destroy(mvt_buf);
    MltBuffer_destroy(mlt_buf);
    MltEncoderOptions_destroy(opts);

    printf("C round-trip test passed\n");
    return 0;
}
