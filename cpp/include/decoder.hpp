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

    Decoder();
    ~Decoder();

    MapLibreTile decode(DataView, const TileSetMetadata&);

private:
    struct Impl;
    std::unique_ptr<Impl> impl;
};

} // namespace mlt::decoder
