#pragma once

#include <string>
#include <unordered_map>
#include <vector>
#include <variant>

namespace mlt {

/// A block of data and a collection of strings views on it
using StringDictViews = std::pair<std::vector<std::uint8_t>,std::vector<std::string_view>>;

/// A single feature property.
/// String properties reference the source property vector and must not outlive it.
using Property = std::variant<nullptr_t,
                                bool,
                                std::uint32_t,
                                std::uint64_t,
                                float,
                                double,
                                std::string_view>;

using PropertyMap = std::unordered_map<std::string, Property>;

/// A single property for a column, with one value per item
using PropertyVec = std::variant<std::vector<std::uint8_t>, // Booleans packing into bytes
                                std::vector<std::uint32_t>,
                                std::vector<std::uint64_t>,
                                std::vector<float>,
                                std::vector<double>,
                                StringDictViews>;

using PropertyVecMap = std::unordered_map<std::string, PropertyVec>;

} // namespace mlt