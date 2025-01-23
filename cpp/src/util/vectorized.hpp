#pragma once

#include <common.hpp>

#include <vector>

namespace mlt::util::decoding::vectorized {

inline void decodeComponentwiseDeltaVec2(std::vector<std::uint32_t>& data) {
    data[0] = (data[0] >> 1) ^ ((data[0] << 31) >> 31);
    data[1] = (data[1] >> 1) ^ ((data[1] << 31) >> 31);
    for (std::size_t i = 2; i < data.size(); i += 2) {
        data[i] = ((data[i] >> 1) ^ ((data[i] << 31) >> 31)) + data[i - 2];
        data[i + 1] = ((data[i + 1] >> 1) ^ ((data[i + 1] << 31) >> 31)) + data[i - 1];
    }
}
} // namespace mlt::util::decoding::vectorized
