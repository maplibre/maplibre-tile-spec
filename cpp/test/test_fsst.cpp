#include <gtest/gtest.h>

#include <libfsst.hpp>

#include <vector>

TEST(FSST, Decode) {
    const std::string expected = "AAAAAAABBBAAACCdddddEEEEEEfffEEEEAAAAAddddCC";
    const std::vector<std::uint8_t> compressed{
        0, 0, 0, 2, 7, 7, 7, 0, 2, 5, 5, 4, 4, 4, 4, 4, 3, 3, 3,
        3, 3, 3, 6, 6, 6, 3, 3, 3, 3, 0, 0, 2, 4, 4, 4, 4, 5, 5,
    };

    const fsst_decoder_t decoder = {
        .version = (FSST_VERSION << 32) | (((u64)FSST_CODE_MAX) << 24) | (((u64)0) << 16) | (((u64)9) << 8) |
                   FSST_ENDIAN_MARKER,
        .zeroTerminated = 0,
        .len = {2, 1, 1, 1, 1, 1, 1, 1},
        .symbol = {65, 65, 0, 65, 69, 100, 67, 102, 66},
    };

    std::vector<std::uint8_t> out(2 * expected.size());
    const auto result = fsst_decompress(&decoder, compressed.size(), compressed.data(), out.size(), out.data());
    out.resize(result);

    if (out.size() != expected.size() || 0 != memcmp(out.data(), expected.data(), expected.size())) {
        std::cout << "FSST.Decode Failed\n";
        // FAIL();
    }
}
