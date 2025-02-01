#pragma once

#include <mlt/common.hpp>
#include <mlt/feature.hpp>
#include <mlt/metadata/tileset.hpp>
#include <mlt/tile.hpp>

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
                                      const Feature::PropertyMap&) noexcept(false);
};

} // namespace mlt::decoder
