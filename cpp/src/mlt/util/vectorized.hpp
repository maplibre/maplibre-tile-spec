#pragma once

#include <mlt/common.hpp>

#include <cassert>
#include <type_traits>
#include <vector>

namespace mlt::util::decoding::vectorized {

template <typename T>
    requires(std::is_integral_v<T> && sizeof(T) == 4)
inline void decodeComponentwiseDeltaVec2(std::vector<T>& data) noexcept {
    assert((data.size() % 2) == 0);
    if (data.size() < 2) {
        return;
    }
    const auto u = [](T x) {
        return static_cast<std::make_unsigned_t<T>>(x);
    };
    const auto t = [](std::make_unsigned_t<T> x) {
        return static_cast<T>(x);
    };
    data[0] = (u(data[0]) >> 1) ^ ((u(data[0]) << 31) >> 31);
    data[1] = (u(data[1]) >> 1) ^ ((u(data[1]) << 31) >> 31);
    for (std::size_t i = 2; i + 1 < data.size(); i += 2) {
        data[i] = t(((u(data[i]) >> 1) ^ ((u(data[i]) << 31) >> 31)) + u(data[i - 2]));
        data[i + 1] = t(((u(data[i + 1]) >> 1) ^ ((u(data[i + 1]) << 31) >> 31)) + u(data[i - 1]));
    }
}
} // namespace mlt::util::decoding::vectorized
