#pragma once

#include <common.hpp>
#include <feature.hpp>
#include <metadata/tileset.hpp>
#include <tile.hpp>

#include <memory>

namespace mlt::decoder {

class Decoder {
public:
    using TileSetMetadata = metadata::tileset::TileSetMetadata;

    Decoder() noexcept(false);
    ~Decoder();

    Decoder(const Decoder&) = delete;
    Decoder(Decoder&&) noexcept = default;
    Decoder& operator=(const Decoder&) = delete;
    Decoder& operator=(Decoder&&) noexcept = default;

    MapLibreTile decode(DataView, const TileSetMetadata&) noexcept(false);

private:
    struct Impl;
    std::unique_ptr<Impl> impl;

    std::vector<Feature> makeFeatures(const std::vector<Feature::id_t>&,
                                      std::vector<std::unique_ptr<Geometry>>&&,
                                      const Feature::PropertyMap&,
                                      const Feature::extent_t,
                                      const count_t numFeatures) noexcept(false);
};

} // namespace mlt::decoder
