#pragma once

#include <cstdint>
#include <iostream>
#include <string_view>
#include <utility>

namespace mlt {

using DataView = std::string_view;

using count_t = std::uint32_t;

template <typename T, std::size_t N>
constexpr std::size_t countof(T (&)[N]) {
    return N;
}

#define MLT_LOG_WARN(s) std::cerr << s << '\n'

} // namespace mlt

namespace std {
#if !__has_cpp_attribute(__cpp_lib_to_underlying)
template <typename E>
constexpr auto to_underlying(E e) noexcept {
    return static_cast<std::underlying_type_t<E>>(e);
}
#endif

#if !__has_cpp_attribute(__cpp_lib_byteswap)
template <std::integral T>
constexpr T byteswap(T value) noexcept {
    static_assert(std::has_unique_object_representations_v<T>, "T may not have padding bits");
    auto value_representation = std::bit_cast<std::array<std::byte, sizeof(T)>>(value);
    std::ranges::reverse(value_representation);
    return std::bit_cast<T>(value_representation);
}
#endif
} // namespace std
