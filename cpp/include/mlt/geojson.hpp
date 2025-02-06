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

class GeoJSON {
public:
    using json = nlohmann::json;

    static json toGeoJSON(const Feature& feature, const Projection& projection) noexcept(false) {
        auto result = json{
            {"type", "Feature"},
            {"id", feature.getID()},
            {"geometry", buildAnyGeometryElement(feature.getGeometry(), projection)},
        };
        if (!feature.getProperties().empty()) {
            result["properties"] = buildProperties(feature.getProperties());
        }
        return result;
    }

    static json toGeoJSON(const Layer& layer, const TileCoordinate& tileCoord) noexcept(false) {
        const auto projection = Projection{layer.getExtent(), tileCoord};
        const auto features = std::ranges::views::all(layer.getFeatures());
        return {
            {"name", layer.getName()},
            {"version", layer.getVersion()},
            {"extent", layer.getExtent()},
            {"features", buildArray(features, [&](const auto& feature) { return toGeoJSON(feature, projection); })}};
    }

    static json toGeoJSON(const MapLibreTile& tile, const TileCoordinate& tileCoord) noexcept(false) {
        const auto layers = std::ranges::views::all(tile.getLayers());
        return {
            {"layers", buildArray(layers, [&](const auto& layer) { return toGeoJSON(layer, tileCoord); })},
        };
    }

private:
#pragma region JSON utils
    /// Create a json array object with pre-allocated space
    static json buildArray(std::size_t reservedSize) noexcept(false) {
        json array = json::array();
        assert(array.is_array());
        array.get_ref<nlohmann::json::array_t&>().reserve(reservedSize);
        return array;
    }

    /// Convert a collection range into a json array by applying the given function to each element
    template <typename TRange, typename TFunc>
        requires requires(TFunc f, typename std::ranges::range_rvalue_reference_t<TRange> v) {
            { f(v) } -> std::same_as<json>;
        }
    static json buildArray(TRange sourceRange, TFunc transform) noexcept(false) {
        return append(sourceRange, buildArray(std::ranges::size(sourceRange)), std::forward<TFunc>(transform));
    }

    /// Add to a json array the result of a function applied to each element in a range
    template <typename TRange, typename TFunc>
        requires requires(TFunc f, typename std::ranges::range_rvalue_reference_t<TRange> v) {
            { f(v) } -> std::same_as<json>;
        }
    static json append(TRange sourceRange, json&& array, TFunc transform) noexcept(false) {
        assert(array.is_array());
        std::ranges::transform(
            std::forward<TRange>(sourceRange), std::back_inserter(array), std::forward<TFunc>(transform));
        return std::move(array);
    }
#pragma endregion JSON utils

#pragma region Geometry
    /// Build the coordinate representation for a single coordinate
    static json buildCoordinateArray(const Coordinate& coord, const Projection& projection) noexcept(false) {
        return buildProjectedCoordinateArray(projection.project(coord));
    }

    /// Build the coordinate representation for a single coordinate that has already been projected
    static json buildProjectedCoordinateArray(const Coordinate& coord) noexcept(false) {
        return json::array({coord.x, coord.y});
    }

    /// Build the coordinate representation for a collection of coordinates
    static json buildCoordinatesArray(const CoordVec& coords, const Projection& projection) noexcept(false) {
        return buildArray(coords, [&](const auto& coord) { return buildCoordinateArray(coord, projection); });
    }

    /// Build the coordinate representation for a polygon, consisting of the rings concatenated to the shell
    static json buildPolygonCoords(const CoordVec& polyShell,
                                   const std::vector<CoordVec>& polyRings,
                                   const Projection& projection) noexcept(false) {
        auto result = buildArray(1 + polyRings.size());
        result.push_back(buildCoordinatesArray(polyShell, projection));
        return append(polyRings, std::move(result), [&](const auto& lineString) {
            return buildCoordinatesArray(lineString, projection);
        });
    }

    /// Create the value type for the "geometry" entry in a GeoJSON feature with the given coordinate representation
    static json buildGeometryElement(std::string_view type, json&& coords) noexcept(false) {
        return {
            {"type", type},
            {"coordinates", std::move(coords)},
        };
    }

    static json buildGeometryElement(const Point& point, const Projection& projection) noexcept(false) {
        return {
            {"type", "Point"},
            {"coordinates", buildCoordinateArray(point.getCoordinate(), projection)},
        };
    }

    static json buildGeometryElement(const LineString& line, const Projection& projection) noexcept(false) {
        return buildGeometryElement("LineString", buildCoordinatesArray(line.getCoordinates(), projection));
    }

    static json buildGeometryElement(const LinearRing& line, const Projection& projection) noexcept(false) {
        return buildGeometryElement("LineString", buildCoordinatesArray(line.getCoordinates(), projection));
    }

    static json buildGeometryElement(const MultiPoint& points, const Projection& projection) noexcept(false) {
        return buildGeometryElement("MultiPoint", buildCoordinatesArray(points.getCoordinates(), projection));
    }

    static json buildGeometryElement(const MultiLineString& mls, const Projection& projection) noexcept(false) {
        return buildGeometryElement("MultiLineString", buildArray(mls.getLineStrings(), [&](const auto& lineString) {
                                        return buildCoordinatesArray(lineString, projection);
                                    }));
    }

    static json buildGeometryElement(const Polygon& poly, const Projection& projection) noexcept(false) {
        return buildGeometryElement("Polygon", buildPolygonCoords(poly.getShell(), poly.getRings(), projection));
    }

    static json buildGeometryElement(const MultiPolygon& poly, const Projection& projection) noexcept(false) {
        return buildGeometryElement("MultiPolygon", buildArray(poly.getPolygons(), [&](const auto& poly) {
                                        return buildPolygonCoords(poly.first, poly.second, projection);
                                    }));
    }

    static json buildAnyGeometryElement(const Geometry& geometry, const Projection& projection) noexcept(false) {
        switch (geometry.type) {
            case metadata::tileset::GeometryType::POINT:
                return buildGeometryElement(static_cast<const Point&>(geometry), projection);
            case metadata::tileset::GeometryType::LINESTRING:
                return buildGeometryElement(static_cast<const LineString&>(geometry), projection);
            case metadata::tileset::GeometryType::POLYGON:
                return buildGeometryElement(static_cast<const Polygon&>(geometry), projection);
            case metadata::tileset::GeometryType::MULTIPOINT:
                return buildGeometryElement(static_cast<const MultiPoint&>(geometry), projection);
            case metadata::tileset::GeometryType::MULTILINESTRING:
                return buildGeometryElement(static_cast<const MultiLineString&>(geometry), projection);
            case metadata::tileset::GeometryType::MULTIPOLYGON:
                return buildGeometryElement(static_cast<const MultiPolygon&>(geometry), projection);
            default:
                throw std::runtime_error("Unsupported geometry type " +
                                         std::to_string(std::to_underlying(geometry.type)));
        }
    }
#pragma endregion Geometry

#pragma region Properties
    static json buildProperties(const PropertyMap& properties) noexcept(false) {
        auto result = json::object();
        for (const auto& [key, value] : properties) {
            if (auto json = std::visit(PropertyVisitor(), value); json) {
                result[key] = std::move(*json);
            }
        }
        return result;
    }

    struct PropertyVisitor {
        template <typename T>
        std::optional<json> operator()(const std::optional<T>& value) const noexcept(false) {
            return value ? operator()(*value) : std::nullopt;
        }
        template <typename T>
        std::optional<json> operator()(const T& value) const noexcept(false) {
            return value;
        }
    };
#pragma endregion Properties
};

} // namespace mlt

#endif
