#if MLT_WITH_FASTPFOR
#include <gtest/gtest.h>

// From fastpfor/...
#include <compositecodec.h>
#include <fastpfor.h>
#include <variablebyte.h>
#if MLT_WITH_FASTPFOR_SIMD
#include <simdfastpfor.h>
#endif // MLT_WITH_FASTPFOR_SIMD

#include <algorithm>
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
auto simpleValueView(int n) {
    return std::views::iota(0ul) | std::views::transform([](auto n) { return (n & 1) + 1; }) | std::views::take(n) |
           std::ranges::views::common;
}
auto simpleValues(int n) {
    const auto view = simpleValueView(n);
    return std::vector<uint32_t>(view.begin(), view.end());
}
} // namespace

// Check decode of data encoded by the Java implementation with a payload larger than,
// but not a multiple of, the block size. The result was captured before the final
// byte-swapping, so no additional swapping is needed as with a typical payload.
TEST(FastPfor, JavaV1) {
    // clang-format off
    const std::int32_t javaEncodedByteSwapped[] = {
        768,         49,
        -1717986919, -1717986919, -1717986919, -1717986919, -1717986919, -1717986919, -1717986919, -1717986919,
        -1717986919, -1717986919, -1717986919, -1717986919, -1717986919, -1717986919, -1717986919, -1717986919,
        -1717986919, -1717986919, -1717986919, -1717986919, -1717986919, -1717986919, -1717986919, -1717986919,
        -1717986919, -1717986919, -1717986919, -1717986919, -1717986919, -1717986919, -1717986919, -1717986919,
        -1717986919, -1717986919, -1717986919, -1717986919, -1717986919, -1717986919, -1717986919, -1717986919,
        -1717986919, -1717986919, -1717986919, -1717986919, -1717986919, -1717986919, -1717986919, -1717986919,
        6,           131074,      2,           0,
        -2105441663, -2105441663, -2105441663, -2105441663, -2105441663, -2105441663, -2105441663, -2105441663,
        -2105441663, -2105441663, -2105441663, -2105441663, -2105441663, -2105441663, -2105441663, -2105441663,
        -2105441663, -2105441663, -2105441663, -2105441663, -2105441663, -2105441663, -2105441663, -2105441663,
        -2105441663, -2105441663, -2105441663, -2105441663, -2105441663, -2105441663, -2105441663, -2105441663,
        -2105441663, -2105441663, -2105441663, -2105441663, -2105441663, -2105441663, -2105441663, -2105441663,
        -2105441663, -2105441663, -2105441663, -2105441663, -2105441663, -2105441663, -2105441663, -2105441663,
        -2105441663, -2105441663, -2105441663, -2105441663, -2105441663, -2105441663, -2105441663, -2105441663,
        -2105441663, -2105441663
    };
    // clang-format on

    std::vector<std::uint32_t> decoded(1000);
    std::size_t outSize = decoded.size();
    FastPForLib::CompositeCodec<FastPForLib::FastPFor<8>, FastPForLib::VariableByte> codec;
    codec.decodeArray(reinterpret_cast<const uint32_t*>(javaEncodedByteSwapped),
                      sizeof(javaEncodedByteSwapped) / sizeof(uint32_t),
                      decoded.data(),
                      outSize);
    EXPECT_EQ(decoded, simpleValues(1000));
}

#if MLT_WITH_FASTPFOR_SIMD
namespace {
template <int BLOCK_SIZE>
void testSIMDInterop(int valuesCount) {
    FastPForLib::CompositeCodec<FastPForLib::FastPFor<BLOCK_SIZE>, FastPForLib::VariableByte> codec;
    FastPForLib::CompositeCodec<FastPForLib::SIMDFastPFor<BLOCK_SIZE>, FastPForLib::VariableByte> simd_codec;

    const auto values = simpleValues(valuesCount);
    const auto encoded = codec.compress(values);
    const auto simd_encoded = simd_codec.compress(values);

    {
        SCOPED_TRACE("Plain uncompress of plain compress");
        EXPECT_EQ(codec.uncompress(encoded, values.size()), values);
    }
    {
        SCOPED_TRACE("SIMD uncompress of SIMD compress");
        EXPECT_EQ(simd_codec.uncompress(simd_encoded, values.size()), values);
    }
    {
        SCOPED_TRACE("SIMD uncompress of plain compress");
        const auto decoded = simd_codec.uncompress(encoded, values.size());
        if (valuesCount < 32 * BLOCK_SIZE) {
            EXPECT_EQ(decoded, values);
        } else {
            EXPECT_NE(decoded, values);
        }
    }
    {
        SCOPED_TRACE("Plain uncompress of SIMD compress");
        const auto decoded = codec.uncompress(simd_encoded, values.size());
        if (valuesCount < 32 * BLOCK_SIZE) {
            EXPECT_EQ(decoded, values);
        } else {
            EXPECT_NE(decoded, values);
        }
    }
}

struct SIMDInteropCase {
    int values;
    int blockSize;
};

std::vector<SIMDInteropCase> simdInteropCases() {
    constexpr int valueCounts[] = {96, 128, 160, 256, 384};
    constexpr int blockSizes[] = {4, 8};

    std::vector<SIMDInteropCase> cases;
    cases.reserve(std::size(valueCounts) * std::size(blockSizes) * 2);

    for (const int values : valueCounts) {
        for (const int blockSize : blockSizes) {
            cases.push_back({values, blockSize});
        }
    }

    return cases;
}

std::string simdInteropCaseName(const ::testing::TestParamInfo<SIMDInteropCase>& info) {
    return "Values_" + std::to_string(info.param.values) + "_BlockSize_" + std::to_string(info.param.blockSize);
}

class PFORSIMDInteropTest : public ::testing::TestWithParam<SIMDInteropCase> {};

TEST_P(PFORSIMDInteropTest, Interop) {
    const auto testCase = GetParam();
    switch (testCase.blockSize) {
        case 4:
            testSIMDInterop<4>(testCase.values);
            break;
        case 8:
            testSIMDInterop<8>(testCase.values);
            break;
        default:
            FAIL() << "Unsupported block size: " << testCase.blockSize;
    }
}

INSTANTIATE_TEST_SUITE_P(FastPfor, PFORSIMDInteropTest, ::testing::ValuesIn(simdInteropCases()), simdInteropCaseName);
} // namespace
#endif // MLT_WITH_FASTPFOR_SIMD
#endif // MLT_WITH_FASTPFOR
