#include <gtest/gtest.h>

#include <libfsst.hpp>

#include <string>
#include <vector>
#include "mlt/decode/string.hpp"

TEST(FSST, RoundTrip) {
    using namespace libfsst;

    const std::string expected = "AAAAAAABBBAAACCdddddEEEEEEfffEEEEAAAAAddddCC";
    const std::size_t expectedCount = 1;
    const auto* expectedPtr = reinterpret_cast<const std::uint8_t*>(expected.data());
    const std::size_t expectedLen = expected.size();

    fsst_encoder_t* encoder = fsst_create(1, &expectedLen, &expectedPtr, /*zeroTerminated=*/0);

    std::vector<std::uint8_t> compressed(1024);
    size_t lenOut = compressed.size();
    std::vector<std::uint8_t*> strOut(expectedCount);
    auto result = compressAuto(
        reinterpret_cast<Encoder*>(encoder),
        expectedCount,
        const_cast<std::size_t*>((&expectedLen)),   // declaration doesn't match definition
        const_cast<std::uint8_t**>((&expectedPtr)), // See https://github.com/springmeyer/fsst/pull/1
        compressed.size(),
        compressed.data(),
        &lenOut,
        strOut.data(),
        /*Use SIMD if available=*/1);
    fsst_destroy(encoder);
    EXPECT_EQ(result, expectedCount);

    const auto decoder = fsst_decoder(encoder);

    std::vector<std::uint8_t> out(10 * expected.size());
    result = fsst_decompress(&decoder, compressed.size(), compressed.data(), out.size(), out.data());

    EXPECT_GE(result, expected.size());
    EXPECT_EQ(0, memcmp(expected.c_str(), out.data(), expected.size()));
}

TEST(FSST, DecodeFromJava) {
    using namespace libfsst;

    const std::string expected = "AAAAAAABBBAAACCdddddEEEEEEfffEEEEAAAAAddddCC";

    const fsst_decoder_t decoder = {
        .version = 0, // not used
        .zeroTerminated = 0,
        .len = {2, 2, 2, 1, 1, 1, 1, 1, 1},
        .symbol = {65, 65, 69, 69, 100, 100, 65, 66, 67, 69, 100, 102},
    };
    const std::vector<std::uint8_t> javaCompressed = {0, 0, 0, 3, 4, 4, 4, 0, 3, 5, 5, 2, 2, 7, 1,
                                                      1, 1, 8, 8, 8, 1, 1, 0, 0, 3, 2, 2, 5, 5};

    std::vector<std::uint8_t> out(10 * expected.size());
    const auto result = fsst_decompress(&decoder, javaCompressed.size(), javaCompressed.data(), out.size(), out.data());
    out.resize(result);

    EXPECT_EQ(result, expected.size());

    // TODO: Why doesn't this work?
    // EXPECT_EQ(0, memcmp(expected.c_str(), out.data(), expected.size()));
}

TEST(FSST, DecodeFromJava2) {
    const std::string expected = "AAAAAAABBBAAACCdddddEEEEEEfffEEEEAAAAAddddCC";
    const std::vector<std::uint8_t> symbols = {65, 65, 69, 69, 100, 100, 65, 66, 67, 69, 100, 102};
    const std::vector<std::uint32_t> symbolLengths = {2, 2, 2, 1, 1, 1, 1, 1, 1};
    const std::vector<std::uint8_t> javaCompressed = {0, 0, 0, 3, 4, 4, 4, 0, 3, 5, 5, 2, 2, 7, 1,
                                                      1, 1, 8, 8, 8, 1, 1, 0, 0, 3, 2, 2, 5, 5};

    const auto decoded = mlt::decoder::StringDecoder::decodeFSST(
        symbols, symbolLengths, javaCompressed, expected.size());

    EXPECT_EQ(decoded.size(), expected.size());
    EXPECT_EQ(0, memcmp(expected.c_str(), decoded.data(), expected.size()));

    // also make sure buffer growth works
    const auto decoded2 = mlt::decoder::StringDecoder::decodeFSST(symbols, symbolLengths, javaCompressed, 0);
    EXPECT_EQ(decoded2.size(), expected.size());
    EXPECT_EQ(0, memcmp(expected.c_str(), decoded2.data(), expected.size()));

    const auto decoded3 = mlt::decoder::StringDecoder::decodeFSST(
        symbols, symbolLengths, javaCompressed, expected.size() / 2);
    EXPECT_EQ(decoded3.size(), expected.size());
    EXPECT_EQ(0, memcmp(expected.c_str(), decoded3.data(), expected.size() / 2));
}
