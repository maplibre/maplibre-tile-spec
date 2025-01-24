#pragma once

#include <feature.hpp>

#include <vector>

namespace mlt {

class Layer {
public:
    Layer() = delete;
    Layer(const Layer&) = delete;
    Layer(Layer&&) = default;

    Layer(std::string name_, int version_, Feature::extent_t tileExtent_, std::vector<Feature> features_)
        : name(std::move(name_)),
          version(version_),

          tileExtent(tileExtent_),
          features(std::move(features_)) {}

    const std::string& getName() const { return name; }
    int getVersion() const { return version; }
    Feature::extent_t getTileExtent() const { return tileExtent; }
    const std::vector<Feature>& getFeatures() const { return features; }

private:
    std::string name;
    int version;
    std::vector<Feature> features;
    Feature::extent_t tileExtent;
};

} // namespace mlt
