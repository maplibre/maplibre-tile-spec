#include "synthetic-geojson.hpp"

#include <mlt/coordinate.hpp>
#include <mlt/feature.hpp>
#include <mlt/geometry.hpp>
#include <mlt/json.hpp>
#include <mlt/layer.hpp>
#include <mlt/metadata/tileset.hpp>

namespace mlt::test {

using json = nlohmann::json;

namespace {

/// Serialize a coordinate value as an integer if it is a whole number, otherwise as a float
json buildRawCoordValue(float v) {
    auto i = static_cast<std::int64_t>(v);
    return (v == static_cast<float>(i)) ? json(i) : json(v);
}

json buildRawCoordinateArray(const Coordinate& coord) {
    return json::array({buildRawCoordValue(coord.x), buildRawCoordValue(coord.y)});
}

json buildRawCoordinatesArray(const CoordVec& coords) {
    return mlt::json::detail::buildArray(coords, [](const auto& c) { return buildRawCoordinateArray(c); });
}

json buildRawPolygonCoords(const std::vector<CoordVec>& rings) {
    return mlt::json::detail::buildArray(rings, [](const auto& ring) { return buildRawCoordinatesArray(ring); });
}

json buildRawGeometryElement(const geometry::Geometry& geometry) {
    using metadata::tileset::GeometryType;
    switch (geometry.type) {
        case GeometryType::POINT: {
            const auto& g = static_cast<const geometry::Point&>(geometry);
            return {{"type", "Point"}, {"coordinates", buildRawCoordinateArray(g.getCoordinate())}};
        }
        case GeometryType::LINESTRING: {
            const auto& g = static_cast<const geometry::LineString&>(geometry);
            return {{"type", "LineString"}, {"coordinates", buildRawCoordinatesArray(g.getCoordinates())}};
        }
        case GeometryType::POLYGON: {
            const auto& g = static_cast<const geometry::Polygon&>(geometry);
            return {{"type", "Polygon"}, {"coordinates", buildRawPolygonCoords(g.getRings())}};
        }
        case GeometryType::MULTIPOINT: {
            const auto& g = static_cast<const geometry::MultiPoint&>(geometry);
            return {{"type", "MultiPoint"}, {"coordinates", buildRawCoordinatesArray(g.getCoordinates())}};
        }
        case GeometryType::MULTILINESTRING: {
            const auto& g = static_cast<const geometry::MultiLineString&>(geometry);
            return {{"type", "MultiLineString"},
                    {"coordinates", mlt::json::detail::buildArray(g.getLineStrings(), [](const auto& ls) {
                         return buildRawCoordinatesArray(ls);
                     })}};
        }
        case GeometryType::MULTIPOLYGON: {
            const auto& g = static_cast<const geometry::MultiPolygon&>(geometry);
            return {{"type", "MultiPolygon"},
                    {"coordinates", mlt::json::detail::buildArray(g.getPolygons(), [](const auto& poly) {
                         return buildRawPolygonCoords(poly);
                     })}};
        }
        default:
            throw std::runtime_error("Unsupported geometry type " + std::to_string(std::to_underlying(geometry.type)));
    }
}

} // anonymous namespace

json toFeatureCollection(const MapLibreTile& tile) {
    auto features = json::array();
    for (const auto& layer : tile.getLayers()) {
        for (const auto& feature : layer.getFeatures()) {
            auto props = mlt::json::detail::buildProperties(layer, feature);
            props["_extent"] = layer.getExtent();
            props["_layer"] = layer.getName();

            auto featureObj = json{
                {"geometry", buildRawGeometryElement(feature.getGeometry())},
                {"properties", std::move(props)},
                {"type", "Feature"},
            };
            if (const auto id = feature.getID(); id.has_value()) {
                featureObj["id"] = *id;
            }
            features.push_back(std::move(featureObj));
        }
    }
    return {{"features", std::move(features)}, {"type", "FeatureCollection"}};
}

} // namespace mlt::test
