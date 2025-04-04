#include <gtest/gtest.h>

#include <mlt/util/buffer_stream.hpp>
#include <mlt/util/varint.hpp>

using namespace mlt;

namespace {
template <typename T = std::uint32_t>
T decode(std::vector<std::uint8_t> data) {
    mlt::BufferStream buffer{std::string_view{reinterpret_cast<const char*>(data.data()), data.size()}};
    return mlt::util::decoding::decodeVarint<T>(buffer);
}
} // namespace

TEST(Varint, All) {
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
