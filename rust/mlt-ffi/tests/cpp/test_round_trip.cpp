// Round-trip test: MVT -> MLT -> MVT via the C++ diplomat bindings.
#include "../../generated/cpp/MltBuffer.hpp"
#include "../../generated/cpp/MltConverter.hpp"
#include "../../generated/cpp/MltEncoderOptions.hpp"
#include <cassert>
#include <cstdint>
#include <cstdio>

// test/fixtures/simple/point-boolean.mvt (35 bytes)
static const uint8_t MVT_FIXTURE[] = {0x1a, 0x21, 0x0a, 0x05, 0x6c, 0x61, 0x79, 0x65, 0x72, 0x12, 0x0d, 0x08,
                                      0x01, 0x12, 0x02, 0x00, 0x00, 0x18, 0x01, 0x22, 0x03, 0x09, 0x32, 0x22,
                                      0x1a, 0x03, 0x6b, 0x65, 0x79, 0x22, 0x02, 0x38, 0x01, 0x78, 0x02};

int main() {
    // 1. Create default encoder options
    auto opts = MltEncoderOptions::new_();
    assert(opts);

    // 2. MVT -> MLT
    diplomat::span<const uint8_t> mvt_span(MVT_FIXTURE, sizeof(MVT_FIXTURE));
    auto mlt_res = MltConverter::mvt_to_mlt(mvt_span, *opts);
    assert(mlt_res.is_ok());
    auto mlt_buf = std::move(mlt_res).ok().value();
    assert(mlt_buf->len() > 0);

    // 3. MLT -> MVT
    auto mlt_bytes = mlt_buf->as_bytes();
    auto mvt_res = MltConverter::mlt_to_mvt(mlt_bytes);
    assert(mvt_res.is_ok());
    auto mvt_buf = std::move(mvt_res).ok().value();
    assert(mvt_buf->len() > 0);

    // 4. Verify round-trip produced valid MVT
    auto mvt_out = mvt_buf->as_bytes();
    assert(mvt_out.size() > 0);
    assert(mvt_out.data()[0] == 0x1a);

    printf("C++ round-trip test passed\n");
    return 0;
}
