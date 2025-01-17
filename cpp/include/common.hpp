#pragma once

#include <string_view>
#include <utility>

namespace mlt {

using DataView = std::string_view;

using offset_t = std::uint32_t;

} // namespace mlt

#if !__has_cpp_attribute(__cpp_lib_to_underlying)
namespace std {
template <typename E>
constexpr auto to_underlying(E e) noexcept {
    return static_cast<std::underlying_type_t<E>>(e);
}
} // namespace std
#endif
