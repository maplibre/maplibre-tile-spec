#pragma once

#include <mlt/coordinate.hpp>
#include <mlt/metadata/tileset.hpp>
#include <mlt/util/noncopyable.hpp>

#include <memory>
#include <vector>

namespace mlt {

class Geometry : public util::noncopyable {
public:
    using GeometryType = metadata::tileset::GeometryType;

protected:
    Geometry(GeometryType type_) noexcept
        : type(type_) {}

public:
    virtual ~Geometry() noexcept = default;

    const metadata::tileset::GeometryType type;
};

class Point : public Geometry {
public:
    Point(const Coordinate& coord) noexcept
        : Geometry(GeometryType::POINT),
          coordinate(coord) {}

    const Coordinate& getCoordinate() const noexcept { return coordinate; }

private:
    const Coordinate coordinate;
};

class MultiPoint : public Geometry {
public:
    MultiPoint(CoordVec coords) noexcept
        : Geometry(GeometryType::MULTIPOINT),
          coordinates(std::move(coords)) {}

    const CoordVec& getCoordinates() const noexcept { return coordinates; }

protected:
    MultiPoint(CoordVec coords, GeometryType type_) noexcept
        : Geometry(type_),
          coordinates(std::move(coords)) {}

private:
    CoordVec coordinates;
};

class LineString : public MultiPoint {
public:
    LineString(CoordVec coords) noexcept
        : MultiPoint(std::move(coords), GeometryType::LINESTRING) {}

private:
};

class LinearRing : public MultiPoint {
public:
    LinearRing(CoordVec coords) noexcept
        : MultiPoint(std::move(coords)) {}

private:
};

class MultiLineString : public Geometry {
public:
    MultiLineString(std::vector<CoordVec> lineStrings_) noexcept
        : Geometry(GeometryType::MULTILINESTRING),
          lineStrings(std::move(lineStrings_)) {}

    const std::vector<CoordVec>& getLineStrings() const noexcept { return lineStrings; }

private:
    std::vector<CoordVec> lineStrings;
};

class Polygon : public Geometry {
public:
    using Shell = CoordVec;
    using Ring = CoordVec;
    using RingVec = std::vector<Ring>;

    Polygon(Shell shell_, RingVec rings_) noexcept
        : Geometry(GeometryType::POLYGON),
          shell(std::move(shell_)),
          rings(std::move(rings_)) {}

    const Shell& getShell() const noexcept { return shell; }
    const RingVec& getRings() const noexcept { return rings; }

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

    MultiPolygon(std::vector<ShellRingsPair> polygons_) noexcept
        : Geometry(GeometryType::MULTIPOLYGON),
          polygons(std::move(polygons_)) {}

    const std::vector<ShellRingsPair>& getPolygons() const noexcept { return polygons; }

private:
    std::vector<ShellRingsPair> polygons;
};

class GeometryFactory {
public:
    auto createPoint(const Coordinate& coord) noexcept(false) { return std::make_unique<Point>(coord); }
    auto createMultiPoint(CoordVec&& coords) noexcept(false) { return std::make_unique<MultiPoint>(std::move(coords)); }
    auto createLineString(CoordVec&& coords) noexcept(false) { return std::make_unique<LineString>(std::move(coords)); }
    auto createLinearRing(CoordVec&& coords) noexcept(false) { return std::make_unique<LineString>(std::move(coords)); }
    auto createPolygon(CoordVec&& shell, std::vector<CoordVec>&& rings) noexcept(false) {
        return std::make_unique<Polygon>(std::move(shell), std::move(rings));
    }
    auto createMultiLineString(std::vector<CoordVec>&& lineStrings) noexcept(false) {
        return std::make_unique<MultiLineString>(std::move(lineStrings));
    }
    auto createMultiPolygon(std::vector<MultiPolygon::ShellRingsPair>&& polys) noexcept(false) {
        return std::make_unique<MultiPolygon>(std::move(polys));
    }
};

} // namespace mlt
