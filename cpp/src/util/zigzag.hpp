#pragma once

#include <common.hpp>
#include <type_traits>

namespace mlt::util::decoding {

template <typename T>
    requires((std::is_integral_v<T> && std::is_unsigned_v<T>) ||
             (std::is_enum_v<T> && std::is_unsigned_v<std::underlying_type_t<T>>))
T decodeZigZag(T encoded) noexcept {
    const auto signedValue = static_cast<std::make_signed_t<T>>(encoded);
    const auto unsignedValue = static_cast<std::make_unsigned_t<T>>(encoded);
    return static_cast<T>((unsignedValue >> 1) ^ (-(signedValue & 1)));
}

} // namespace mlt::util::decoding
