#pragma once

#include <mlt/common.hpp>
#include <mlt/feature.hpp>
#include <mlt/metadata/tileset.hpp>
#include <mlt/properties.hpp>
#include <mlt/tile.hpp>
#include <mlt/util/noncopyable.hpp>
#include <mlt/geometry.hpp>

#include <memory>
#include <vector>

namespace mlt {

class Decoder : public util::noncopyable {
public:
    using TileSetMetadata = metadata::tileset::TileSetMetadata;
    using Geometry = geometry::Geometry;
    using GeometryFactory = geometry::GeometryFactory;

    Decoder(bool legacy = false, std::unique_ptr<GeometryFactory>&& = std::make_unique<GeometryFactory>());
    ~Decoder() noexcept;
    Decoder(Decoder&&) = delete;
    Decoder& operator=(Decoder&&) = delete;

    MapLibreTile decode(DataView, const TileSetMetadata&);

private:
    struct Impl;
    std::unique_ptr<Impl> impl;
    bool legacy;

    std::vector<Feature> makeFeatures(const std::vector<Feature::id_t>&,
                                      std::vector<std::unique_ptr<Geometry>>&&,
                                      const PropertyVecMap&);
};

} // namespace mlt
