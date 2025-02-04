#pragma once

#include <mlt/common.hpp>

#include <cassert>
#include <type_traits>
#include <vector>

namespace mlt::util::decoding::vectorized {

namespace {
template <int bits, typename T>
T unsigned_rshift(T t) {
    return static_cast<T>(static_cast<std::make_unsigned_t<T>>(t) >> bits);
}
}

template <typename T>
    requires(std::is_integral_v<T> && sizeof(T) == 4)
inline void decodeComponentwiseDeltaVec2(std::vector<T>& data) noexcept {
    assert((data.size() % 2) == 0);
    if (1 < data.size()) {
        data[0] = unsigned_rshift<1>(data[0]) ^ ((data[0] << 31) >> 31);
        data[1] = unsigned_rshift<1>(data[1]) ^ ((data[1] << 31) >> 31);
        for (std::size_t i = 2; i + 1 < data.size(); i += 2) {
            data[i] = unsigned_rshift<1>(data[i]) ^ ((data[i] << 31) >> 31) + data[i - 2];
            data[i + 1] = unsigned_rshift<1>(data[i + 1]) ^ ((data[i + 1] << 31) >> 31) + data[i - 1];
        }
    }
}

} // namespace mlt::util::decoding::vectorized
