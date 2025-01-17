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

private:
    std::vector<Layer> layers;
};

} // namespace mlt
