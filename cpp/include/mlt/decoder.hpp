#pragma once

#include <mlt/common.hpp>
#include <mlt/feature.hpp>
#include <mlt/metadata/tileset.hpp>
#include <mlt/properties.hpp>
#include <mlt/tile.hpp>
#include <mlt/util/noncopyable.hpp>

#include <memory>
#include <vector>

namespace mlt::decoder {

class Decoder : public util::noncopyable {
public:
    using TileSetMetadata = metadata::tileset::TileSetMetadata;

    Decoder() noexcept(false);
    ~Decoder() noexcept;
    Decoder(Decoder&&) = delete;
    Decoder& operator=(Decoder&&) = delete;

    MapLibreTile decode(DataView, const TileSetMetadata&) noexcept(false);

private:
    struct Impl;
    std::unique_ptr<Impl> impl;

    std::vector<Feature> makeFeatures(const std::vector<Feature::id_t>&,
                                      std::vector<std::unique_ptr<Geometry>>&&,
                                      const PropertyVecMap&) noexcept(false);
};

} // namespace mlt::decoder
