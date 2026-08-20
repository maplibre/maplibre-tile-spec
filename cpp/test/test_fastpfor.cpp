#include <gtest/gtest.h>

// From fastpfor/...
#include <fastpfor/compositecodec.h>
#include <fastpfor/fastpfor.h>
#include <fastpfor/variablebyte.h>

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <ranges>
#include <string>
#include <vector>

// Test less than one block of data, which does not use both parts of the CompositeCodec
TEST(FastPfor, DecompressPartial) {
    const std::vector<std::uint32_t> data = {0, 9605520};
    const std::vector<std::uint32_t> expected = {16, 17, 18};

    FastPForLib::CompositeCodec<FastPForLib::FastPFor<8>, FastPForLib::VariableByte> codec;
    const auto result = codec.uncompress(data, expected.size());
    EXPECT_TRUE(std::ranges::equal(result, expected));
}

namespace {
// Return a sequence of 1,2,1,2,... of length n
auto simpleValueView(std::size_t n) {
    return std::views::iota(0ul) | std::views::transform([](auto x) { return (x & 1u) + 1u; }) | std::views::take(n) |
           std::views::common;
}
auto simpleValues(std::size_t n) {
    const auto view = simpleValueView(n);
    return std::vector<uint32_t>(view.begin(), view.end());
}
} // namespace

// Check decode of data encoded by the Java implementation with a payload larger than,
// but not a multiple of, the block size. The result was captured before the final
// byte-swapping, so no additional swapping is needed as with a typical payload.
TEST(FastPfor, JavaV1) {
    const std::uint32_t javaEncodedByteSwapped[] = {
        0x00000300ul, 0x00000031ul, 0x99999999ul, 0x99999999ul, 0x99999999ul, 0x99999999ul, 0x99999999ul, 0x99999999ul,
        0x99999999ul, 0x99999999ul, 0x99999999ul, 0x99999999ul, 0x99999999ul, 0x99999999ul, 0x99999999ul, 0x99999999ul,
        0x99999999ul, 0x99999999ul, 0x99999999ul, 0x99999999ul, 0x99999999ul, 0x99999999ul, 0x99999999ul, 0x99999999ul,
        0x99999999ul, 0x99999999ul, 0x99999999ul, 0x99999999ul, 0x99999999ul, 0x99999999ul, 0x99999999ul, 0x99999999ul,
        0x99999999ul, 0x99999999ul, 0x99999999ul, 0x99999999ul, 0x99999999ul, 0x99999999ul, 0x99999999ul, 0x99999999ul,
        0x99999999ul, 0x99999999ul, 0x99999999ul, 0x99999999ul, 0x99999999ul, 0x99999999ul, 0x99999999ul, 0x99999999ul,
        0x99999999ul, 0x99999999ul, 0x00000006ul, 0x00020002ul, 0x00000002ul, 0x00000000ul, 0x82818281ul, 0x82818281ul,
        0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul,
        0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul,
        0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul,
        0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul,
        0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul,
        0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul,
        0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul, 0x82818281ul,
    };

    std::vector<std::uint32_t> decoded(1000);
    std::size_t outSize = decoded.size();
    FastPForLib::CompositeCodec<FastPForLib::FastPFor<8>, FastPForLib::VariableByte> codec;
    codec.decodeArray(reinterpret_cast<const uint32_t*>(javaEncodedByteSwapped),
                      sizeof(javaEncodedByteSwapped) / sizeof(uint32_t),
                      decoded.data(),
                      outSize);
    EXPECT_EQ(outSize, decoded.size());
    EXPECT_EQ(decoded, simpleValues(decoded.size()));
}
