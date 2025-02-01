#pragma once

#include <vector>

namespace mlt {

struct Coordinate {
    const double x;
    const double y;
};
using CoordVec = std::vector<Coordinate>;

struct TileCoordinate {
    std::uint32_t x;
    std::uint32_t y;
    std::uint32_t z;
};

} // namespace mlt
