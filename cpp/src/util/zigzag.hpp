#pragma once

#include <common.hpp>

namespace mlt::util::decoding {

namespace detail {
/// Equivalent of `>>>` in Java/C#
template <typename T, int bits>
requires std::is_integral_v<T>
T unsignedShiftRight(T value) noexcept {
    return static_cast<T>(static_cast<std::make_unsigned_t<T>>(value) >> bits);
}
}

template <typename T>
requires std::is_integral_v<T>
T decodeZigZag(T encoded) noexcept {
    return detail::unsignedShiftRight<1>(encoded) ^ (-(encoded & 1));
}

template <typename T>
void decodeZigZag(T* encoded, offset_t count) noexcept {
    for (offset_t i = 0; i < count; ++i) {
        encoded[i] = decodeZigZag(encoded[i]);
    }
}

}
