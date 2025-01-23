#pragma once

#include <memory>
#include <vector>

namespace mlt {

struct Coordinate {
    const double x;
    const double y;
};

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
    MultiPoint(std::vector<Coordinate> coords)
        : coordinates(std::move(coords)) {}

private:
    std::vector<Coordinate> coordinates;
};

class LineString : public Geometry {
public:
    LineString(std::vector<Coordinate> coords)
        : coordinates(std::move(coords)) {}

private:
    std::vector<Coordinate> coordinates;
};

class GeometryFactory {
public:
    std::unique_ptr<Point> createPoint(const Coordinate& coord) { return std::make_unique<Point>(coord); }
    std::unique_ptr<MultiPoint> createMultiPoint(std::vector<Coordinate> coords) {
        return std::make_unique<MultiPoint>(std::move(coords));
    }
    std::unique_ptr<LineString> createLineString(std::vector<Coordinate> coords) {
        return std::make_unique<LineString>(std::move(coords));
    }
#if 0
    createLineString(vertices: Point[]) {
        return new LineString(vertices);
    }
    createLinearRing(linearRing: Point[]): LinearRing {
        return new LinearRing(linearRing);
    }
    createPolygon(shell: LinearRing, rings: LinearRing[]) {
        return new Polygon(shell, rings);
    }
    createMultiLineString(lineStrings: LineString[]) {
        return new MultiLineString(lineStrings);
    }
    createMultiPolygon(polygons: Polygon[]) {
        return new MultiPolygon(polygons);
    }
#endif
};

} // namespace mlt
