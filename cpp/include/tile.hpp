#pragma once

#include <layer.hpp>

#include <vector>

namespace mlt {

class MapLibreTile {
public:
    MapLibreTile() = delete;
    MapLibreTile(const MapLibreTile&) = delete;
    MapLibreTile(MapLibreTile&&) = default;

    MapLibreTile(std::vector<Layer> layers_)
        : layers(std::move(layers_)) {}

    const std::vector<Layer>& getLayers() const { return layers; }

    const Layer* getLayer(const std::string_view& name) const {
        auto hit = std::ranges::find_if(layers, [&](const auto& layer) { return layer.getName() == name; });
        return (hit != layers.end()) ? &*hit : nullptr;
    }

private:
    std::vector<Layer> layers;
};

} // namespace mlt
