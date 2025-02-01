#pragma once

#include <mlt/feature.hpp>

#include <vector>

namespace mlt {

class Layer {
public:
    using extent_t = std::uint32_t;

    Layer() = delete;
    Layer(const Layer&) = delete;
    Layer(Layer&&) noexcept = default;

    Layer(std::string name_, int version_, extent_t extent_, std::vector<Feature> features_) noexcept
        : name(std::move(name_)),
          version(version_),
          extent(extent_),
          features(std::move(features_)) {}

    const std::string& getName() const noexcept { return name; }
    int getVersion() const noexcept { return version; }
    extent_t getExtent() const noexcept { return extent; }
    const std::vector<Feature>& getFeatures() const noexcept { return features; }

private:
    std::string name;
    int version;
    extent_t extent;
    std::vector<Feature> features;
};

} // namespace mlt
