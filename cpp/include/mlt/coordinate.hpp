#pragma once

#include <vector>

namespace mlt {

struct Coordinate {
    double x;
    double y;

    Coordinate(double x_, double y_)
        : x(x_),
          y(y_) {}
};
using CoordVec = std::vector<Coordinate>;

struct TileCoordinate {
    std::uint32_t x;
    std::uint32_t y;
    std::uint32_t z;
};

} // namespace mlt
