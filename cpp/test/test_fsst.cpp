#include <gtest/gtest.h>

#include <libfsst.hpp>

#include <vector>

TEST(FSST, Decode) {
    const auto expected = "AAAAAAABBBAAACCdddddEEEEEEfffEEEEAAAAAddddCC";
    const std::vector<std::uint8_t> symbols{65, 65, 0, 65, 69, 100, 67, 102, 66};
    const std::vector<std::int32_t> symbolLengths{2, 1, 1, 1, 1, 1, 1, 1};
    const std::vector<std::uint8_t> compressed{
        0, 0, 0, 2, 7, 7, 7, 0, 2, 5, 5, 4, 4, 4, 4, 4, 3, 3, 3,
        3, 3, 3, 6, 6, 6, 3, 3, 3, 3, 0, 0, 2, 4, 4, 4, 4, 5, 5,
    };

    // TODO: how to create the decoder?
}
