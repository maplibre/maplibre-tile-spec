#pragma once

#include <mlt/common.hpp>

namespace mlt::util::decoding {

static std::uint32_t decodeMorton(std::uint32_t code, int numBits) noexcept {
    std::uint32_t coordinate = 0;
    for (int i = 0; i < numBits; ++i) {
        coordinate |= (code & (1L << (2 * i))) >> i;
    }
    return coordinate;
}

} // namespace mlt::util::decoding
