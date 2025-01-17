#pragma once

#include <feature.hpp>

#include <vector>

namespace mlt {

class Layer {
public:
    Layer() = delete;
    Layer(const Layer&) = delete;
    Layer(Layer&&) = default;

    Layer(std::string name_, std::vector<Feature> features_, int tileExtent_)
        : name(std::move(name_)),
          features(std::move(features_)),
          tileExtent(tileExtent_) {}

    const std::string& getName() const { return name; }
    const std::vector<Feature>& getFeatures() const { return features; }
    int getTileExtent() const { return tileExtent; }

private:
    std::string name;
    std::vector<Feature> features;
    int tileExtent;
};

} // namespace mlt
