#pragma once

#include <mlt/util/packed_bitset.hpp>

#include <string>
#include <unordered_map>
#include <vector>
#include <variant>

namespace mlt {

/// A block of data and a collection of strings views on it
using StringDictViews = std::pair<std::vector<std::uint8_t>, std::vector<std::string_view>>;

/// A single feature property.
/// String properties reference the source property vector and must not outlive it.
using Property = std::variant<nullptr_t,
                              bool,
                              std::uint32_t,
                              std::optional<std::uint32_t>,
                              std::uint64_t,
                              std::optional<std::uint64_t>,
                              float,
                              std::optional<float>,
                              double,
                              std::optional<double>,
                              std::string_view>;

using PropertyMap = std::unordered_map<std::string, Property>;

/// A single property for a column, with one value per item
using PropertyVec = std::variant<std::vector<std::uint8_t>,
                                 std::vector<std::uint32_t>,
                                 std::vector<std::uint64_t>,
                                 std::vector<float>,
                                 std::vector<double>,
                                 StringDictViews>;

using PresentProperties = std::pair<PropertyVec, PackedBitset>;

using PropertyVecMap = std::unordered_map<std::string, PresentProperties>;

namespace detail {
struct PropertyCounter {
    template <typename T>
    std::size_t operator()(const std::vector<T>& vec) const {
        return vec.size();
    }
    std::size_t operator()(const StringDictViews& pair) const { return pair.second.size(); }
};
} // namespace detail
static inline std::size_t propertyCount(const PropertyVec& vec) {
    return std::visit(detail::PropertyCounter(), vec);
}

} // namespace mlt