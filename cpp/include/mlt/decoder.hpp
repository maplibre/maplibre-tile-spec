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

    /// Decode a tile given its raw data and already-decoded metadata.
    MapLibreTile decode(DataView, const TileMetadata&);
    MapLibreTile decode(BufferStream&, const TileMetadata&);

    /// Decode a tile consisting of metadata and tile data.
    MapLibreTile decodeTile(DataView);

    /// Decode the metadata portion of a tile, leaving the buffer position at the start of the tile data.
    metadata::tileset::TileMetadata decodeTileMetadata(BufferStream&);

private:
    struct Impl;
    std::unique_ptr<Impl> impl;

    std::vector<Feature> makeFeatures(const std::vector<Feature::id_t>&, std::vector<std::unique_ptr<Geometry>>&&);
};

} // namespace mlt
