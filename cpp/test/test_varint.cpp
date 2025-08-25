#include <gtest/gtest.h>

#include <mlt/util/buffer_stream.hpp>
#include <mlt/util/varint.hpp>
#include <mlt/util/vectorized.hpp>

using namespace mlt;

namespace {
template <typename T = std::uint32_t>
T decode(std::vector<std::uint8_t> data) {
    mlt::BufferStream buffer{std::string_view{reinterpret_cast<const char*>(data.data()), data.size()}};
    return mlt::util::decoding::decodeVarint<T>(buffer);
}
} // namespace

TEST(Varint, Size) {
    using namespace mlt::util::decoding;
    EXPECT_EQ(getVarintSize(0u), 1);
    EXPECT_EQ(getVarintSize(1u), 1);
    EXPECT_EQ(getVarintSize(0x7fu), 1);
    EXPECT_EQ(getVarintSize(0x80u), 2);
    EXPECT_EQ(getVarintSize(0x3fffu), 2);
    EXPECT_EQ(getVarintSize(0x4000u), 3);
    EXPECT_EQ(getVarintSize(0xffffffffu), 5);
    EXPECT_EQ(getVarintSize(0x800000000ull), 6);
}

TEST(Varint, Decode) {
    EXPECT_EQ(decode({0x0}), 0x0);
    EXPECT_EQ(decode({0x7f}), 0x7f);
    EXPECT_EQ(decode({0x80, 1}), 0x80);
    EXPECT_EQ(decode({0xff, 0x7f}), 0x3fff);
    EXPECT_THROW(decode({0xff, 0x80}), std::runtime_error);
    EXPECT_EQ(decode({0x80, 0x80, 0x01}), 0x4000);
    EXPECT_EQ(decode({0xff, 0xff, 0x03}), 0xffff);
    EXPECT_EQ(decode({0xff, 0xff, 0xff, 0xff, 0x0f}), 0xffffffff);

    EXPECT_EQ(decode<std::uint64_t>({0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01}),
              0xffffffffffffffffULL);
    EXPECT_THROW(decode<std::uint64_t>({0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x02}),
                 std::runtime_error);
}
