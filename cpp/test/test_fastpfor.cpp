#include <gtest/gtest.h>

// From fastpfor/...
#include <fastpfor.h>
#include <compositecodec.h>
#include <variablebyte.h>

#include <algorithm>
#include <cstdint>

// Test less than one block of data, which does not use both parts of the CompositeCodec
TEST(PFOR, DecompressPartial) {
    const std::vector<std::uint32_t> data = {0, 9605520};
    const std::vector<std::uint32_t> expected = {16, 17, 18};

    FastPForLib::CompositeCodec<FastPForLib::FastPFor<8>, FastPForLib::VariableByte> codec;
    auto result = codec.uncompress(data, expected.size());
    EXPECT_TRUE(std::ranges::equal(result, expected));
}
