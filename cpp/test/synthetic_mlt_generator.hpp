#pragma once

#include <mlt/encoder.hpp>
#include <mlt/metadata/tileset.hpp>

#include <cstdint>
#include <map>
#include <string>
#include <utility>
#include <vector>

namespace mlt::test {

class SyntheticMltGenerator {
public:
    using Vertex = Encoder::Vertex;
    using Geometry = Encoder::Geometry;
    using Feature = Encoder::Feature;
    using Layer = Encoder::Layer;
    using PropertyValue = Encoder::PropertyValue;
    using PropertyMap = std::map<std::string, PropertyValue>;
    using Ring = std::vector<Vertex>;
    using Rings = std::vector<Ring>;
    using Parts = std::vector<Ring>;
    using Polygons = std::vector<Rings>;

    struct GeneratedTile {
        std::string name;
        std::vector<std::uint8_t> bytes;
    };

    static constexpr std::uint32_t defaultExtent = 80;
    static constexpr const char* defaultLayerName = "layer1";
    static constexpr Vertex c0 = {.x = 13, .y = 42};

    static constexpr Vertex c(std::int32_t x, std::int32_t y) noexcept { return {.x = x, .y = y}; }

    static Geometry point(Vertex coord) {
        return {
            .type = metadata::tileset::GeometryType::POINT,
            .coordinates = {coord},
        };
    }

    static Geometry line(Ring coords) {
        return {
            .type = metadata::tileset::GeometryType::LINESTRING,
            .coordinates = std::move(coords),
        };
    }

    static Geometry poly(Ring shell) { return poly(Rings{std::move(shell)}); }

    static Geometry poly(Rings rings) {
        Geometry geometry = {
            .type = metadata::tileset::GeometryType::POLYGON,
        };
        for (auto& currentRing : rings) {
            geometry.ringSizes.push_back(static_cast<std::uint32_t>(currentRing.size()));
            geometry.coordinates.insert(geometry.coordinates.end(),
                                        std::make_move_iterator(currentRing.begin()),
                                        std::make_move_iterator(currentRing.end()));
        }
        return geometry;
    }

    static Geometry multiPoint(Ring coords) {
        return {
            .type = metadata::tileset::GeometryType::MULTIPOINT,
            .coordinates = std::move(coords),
        };
    }

    static Geometry multiLine(Parts lines) {
        return {
            .type = metadata::tileset::GeometryType::MULTILINESTRING,
            .parts = std::move(lines),
        };
    }

    static Geometry multiPoly(Polygons polygons) {
        Geometry geometry = {
            .type = metadata::tileset::GeometryType::MULTIPOLYGON,
        };
        for (auto& polygon : polygons) {
            Ring coordinates;
            std::vector<std::uint32_t> ringSizes;
            for (auto& currentRing : polygon) {
                ringSizes.push_back(static_cast<std::uint32_t>(currentRing.size()));
                coordinates.insert(coordinates.end(),
                                   std::make_move_iterator(currentRing.begin()),
                                   std::make_move_iterator(currentRing.end()));
            }
            geometry.parts.push_back(std::move(coordinates));
            geometry.partRingSizes.push_back(std::move(ringSizes));
        }
        return geometry;
    }

    static Feature feat(Geometry geometry, PropertyMap properties = {}) {
        return {
            .geometry = std::move(geometry),
            .properties = std::move(properties),
        };
    }

    static Layer layer(std::string name, std::vector<Feature> features, std::uint32_t extent = defaultExtent) {
        return {
            .name = std::move(name),
            .extent = extent,
            .features = std::move(features),
        };
    }

    static EncoderConfig cfg() {
        return {
            .useFastPfor = false,
            .includeIds = false,
            .sortFeatures = false,
            .preTessellate = false,
            .includeOutlines = true,
            .useMortonEncoding = false,
            .useFsst = false,
        };
    }

    static std::vector<std::uint8_t> encode(Layer layer, const EncoderConfig& config = cfg()) {
        return Encoder().encode({std::move(layer)}, config);
    }

    static GeneratedTile generatePoints() {
        return {
            .name = "point",
            .bytes = encode(layer(defaultLayerName, {feat(point(c0))}, defaultExtent), cfg()),
        };
    }
};

} // namespace mlt::test
