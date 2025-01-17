#pragma once

#include <common.hpp>
#include <feature.hpp>
#include <metadata/tileset.hpp>
#include <tile.hpp>

namespace mlt::decoder {

class Decoder {
public:
    using TileSetMetadata = metadata::tileset::TileSetMetadata;

    MapLibreTile decode(DataView tileData, const TileSetMetadata&);
};

} // namespace mlt::decoder
