#pragma once

#include <vector>

namespace mlt::util {

/// Create a vector of N items by invoking the given function N times
template <typename T, typename F>
    requires requires(F f, std::size_t i) {
        { f(i) } -> std::same_as<T>;
    }
std::vector<T> generateVector(const std::size_t count, F generator) noexcept(false) {
    std::vector<T> result;
    result.reserve(count);
    std::generate_n(std::back_inserter(result), count, [i = 0, f = std::move(generator)]() mutable { return f(i++); });
    return result;
}

} // namespace mlt::util
