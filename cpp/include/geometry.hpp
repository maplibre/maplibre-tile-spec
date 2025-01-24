#pragma once

#include <memory>
#include <vector>

namespace mlt {

struct Coordinate {
    const double x;
    const double y;
};
using CoordVec = std::vector<Coordinate>;

class Geometry {
public:
    virtual ~Geometry() = default;
};

class Point : public Geometry {
public:
    Point(const Coordinate& coord)
        : coordinate(coord) {}

    const Coordinate& getCoordinate() const { return coordinate; }

private:
    const Coordinate coordinate;
};

class MultiPoint : public Geometry {
public:
    MultiPoint(CoordVec coords)
        : coordinates(std::move(coords)) {}

private:
    CoordVec coordinates;
};

class LineString : public MultiPoint {
public:
    LineString(CoordVec coords)
        : MultiPoint(std::move(coords)) {}

private:
};

class LinearRing : public MultiPoint {
public:
    LinearRing(CoordVec coords)
        : MultiPoint(std::move(coords)) {}

private:
};

class MultiLineString : public Geometry {
public:
    MultiLineString(std::vector<CoordVec> lineStrings_)
        : lineStrings(std::move(lineStrings_)) {}

private:
    std::vector<CoordVec> lineStrings;
};

class Polygon : public Geometry {
public:
    using Shell = CoordVec;
    using Ring = CoordVec;
    using RingVec = std::vector<Ring>;

    Polygon(Shell shell_, RingVec rings_)
        : shell(std::move(shell_)),
          rings(std::move(rings_)) {}

private:
    Shell shell;
    RingVec rings;
};

class MultiPolygon : public Geometry {
public:
    using Shell = CoordVec;
    using Ring = CoordVec;
    using RingVec = std::vector<Ring>;
    using ShellRingsPair = std::pair<Shell, RingVec>;

    MultiPolygon(std::vector<ShellRingsPair> polygons_)
        : polygons(std::move(polygons_)) {}

private:
    std::vector<ShellRingsPair> polygons;
};

class GeometryFactory {
public:
    std::unique_ptr<Point> createPoint(const Coordinate& coord) { return std::make_unique<Point>(coord); }
    std::unique_ptr<MultiPoint> createMultiPoint(CoordVec&& coords) {
        return std::make_unique<MultiPoint>(std::move(coords));
    }
    std::unique_ptr<LineString> createLineString(CoordVec&& coords) {
        return std::make_unique<LineString>(std::move(coords));
    }
    std::unique_ptr<LineString> createLinearRing(CoordVec&& coords) {
        return std::make_unique<LineString>(std::move(coords));
    }
    std::unique_ptr<Polygon> createPolygon(CoordVec&& shell, std::vector<CoordVec>&& rings) {
        return std::make_unique<Polygon>(std::move(shell), std::move(rings));
    }
    std::unique_ptr<MultiLineString> createMultiLineString(std::vector<CoordVec>&& lineStrings) {
        return std::make_unique<MultiLineString>(std::move(lineStrings));
    }
    std::unique_ptr<MultiPolygon> createMultiPolygon(std::vector<MultiPolygon::ShellRingsPair>&& polygons) {
        return std::make_unique<MultiPolygon>(std::move(polygons));
    }
};

} // namespace mlt
