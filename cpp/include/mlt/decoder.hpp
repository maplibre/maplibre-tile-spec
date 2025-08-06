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
    using TileMetadata = metadata::tileset::TileMetadata;
    using Geometry = geometry::Geometry;
    using GeometryFactory = geometry::GeometryFactory;

    Decoder(std::unique_ptr<GeometryFactory>&& = std::make_unique<GeometryFactory>());
    ~Decoder() noexcept;
    Decoder(Decoder&&) = delete;
    Decoder& operator=(Decoder&&) = delete;

    MapLibreTile decode(DataView, const TileMetadata&);
    MapLibreTile decode(BufferStream&, const TileMetadata&);

private:
    struct Impl;
    std::unique_ptr<Impl> impl;

    std::vector<Feature> makeFeatures(const std::vector<Feature::id_t>&, std::vector<std::unique_ptr<Geometry>>&&);
};

} // namespace mlt
