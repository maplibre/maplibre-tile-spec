#pragma once

#include <mlt/util/packed_bitset.hpp>
#include <mlt/util/noncopyable.hpp>

#include <cstdint>
#include <string>
#include <unordered_map>
#include <vector>
#include <variant>

namespace mlt {

/// A block of data and a collection of strings views on it
class StringDictViews : util::noncopyable {
public:
    StringDictViews() = default;
    StringDictViews(std::vector<std::uint8_t>&& data_, std::vector<std::string_view> views_) noexcept
        : data(std::move(data_)),
          views(std::move(views_)) {}
    StringDictViews(StringDictViews&&) noexcept = default;
    StringDictViews& operator=(StringDictViews&&) = delete;

    const auto& getStrings() const noexcept { return views; }

private:
    std::vector<std::uint8_t> data;
    std::vector<std::string_view> views;
};

/// A single feature property.
/// String properties reference the source property vector and must not outlive it.
using Property = std::variant<nullptr_t,
                              bool,
                              std::optional<bool>,
                              std::uint32_t,
                              std::optional<std::uint32_t>,
                              std::uint64_t,
                              std::optional<std::uint64_t>,
                              float,
                              std::optional<float>,
                              double,
                              std::optional<double>,
                              std::string_view>;

/// Map of properties for a single feature
using PropertyMap = std::unordered_map<std::string, Property>;

/// A single property for a column, with one value per feature
using PropertyVec = std::variant<std::vector<std::uint8_t>,
                                 std::vector<std::uint32_t>,
                                 std::vector<std::uint64_t>,
                                 std::vector<float>,
                                 std::vector<double>,
                                 StringDictViews>;

/// A column of properties and the present bits for each feature
class PresentProperties : public util::noncopyable {
public:
    PresentProperties() = delete;
    PresentProperties(PropertyVec properties_, PackedBitset present_) noexcept
        : properties(std::move(properties_)),
          present(std::move(present_)) {}

    const PropertyVec& getProperties() const noexcept { return properties; }
    const PackedBitset& getPresentBits() const noexcept { return present; }

private:
    PropertyVec properties;
    PackedBitset present;
};

/// All the property columns for a layer
using PropertyVecMap = std::unordered_map<std::string, PresentProperties>;

namespace detail {
struct PropertyCounter {
    template <typename T>
    std::size_t operator()(const std::vector<T>& vec) const noexcept {
        return vec.size();
    }
    std::size_t operator()(const StringDictViews& views) const noexcept { return views.getStrings().size(); }
};
} // namespace detail
static inline std::size_t propertyCount(const PropertyVec& vec) noexcept(false) {
    return std::visit(detail::PropertyCounter(), vec);
}

} // namespace mlt