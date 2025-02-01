#pragma once

#include <mlt/coordinate.hpp>
#include <mlt/metadata/tileset.hpp>

#include <memory>
#include <vector>

namespace mlt {

class Geometry {
public:
    using GeometryType = metadata::tileset::GeometryType;

protected:
    Geometry(GeometryType type_)
        : type(type_) {}

public:
    virtual ~Geometry() = default;

    const metadata::tileset::GeometryType type;
};

class Point : public Geometry {
public:
    Point(const Coordinate& coord)
        : Geometry(GeometryType::POINT),
          coordinate(coord) {}

    const Coordinate& getCoordinate() const { return coordinate; }

private:
    const Coordinate coordinate;
};

class MultiPoint : public Geometry {
public:
    MultiPoint(CoordVec coords)
        : Geometry(GeometryType::MULTIPOINT),
          coordinates(std::move(coords)) {}

    const CoordVec& getCoordinates() const { return coordinates; }

protected:
    MultiPoint(CoordVec coords, GeometryType type_)
        : Geometry(type_),
          coordinates(std::move(coords)) {}

private:
    CoordVec coordinates;
};

class LineString : public MultiPoint {
public:
    LineString(CoordVec coords)
        : MultiPoint(std::move(coords), GeometryType::LINESTRING) {}

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
        : Geometry(GeometryType::MULTILINESTRING),
          lineStrings(std::move(lineStrings_)) {}

    const std::vector<CoordVec>& getLineStrings() const { return lineStrings; }

private:
    std::vector<CoordVec> lineStrings;
};

class Polygon : public Geometry {
public:
    using Shell = CoordVec;
    using Ring = CoordVec;
    using RingVec = std::vector<Ring>;

    Polygon(Shell shell_, RingVec rings_)
        : Geometry(GeometryType::POLYGON),
          shell(std::move(shell_)),
          rings(std::move(rings_)) {}

    const Shell& getShell() const { return shell; }
    const RingVec& getRings() const { return rings; }

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
        : Geometry(GeometryType::MULTIPOLYGON),
          polygons(std::move(polygons_)) {}

    const std::vector<ShellRingsPair>& getPolygons() const { return polygons; }

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
