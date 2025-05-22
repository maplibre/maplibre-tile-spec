#pragma once

#if MLT_WITH_JSON
#include <mlt/common.hpp>
#include <mlt/coordinate.hpp>
#include <mlt/feature.hpp>
#include <mlt/geometry.hpp>
#include <mlt/layer.hpp>
#include <mlt/projection.hpp>
#include <mlt/tile.hpp>

#include <nlohmann/json.hpp>

#include <iterator>
#include <stdexcept>
#include <string>
#include <utility>

namespace mlt {
namespace geojson {

using json = nlohmann::json;

namespace detail {

#pragma region JSON utils
/// Create a json array object with pre-allocated space
inline json buildArray(std::size_t reservedSize) {
    json array = json::array();
    assert(array.is_array());
    array.get_ref<nlohmann::json::array_t&>().reserve(reservedSize);
    return array;
}

/// Add to a json array the result of a function applied to each element in a range
template <typename TRange, typename TFunc>
    requires requires(TFunc f, typename std::ranges::range_rvalue_reference_t<TRange> v) {
        { f(v) } -> std::same_as<json>;
    }
inline json append(TRange sourceRange, json&& array, TFunc transform) {
    assert(array.is_array());
    std::ranges::transform(
        std::forward<TRange>(sourceRange), std::back_inserter(array), std::forward<TFunc>(transform));
    return std::move(array);
}

/// Convert a collection range into a json array by applying the given function to each element
template <typename TRange, typename TFunc>
    requires requires(TFunc f, typename std::ranges::range_rvalue_reference_t<TRange> v) {
        { f(v) } -> std::same_as<json>;
    }
inline json buildArray(TRange sourceRange, TFunc transform) {
    return append(sourceRange, buildArray(std::ranges::size(sourceRange)), std::forward<TFunc>(transform));
}
#pragma endregion JSON utils

#pragma region Geometry
/// Build the coordinate representation for a single coordinate that has already been projected
inline json buildProjectedCoordinateArray(const Coordinate& coord) {
    return json::array({coord.x, coord.y});
}

/// Build the coordinate representation for a single coordinate
inline json buildCoordinateArray(const Coordinate& coord, const Projection& projection) {
    return buildProjectedCoordinateArray(projection.project(coord));
}

/// Build the coordinate representation for a collection of coordinates
inline json buildCoordinatesArray(const CoordVec& coords, const Projection& projection) {
    return buildArray(coords, [&](const auto& coord) { return buildCoordinateArray(coord, projection); });
}

/// Build the coordinate representation for a polygon, consisting of the rings concatenated to the shell
inline json buildPolygonCoords(const std::vector<CoordVec>& polyRings, const Projection& projection) {
    auto result = buildArray(polyRings.size());
    return append(polyRings, std::move(result), [&](const auto& lineString) {
        return buildCoordinatesArray(lineString, projection);
    });
}

/// Create the value type for the "geometry" entry in a GeoJSON feature with the given coordinate representation
inline json buildGeometryElement(std::string_view type, json&& coords) {
    return {
        {"type", type},
        {"coordinates", std::move(coords)},
    };
}

inline json buildGeometryElement(const geometry::Point& point, const Projection& projection) {
    return {
        {"type", "Point"},
        {"coordinates", buildCoordinateArray(point.getCoordinate(), projection)},
    };
}

inline json buildGeometryElement(const geometry::LineString& line, const Projection& projection) {
    return buildGeometryElement("LineString", buildCoordinatesArray(line.getCoordinates(), projection));
}

inline json buildGeometryElement(const geometry::LinearRing& line, const Projection& projection) {
    return buildGeometryElement("LineString", buildCoordinatesArray(line.getCoordinates(), projection));
}

inline json buildGeometryElement(const geometry::MultiPoint& points, const Projection& projection) {
    return buildGeometryElement("MultiPoint", buildCoordinatesArray(points.getCoordinates(), projection));
}

inline json buildGeometryElement(const geometry::MultiLineString& mls, const Projection& projection) {
    return buildGeometryElement("MultiLineString", buildArray(mls.getLineStrings(), [&](const auto& lineString) {
                                    return buildCoordinatesArray(lineString, projection);
                                }));
}

inline json buildGeometryElement(const geometry::Polygon& poly, const Projection& projection) {
    return buildGeometryElement("Polygon", buildPolygonCoords(poly.getRings(), projection));
}

inline json buildGeometryElement(const geometry::MultiPolygon& poly, const Projection& projection) {
    return buildGeometryElement("MultiPolygon", buildArray(poly.getPolygons(), [&](const auto& poly) {
                                    return buildPolygonCoords(poly, projection);
                                }));
}

inline json buildAnyGeometryElement(const geometry::Geometry& geometry, const Projection& projection) {
    switch (geometry.type) {
        case metadata::tileset::GeometryType::POINT:
            return buildGeometryElement(static_cast<const geometry::Point&>(geometry), projection);
        case metadata::tileset::GeometryType::LINESTRING:
            return buildGeometryElement(static_cast<const geometry::LineString&>(geometry), projection);
        case metadata::tileset::GeometryType::POLYGON:
            return buildGeometryElement(static_cast<const geometry::Polygon&>(geometry), projection);
        case metadata::tileset::GeometryType::MULTIPOINT:
            return buildGeometryElement(static_cast<const geometry::MultiPoint&>(geometry), projection);
        case metadata::tileset::GeometryType::MULTILINESTRING:
            return buildGeometryElement(static_cast<const geometry::MultiLineString&>(geometry), projection);
        case metadata::tileset::GeometryType::MULTIPOLYGON:
            return buildGeometryElement(static_cast<const geometry::MultiPolygon&>(geometry), projection);
        default:
            throw std::runtime_error("Unsupported geometry type " + std::to_string(std::to_underlying(geometry.type)));
    }
}
#pragma endregion Geometry

#pragma region Properties
struct PropertyVisitor {
    template <typename T>
    std::optional<json> operator()(const T& value) const {
        return value;
    }
};

inline json buildProperties(const Layer& layer, const Feature& feature) {
    auto result = json::object();
    for (const auto& [key, _] : layer.getProperties()) {
        if (const auto property = feature.getProperty(key, layer); property) {
            if (auto json = std::visit(PropertyVisitor(), *property); json) {
                result[key] = std::move(*json);
            }
        }
    }
    return result;
}
#pragma endregion Properties

} // namespace detail

inline json toGeoJSON(const Layer& layer, const Feature& feature, const Projection& projection) {
    auto result = json{
        {"type", "Feature"},
        {"id", feature.getID()},
        {"geometry", detail::buildAnyGeometryElement(feature.getGeometry(), projection)},
    };
    if (!layer.getProperties().empty()) {
        result["properties"] = detail::buildProperties(layer, feature);
    }
    return result;
}

inline json toGeoJSON(const Layer& layer, const TileCoordinate& tileCoord) {
    const auto projection = Projection{layer.getExtent(), tileCoord};
    const auto features = std::ranges::views::all(layer.getFeatures());
    return {{"name", layer.getName()},
            {"version", layer.getVersion()},
            {"extent", layer.getExtent()},
            {"features",
             detail::buildArray(features, [&](const auto& feature) { return toGeoJSON(layer, feature, projection); })}};
}

inline json toGeoJSON(const MapLibreTile& tile, const TileCoordinate& tileCoord) {
    const auto layers = std::ranges::views::all(tile.getLayers());
    return {
        {"layers", detail::buildArray(layers, [&](const auto& layer) { return toGeoJSON(layer, tileCoord); })},
    };
}

} // namespace geojson
} // namespace mlt

#endif
