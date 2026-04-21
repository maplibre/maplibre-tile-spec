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
    using StructValue = Encoder::StructValue;
    using PropertyValue = Encoder::PropertyValue;
    using PropertyMap = std::map<std::string, PropertyValue>;
    using Ring = std::vector<Vertex>;
    using RingVec = std::vector<Ring>;
    using PartVec = std::vector<Ring>;
    using PolygonVec = std::vector<RingVec>;

    struct GeneratedTile {
        std::string name;
        std::vector<std::uint8_t> bytes;
    };
    using GeneratedTileVec = std::vector<GeneratedTile>;

    static constexpr std::uint32_t defaultExtent = 80;
    static constexpr const char* defaultLayerName = "layer1";
    static constexpr Vertex c0 = {.x = 13, .y = 42};
    // Additional coordinates matching Java SyntheticMltUtil
    static constexpr Vertex c1 = {.x = 11, .y = 52};
    static constexpr Vertex c2 = {.x = 71, .y = 72};
    static constexpr Vertex c3 = {.x = 61, .y = 22};
    static constexpr Vertex c21 = {.x = 23, .y = 34};
    static constexpr Vertex c22 = {.x = 73, .y = 4};
    static constexpr Vertex c23 = {.x = 13, .y = 24};
    // Hole coordinates
    static constexpr Vertex h1 = {.x = 65, .y = 66};
    static constexpr Vertex h2 = {.x = 35, .y = 56};
    static constexpr Vertex h3 = {.x = 55, .y = 36};

    static constexpr Vertex c(std::int32_t x, std::int32_t y) noexcept { return {.x = x, .y = y}; }

    /// De-interleave Z-order index bits into x (even bits) and y (odd bits) coordinates.
    static Ring buildMortonCurve(std::size_t numPoints, std::int32_t scale, std::uint32_t mortonBits) {
        Ring curve;
        curve.reserve(numPoints);

        for (std::size_t i = 0; i < numPoints; ++i) {
            std::int32_t x = 0;
            std::int32_t y = 0;

            for (std::uint32_t b = 0; b < mortonBits; ++b) {
                x |= static_cast<std::int32_t>(((i >> (2 * b)) & 1ULL) << b);
                y |= static_cast<std::int32_t>(((i >> (2 * b + 1)) & 1ULL) << b);
            }

            curve.push_back(c(x * scale, y * scale));
        }

        return curve;
    }

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

    static Geometry poly(Ring shell) { return poly(RingVec{std::move(shell)}); }

    static Geometry poly(RingVec rings) {
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

    static Geometry multiLine(PartVec lines) {
        return {
            .type = metadata::tileset::GeometryType::MULTILINESTRING,
            .parts = std::move(lines),
        };
    }

    static Geometry multiPoly(PolygonVec polygons) {
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

    static Feature featWithId(std::uint64_t id, Geometry geometry, PropertyMap properties = {}) {
        return {
            .id = id,
            .geometry = std::move(geometry),
            .properties = std::move(properties),
        };
    }

    static Feature featWithoutId(Geometry geometry, PropertyMap properties = {}) {
        auto feature = feat(std::move(geometry), std::move(properties));
        feature.id = std::nullopt;
        return feature;
    }

    static Layer layer(std::string name, std::vector<Feature> features, std::uint32_t extent = defaultExtent) {
        return {
            .name = std::move(name),
            .extent = extent,
            .features = std::move(features),
        };
    }

    static EncoderConfig cfg(std::function<void(EncoderConfig&)> customizer = {}) {
        EncoderConfig config = {
            .useFastPfor = false,
            .includeIds = false,
            .sortFeatures = false,
            .preTessellate = false,
            .includeOutlines = true,
            .useMortonEncoding = false,
            .useFsst = false,
            .geometryEncodingOption = IntegerEncodingOption::PLAIN,
        };
        if (customizer) {
            customizer(config);
        }
        return config;
    }

    static EncoderConfig cfgWithIds() {
        auto config = cfg();
        config.includeIds = true;
        return config;
    }

    static EncoderConfig cfgWithFsst() {
        auto config = cfg();
        config.useFsst = true;
        return config;
    }

    static EncoderConfig cfgWithMorton() {
        auto config = cfg();
        config.useMortonEncoding = true;
        return config;
    }

    static EncoderConfig cfgWithPlainIntegers() {
        auto config = cfg();
        config.integerEncodingOption = IntegerEncodingOption::PLAIN;
        return config;
    }

    static std::vector<std::uint8_t> encode(Layer layer, const EncoderConfig& config = cfg()) {
        return Encoder().encode({std::move(layer)}, config);
    }

    // Helper to create a ring from vertex coordinates
    static Ring ring(const Vertex* vertices, std::size_t count) { return Ring(vertices, vertices + count); }

    static GeneratedTile generatePoints() {
        return {
            .name = "point",
            .bytes = encode(layer(defaultLayerName, {feat(point(c0))}, defaultExtent), cfg()),
        };
    }

    static GeneratedTile generateLine() {
        Ring line_coords{c1, c2, c3};
        return {
            .name = "line",
            .bytes = encode(layer(defaultLayerName, {feat(line(line_coords))}, defaultExtent), cfg()),
        };
    }

    static GeneratedTile generateLineZeroLength() {
        Ring line_coords{c(6, 6), c(6, 6)};
        return {
            .name = "line_zero_length",
            .bytes = encode(layer(defaultLayerName, {feat(line(line_coords))}, defaultExtent), cfg()),
        };
    }

    static std::vector<GeneratedTile> generateLineMorton() {
        Ring line_coords = buildMortonCurve(16, 8, 4);
        return {{
                    .name = "line_morton_curve_morton",
                    .bytes = encode(layer(defaultLayerName, {feat(line(line_coords))}, defaultExtent), cfg([](auto& c) {
                                        c.useMortonEncoding = true;
                                        c.forceMortonGeometryLayout = true;
                                    })),
                },
                {
                    .name = "line_morton_curve_no_morton",
                    .bytes = encode(layer(defaultLayerName, {feat(line(line_coords))}, defaultExtent),
                                    cfg([](auto& c) { c.useMortonEncoding = false; })),
                }};
    }

    static GeneratedTileVec generateLines() { return {generateLine(), generateLineZeroLength()}; }

    static GeneratedTileVec generatePolyVariants(const std::string& baseName, const Feature& polygonFeature) {
        GeneratedTileVec tiles;

        const auto plainCfg = cfg([](auto& c) {
            c.geometryEncodingOption = IntegerEncodingOption::AUTO;
            c.geometryTopologyEncodingOption = IntegerEncodingOption::AUTO;
        });

        const auto fastPforCfg = cfg([](auto& c) {
            c.geometryEncodingOption = IntegerEncodingOption::AUTO;
            c.geometryTopologyEncodingOption = IntegerEncodingOption::AUTO;
            c.useFastPfor = true;
        });

        const auto tessellatedCfg = cfg([](auto& c) {
            c.geometryEncodingOption = IntegerEncodingOption::AUTO;
            c.geometryTopologyEncodingOption = IntegerEncodingOption::AUTO;
            c.preTessellate = true;
        });

        const auto fastPforTessellatedCfg = cfg([](auto& c) {
            c.geometryEncodingOption = IntegerEncodingOption::AUTO;
            c.geometryTopologyEncodingOption = IntegerEncodingOption::AUTO;
            c.useFastPfor = true;
            c.preTessellate = true;
        });

        tiles.push_back({
            .name = baseName,
            .bytes = encode(layer(defaultLayerName, {polygonFeature}, defaultExtent), plainCfg),
        });
        tiles.push_back({
            .name = baseName + "_fpf",
            .bytes = encode(layer(defaultLayerName, {polygonFeature}, defaultExtent), fastPforCfg),
        });
        tiles.push_back({
            .name = baseName + "_tes",
            .bytes = encode(layer(defaultLayerName, {polygonFeature}, defaultExtent), tessellatedCfg),
        });
        tiles.push_back({
            .name = baseName + "_fpf_tes",
            .bytes = encode(layer(defaultLayerName, {polygonFeature}, defaultExtent), fastPforTessellatedCfg),
        });

        return tiles;
    }

    static GeneratedTileVec generatePolygons() {
        GeneratedTileVec tiles;

        const auto appendAll = [&](GeneratedTileVec generated) {
            tiles.insert(tiles.end(), generated.begin(), generated.end());
        };

        // Java poly/poly_fpf/poly_tes/poly_fpf_tes
        appendAll(generatePolyVariants("poly", feat(poly(Ring{c1, c2, c3}))));

        // Java poly_collinear*
        appendAll(generatePolyVariants("poly_collinear", feat(poly(Ring{c(0, 0), c(10, 0), c(20, 0)}))));

        // Java poly_self_intersect*
        appendAll(
            generatePolyVariants("poly_self_intersect", feat(poly(Ring{c(0, 0), c(10, 10), c(0, 10), c(10, 0)}))));

        // Java poly_hole*
        appendAll(generatePolyVariants("poly_hole", feat(poly(RingVec{Ring{c1, c2, c3}, Ring{h1, h2, h3}}))));

        // Java poly_hole_touching*
        appendAll(generatePolyVariants(
            "poly_hole_touching",
            feat(poly(RingVec{Ring{c(0, 0), c(10, 0), c(10, 10), c(0, 10)}, Ring{c(0, 0), c(2, 2), c(5, 2)}}))));

        // Java poly_multi*
        appendAll(generatePolyVariants(
            "poly_multi", feat(multiPoly(PolygonVec{RingVec{Ring{c1, c2, c3}}, RingVec{Ring{c21, c22, c23}}}))));

        // Java Morton polygon variants.
        Ring mortonCurve = buildMortonCurve(16, 8, 4);
        Ring mortonRing = mortonCurve;

        const auto mortonCfg = cfg([](auto& c) {
            c.useMortonEncoding = true;
            c.forceMortonGeometryLayout = true;
            c.geometryEncodingOption = IntegerEncodingOption::AUTO;
            c.geometryTopologyEncodingOption = IntegerEncodingOption::AUTO;
        });

        const auto nonMortonCfg = cfg([](auto& c) {
            c.useMortonEncoding = false;
            c.geometryEncodingOption = IntegerEncodingOption::AUTO;
            c.geometryTopologyEncodingOption = IntegerEncodingOption::AUTO;
        });

        tiles.push_back({
            .name = "poly_morton_ring_no_morton",
            .bytes = encode(layer(defaultLayerName, {feat(poly(std::move(mortonRing)))}, defaultExtent), nonMortonCfg),
        });

        mortonRing = mortonCurve;
        tiles.push_back({
            .name = "poly_morton_ring_morton",
            .bytes = encode(layer(defaultLayerName, {feat(poly(std::move(mortonRing)))}, defaultExtent), mortonCfg),
        });

        const std::size_t half = mortonCurve.size() / 2;
        Ring mortonRing1(mortonCurve.begin(), mortonCurve.begin() + static_cast<std::ptrdiff_t>(half));
        Ring mortonRing2(mortonCurve.begin() + static_cast<std::ptrdiff_t>(half), mortonCurve.end());

        tiles.push_back({
            .name = "poly_multi_morton_ring_no_morton",
            .bytes = encode(
                layer(defaultLayerName,
                      {feat(multiPoly(PolygonVec{RingVec{std::move(mortonRing1)}, RingVec{std::move(mortonRing2)}}))},
                      defaultExtent),
                nonMortonCfg),
        });

        mortonRing1 = Ring(mortonCurve.begin(), mortonCurve.begin() + static_cast<std::ptrdiff_t>(half));
        mortonRing2 = Ring(mortonCurve.begin() + static_cast<std::ptrdiff_t>(half), mortonCurve.end());

        tiles.push_back({
            .name = "poly_multi_morton_ring_morton",
            .bytes = encode(
                layer(defaultLayerName,
                      {feat(multiPoly(PolygonVec{RingVec{std::move(mortonRing1)}, RingVec{std::move(mortonRing2)}}))},
                      defaultExtent),
                mortonCfg),
        });

        const std::size_t quarter = mortonCurve.size() / 4;
        mortonRing1 = Ring(mortonCurve.begin(), mortonCurve.begin() + static_cast<std::ptrdiff_t>(quarter));
        mortonRing2 = Ring(mortonCurve.begin() + static_cast<std::ptrdiff_t>(quarter), mortonCurve.end());

        tiles.push_back({
            .name = "poly_morton_hole_morton",
            .bytes = encode(layer(defaultLayerName,
                                  {feat(poly(RingVec{std::move(mortonRing1), std::move(mortonRing2)}))},
                                  defaultExtent),
                            mortonCfg),
        });

        mortonRing1 = Ring(mortonCurve.begin(), mortonCurve.begin() + static_cast<std::ptrdiff_t>(quarter));
        mortonRing2 = Ring(mortonCurve.begin() + static_cast<std::ptrdiff_t>(quarter), mortonCurve.end());
        tiles.push_back({
            .name = "poly_multi_morton_hole_morton",
            .bytes = encode(
                layer(defaultLayerName,
                      {feat(multiPoly(PolygonVec{RingVec{std::move(mortonRing1), std::move(mortonRing2)}}))},
                      defaultExtent),
                mortonCfg),
        });

        return tiles;
    }

    static GeneratedTile generateMultiPoint() {
        Ring points{c1, c2, c3};
        return {
            .name = "multipoint",
            .bytes = encode(layer(defaultLayerName, {feat(multiPoint(points))}, defaultExtent), cfg()),
        };
    }

    static GeneratedTileVec generateMultiPoints() { return {generateMultiPoint()}; }

    static GeneratedTile generateMultiLine() {
        Ring line1_coords{c1, c2, c3};
        Ring line2_coords{c21, c22, c23};
        PartVec lines{line1_coords, line2_coords};
        const auto config = cfg([](auto& c) { c.geometryTopologyEncodingOption = IntegerEncodingOption::AUTO; });
        return {
            .name = "multiline",
            .bytes = encode(layer(defaultLayerName, {feat(multiLine(lines))}, defaultExtent), config),
        };
    }

    static GeneratedTileVec generateMultiLineStrings() { return {generateMultiLine()}; }

    static GeneratedTileVec generateMultiPointsMorton() {
        Ring mortonCurve = buildMortonCurve(16, 8, 4);
        const std::size_t half = mortonCurve.size() / 2;
        Ring mortonPts(mortonCurve.begin(), mortonCurve.begin() + static_cast<std::ptrdiff_t>(half));
        const auto config = cfg([](auto& c) {
            c.useMortonEncoding = true;
            c.forceMortonGeometryLayout = true;
        });
        return {{
            .name = "multipoint_morton",
            .bytes = encode(layer(defaultLayerName, {feat(multiPoint(std::move(mortonPts)))}, defaultExtent), config),
        }};
    }

    static GeneratedTileVec generateMultiLineStringsMorton() {
        Ring mortonCurve = buildMortonCurve(16, 8, 4);
        const std::size_t half = mortonCurve.size() / 2;
        Ring mortonLine1(mortonCurve.begin(), mortonCurve.begin() + static_cast<std::ptrdiff_t>(half));
        Ring mortonLine2(mortonCurve.begin() + static_cast<std::ptrdiff_t>(half), mortonCurve.end());
        const auto config = cfg([](auto& c) {
            c.useMortonEncoding = true;
            c.forceMortonGeometryLayout = true;
            c.geometryTopologyEncodingOption = IntegerEncodingOption::AUTO;
        });

        return {{
            .name = "multiline_morton",
            .bytes = encode(layer(defaultLayerName,
                                  {feat(multiLine(PartVec{std::move(mortonLine1), std::move(mortonLine2)}))},
                                  defaultExtent),
                            config),
        }};
    }

    static GeneratedTile generateExtent(std::uint32_t extent) {
        const Ring line_coords{c(0, 0),
                               c(static_cast<std::int32_t>(extent - 1), static_cast<std::int32_t>(extent - 1))};
        return {
            .name = "extent_" + std::to_string(extent),
            .bytes = encode(layer(defaultLayerName, {feat(line(line_coords))}, extent), cfg()),
        };
    }

    static GeneratedTile generateExtentBuf(std::uint32_t extent) {
        const Ring line_coords{c(-42, -42),
                               c(static_cast<std::int32_t>(extent + 42), static_cast<std::int32_t>(extent + 42))};
        return {
            .name = "extent_buf_" + std::to_string(extent),
            .bytes = encode(layer(defaultLayerName, {feat(line(line_coords))}, extent), cfg()),
        };
    }

    static GeneratedTileVec generateExtents() {
        GeneratedTileVec tiles;
        for (const auto e : {512U, 4096U, 131072U, 1073741824U}) {
            tiles.push_back(generateExtent(e));
            tiles.push_back(generateExtentBuf(e));
        }
        return tiles;
    }

    static GeneratedTile generateIds() {
        // Java writes: write("id", idFeat(100), cfg().ids());
        const auto config = cfg([](auto& c) {
            c.includeIds = true;
            c.integerEncodingOption = IntegerEncodingOption::PLAIN;
            c.geometryEncodingOption = IntegerEncodingOption::PLAIN;
        });

        return {
            .name = "id",
            .bytes = encode(layer(defaultLayerName, {featWithId(100, point(c0))}, defaultExtent), config),
        };
    }

    static GeneratedTile generateIdMin() {
        return {
            .name = "id_min",
            .bytes = encode(layer(defaultLayerName, {featWithId(0, point(c0))}, defaultExtent), cfg([](auto& c) {
                                c.includeIds = true;
                                c.integerEncodingOption = IntegerEncodingOption::PLAIN;
                                c.geometryEncodingOption = IntegerEncodingOption::PLAIN;
                            })),
        };
    }

    static GeneratedTile generateId64() {
        return {
            .name = "id64",
            .bytes = encode(layer(defaultLayerName, {featWithId(9234567890ULL, point(c0))}, defaultExtent),
                            cfg([](auto& c) {
                                c.includeIds = true;
                                c.integerEncodingOption = IntegerEncodingOption::PLAIN;
                                c.geometryEncodingOption = IntegerEncodingOption::PLAIN;
                            })),
        };
    }

    static GeneratedTile generateIdsWithEncoding(const std::string& name,
                                                 const std::vector<std::optional<std::uint64_t>>& ids,
                                                 IntegerEncodingOption integerEncodingOption) {
        std::vector<Feature> features;
        features.reserve(ids.size());
        for (const auto& id : ids) {
            if (id.has_value()) {
                features.push_back(featWithId(*id, point(c0)));
            } else {
                features.push_back(featWithoutId(point(c0)));
            }
        }

        const auto config = cfg([&](auto& c) {
            c.includeIds = true;
            c.integerEncodingOption = integerEncodingOption;
            c.geometryEncodingOption = IntegerEncodingOption::PLAIN;
            c.geometryTopologyEncodingOption = IntegerEncodingOption::AUTO;
        });

        return {
            .name = name,
            .bytes = encode(layer(defaultLayerName, std::move(features), defaultExtent), config),
        };
    }

    static GeneratedTile generateIdsSeries() {
        const std::vector<std::optional<std::uint64_t>> ids{103, 103, 103, 103};
        return generateIdsWithEncoding("ids", ids, IntegerEncodingOption::PLAIN);
    }

    static GeneratedTile generateIdsDelta() {
        const std::vector<std::optional<std::uint64_t>> ids{103, 103, 103, 103};
        return generateIdsWithEncoding("ids_delta", ids, IntegerEncodingOption::DELTA);
    }

    static GeneratedTile generateIdsRle() {
        const std::vector<std::optional<std::uint64_t>> ids{103, 103, 103, 103};
        return generateIdsWithEncoding("ids_rle", ids, IntegerEncodingOption::RLE);
    }

    static GeneratedTile generateIdsDeltaRle() {
        const std::vector<std::optional<std::uint64_t>> ids{103, 103, 103, 103};
        return generateIdsWithEncoding("ids_delta_rle", ids, IntegerEncodingOption::DELTA_RLE);
    }

    static GeneratedTile generateIds64() {
        const std::vector<std::optional<std::uint64_t>> ids{9234567890ULL, 9234567890ULL, 9234567890ULL, 9234567890ULL};
        return generateIdsWithEncoding("ids64", ids, IntegerEncodingOption::PLAIN);
    }

    static GeneratedTile generateIds64Delta() {
        const std::vector<std::optional<std::uint64_t>> ids{9234567890ULL, 9234567890ULL, 9234567890ULL, 9234567890ULL};
        return generateIdsWithEncoding("ids64_delta", ids, IntegerEncodingOption::DELTA);
    }

    static GeneratedTile generateIds64Rle() {
        const std::vector<std::optional<std::uint64_t>> ids{9234567890ULL, 9234567890ULL, 9234567890ULL, 9234567890ULL};
        return generateIdsWithEncoding("ids64_rle", ids, IntegerEncodingOption::RLE);
    }

    static GeneratedTile generateIds64DeltaRle() {
        const std::vector<std::optional<std::uint64_t>> ids{9234567890ULL, 9234567890ULL, 9234567890ULL, 9234567890ULL};
        return generateIdsWithEncoding("ids64_delta_rle", ids, IntegerEncodingOption::DELTA_RLE);
    }

    static GeneratedTile generateIdsOpt() {
        const std::vector<std::optional<std::uint64_t>> ids{100, 101, std::nullopt, 105, 106};
        return generateIdsWithEncoding("ids_opt", ids, IntegerEncodingOption::PLAIN);
    }

    static GeneratedTile generateIdsOptDelta() {
        const std::vector<std::optional<std::uint64_t>> ids{100, 101, std::nullopt, 105, 106};
        return generateIdsWithEncoding("ids_opt_delta", ids, IntegerEncodingOption::DELTA);
    }

    static GeneratedTile generateIds64Opt() {
        const std::vector<std::optional<std::uint64_t>> ids{std::nullopt, 9234567890ULL, 101, 105, 106};
        return generateIdsWithEncoding("ids64_opt", ids, IntegerEncodingOption::PLAIN);
    }

    static GeneratedTile generateIds64OptDelta() {
        const std::vector<std::optional<std::uint64_t>> ids{std::nullopt, 9234567890ULL, 101, 105, 106};
        return generateIdsWithEncoding("ids64_opt_delta", ids, IntegerEncodingOption::DELTA);
    }

    static GeneratedTileVec generateIdsCollection() {
        return {
            generateIds(),
            generateIdsSeries(),
            generateIdMin(),
            generateId64(),
            generateIdsDelta(),
            generateIdsRle(),
            generateIdsDeltaRle(),
            generateIds64(),
            generateIds64Delta(),
            generateIds64Rle(),
            generateIds64DeltaRle(),
            generateIdsOpt(),
            generateIdsOptDelta(),
            generateIds64Opt(),
            generateIds64OptDelta(),
        };
    }

    static GeneratedTile generatePropU64() {
        // Feature with a u64 property value
        PropertyMap props;
        props["bignum"] = PropertyValue{static_cast<std::uint64_t>(1234567890123456789ULL)};

        // Java writes a nullable column for this property even though no values are null.
        const auto config = cfg([](auto& c) { c.forceNullableColumns = true; });

        return {
            .name = "prop_u64",
            .bytes = encode(layer(defaultLayerName, {feat(point(c0), props)}, defaultExtent), config),
        };
    }

    static GeneratedTile generatePropsStrFsst() {
        // Java writes 6 features at p1,p2,p3,ph1,ph2,ph3 with these exact values.
        Feature f1 = feat(point(c1),
                          PropertyMap{{"val", PropertyValue{std::string("residential_zone_north_sector_1")}}});
        Feature f2 = feat(point(c2),
                          PropertyMap{{"val", PropertyValue{std::string("commercial_zone_south_sector_2")}}});
        Feature f3 = feat(point(c3), PropertyMap{{"val", PropertyValue{std::string("industrial_zone_east_sector_3")}}});
        Feature f4 = feat(point(h1), PropertyMap{{"val", PropertyValue{std::string("park_zone_west_sector_4")}}});
        Feature f5 = feat(point(h2), PropertyMap{{"val", PropertyValue{std::string("water_zone_north_sector_5")}}});
        Feature f6 = feat(point(h3),
                          PropertyMap{{"val", PropertyValue{std::string("residential_zone_south_sector_6")}}});
        return {
            .name = "props_str_fsst",
            .bytes = encode(layer(defaultLayerName, {f1, f2, f3, f4, f5, f6}, defaultExtent), cfgWithFsst()),
        };
    }

    static GeneratedTileVec generateProperties() { return {generatePropU64(), generatePropsStrFsst()}; }

    static GeneratedTileVec generateFpfAlignments() {
        GeneratedTileVec tiles;

        std::vector<Feature> features;
        features.reserve(128);
        for (std::uint32_t i = 0; i < 128; ++i) {
            features.push_back(feat(point(c0), PropertyMap{{"v", PropertyValue{static_cast<std::uint32_t>(i % 3)}}}));
        }

        for (std::uint32_t pad = 0; pad < 8; ++pad) {
            const auto config = cfg([](auto& c) {
                // Match Java synthetic generation: cfg().fastPFOR()
                c.integerEncodingOption = IntegerEncodingOption::PLAIN;
                c.geometryEncodingOption = IntegerEncodingOption::PLAIN;
                c.geometryTopologyEncodingOption = IntegerEncodingOption::AUTO;
                c.useFastPfor = true;
                c.forceNullableColumns = true;
            });
            tiles.push_back({
                .name = "fpf_align_" + std::to_string(pad + 1),
                .bytes = encode(layer(std::string(pad + 1, 'a'), features, defaultExtent), config),
            });
        }

        return tiles;
    }

    // Mix geometry types matching Java SyntheticMltGenerator
    static Feature mixPt() { return feat(point(c(38, 29))); }

    static Feature mixLine() {
        Ring coords{c(5, 38), c(12, 45), c(9, 70)};
        return feat(line(coords));
    }

    static Feature mixPoly() {
        Ring coords{c(55, 5), c(58, 28), c(75, 22)};
        return feat(poly(coords));
    }

    static Feature mixPolyh() {
        Ring shell{c(52, 35), c(14, 55), c(60, 72)};
        Ring hole{c(32, 50), c(36, 60), c(24, 54)};
        return feat(poly(RingVec{shell, hole}));
    }

    static Feature mixMpt() {
        Ring coords{c(6, 25), c(21, 41), c(23, 69)};
        return feat(multiPoint(coords));
    }

    static Feature mixMline() {
        Ring line1{c(24, 10), c(42, 18)};
        Ring line2{c(30, 36), c(48, 52), c(35, 62)};
        return feat(multiLine(PartVec{line1, line2}));
    }

    static Feature mixMpoly() {
        Ring poly1_shell{c(7, 20), c(21, 31), c(26, 9)};
        Ring poly1_hole{c(15, 20), c(20, 15), c(18, 25)};
        Ring poly2{c(69, 57), c(71, 66), c(73, 64)};
        return feat(multiPoly(PolygonVec{RingVec{poly1_shell, poly1_hole}, RingVec{poly2}}));
    }

    // Helper struct to represent geometry type with name and feature
    struct GeomTypeInfo {
        const char* sym;
        Feature (*getFeature)();
        metadata::tileset::GeometryType geomType;
    };

    static inline const GeomTypeInfo mixGeomTypes[] = {
        {.sym = "pt", .getFeature = mixPt, .geomType = metadata::tileset::GeometryType::POINT},
        {.sym = "line", .getFeature = mixLine, .geomType = metadata::tileset::GeometryType::LINESTRING},
        {.sym = "poly", .getFeature = mixPoly, .geomType = metadata::tileset::GeometryType::POLYGON},
        {.sym = "polyh", .getFeature = mixPolyh, .geomType = metadata::tileset::GeometryType::POLYGON},
        {.sym = "mpt", .getFeature = mixMpt, .geomType = metadata::tileset::GeometryType::MULTIPOINT},
        {.sym = "mline", .getFeature = mixMline, .geomType = metadata::tileset::GeometryType::MULTILINESTRING},
        {.sym = "mpoly", .getFeature = mixMpoly, .geomType = metadata::tileset::GeometryType::MULTIPOLYGON},
    };

    static constexpr std::size_t mixGeomTypeCount = 7;

    // Check if all geometries in a type list are polygons
    static bool allPolygonTypes(const std::vector<std::size_t>& typeIndices) {
        for (std::size_t idx : typeIndices) {
            if (mixGeomTypes[idx].geomType != metadata::tileset::GeometryType::POLYGON &&
                mixGeomTypes[idx].geomType != metadata::tileset::GeometryType::MULTIPOLYGON) {
                return false;
            }
        }
        return true;
    }

    // Generate a single mix combination
    static GeneratedTile generateMixCombination(const std::vector<std::size_t>& typeIndices) {
        std::vector<Feature> features;
        std::string name = "mix_" + std::to_string(typeIndices.size());

        for (std::size_t idx : typeIndices) {
            name += "_";
            name += mixGeomTypes[idx].sym;
            features.push_back(mixGeomTypes[idx].getFeature());
        }

        auto config = cfg();
        config.integerEncodingOption = IntegerEncodingOption::PLAIN;
        // config.geometryEncodingOption = IntegerEncodingOption::PLAIN;

        // Generate plain encoding
        GeneratedTile tile{
            .name = name,
            .bytes = encode(layer(defaultLayerName, features, defaultExtent), config),
        };

        return tile;
    }

    // Generate tessellated variant if all geometries are polygons
    static GeneratedTile generateMixCombinationTessellated(const std::vector<std::size_t>& typeIndices) {
        std::vector<Feature> features;
        std::string name = "mix_" + std::to_string(typeIndices.size());

        for (std::size_t idx : typeIndices) {
            name += "_";
            name += mixGeomTypes[idx].sym;
            features.push_back(mixGeomTypes[idx].getFeature());
        }

        name += "_tes";

        auto config = cfg();
        config.preTessellate = true;
        config.integerEncodingOption = IntegerEncodingOption::PLAIN;

        GeneratedTile tile{
            .name = name,
            .bytes = encode(layer(defaultLayerName, features, defaultExtent), config),
        };

        return tile;
    }

    // Generate all k-combinations of geometry types
    static void generateMixedCombinations(std::vector<GeneratedTile>& tiles,
                                          const std::vector<std::size_t>& typeIndices,
                                          std::vector<std::size_t>& current,
                                          std::size_t k,
                                          std::size_t start) {
        if (current.size() == k) {
            tiles.push_back(generateMixCombination(current));

            // Add tessellated variant if all types are polygons
            if (allPolygonTypes(current)) {
                tiles.push_back(generateMixCombinationTessellated(current));
            }
            return;
        }

        for (std::size_t i = start; i < typeIndices.size(); ++i) {
            // Skip duplicates at this level (avoid duplicate combinations)
            if (i > start && typeIndices[i] == typeIndices[i - 1]) {
                continue;
            }
            current.push_back(typeIndices[i]);
            generateMixedCombinations(tiles, typeIndices, current, k, i + 1);
            current.pop_back();
        }
    }

    static GeneratedTile generateMixPtLine() {
        // Mixed geometry: one point and one line (basic test case)
        return generateMixCombination({0, 1}); // pt, line
    }

    static GeneratedTileVec generateMixed() {
        std::vector<GeneratedTile> tiles;

        // Generate all k-combinations from k=2 to k=7
        std::vector<std::size_t> typeIndices{0, 1, 2, 3, 4, 5, 6}; // Indices for all 7 types
        for (std::size_t k = 2; k <= mixGeomTypeCount; ++k) {
            std::vector<std::size_t> current;
            generateMixedCombinations(tiles, typeIndices, current, k, 0);
        }

        // Generate A-A variants (same geometry twice)
        for (std::size_t i = 0; i < mixGeomTypeCount; ++i) {
            std::vector<std::size_t> combo{i, i};
            tiles.push_back(generateMixCombination(combo));
            // Add tessellated variant if all types are polygons
            if (allPolygonTypes(combo)) {
                tiles.push_back(generateMixCombinationTessellated(combo));
            }
        }

        // Generate A-B-A variants (geometry A, B, A)
        for (std::size_t a = 0; a < mixGeomTypeCount; ++a) {
            for (std::size_t b = 0; b < mixGeomTypeCount; ++b) {
                if (a != b) {
                    std::vector<std::size_t> combo{a, b, a};
                    tiles.push_back(generateMixCombination(combo));
                    if (allPolygonTypes(combo)) {
                        tiles.push_back(generateMixCombinationTessellated(combo));
                    }
                }
            }
        }

        return tiles;
    }

    static GeneratedTileVec generateSharedDictionaries() {
        const auto val = std::string(30, 'A');

        auto noSharedDict = GeneratedTile{
            .name = "props_no_shared_dict",
            .bytes = encode(
                layer(defaultLayerName,
                      {feat(point(c0), PropertyMap{{"name:en", PropertyValue{val}}, {"name:de", PropertyValue{val}}})},
                      defaultExtent),
                cfg([](auto& c) { c.forceNullableColumns = true; })),
        };

        auto sharedDict = GeneratedTile{
            .name = "props_shared_dict",
            .bytes = encode(
                layer(defaultLayerName,
                      {feat(point(c0), PropertyMap{{"name:", PropertyValue{StructValue{{"en", val}, {"de", val}}}}})},
                      defaultExtent),
                cfg()),
        };

        auto sharedDictFsst = GeneratedTile{
            .name = "props_shared_dict_fsst",
            .bytes = encode(
                layer(defaultLayerName,
                      {feat(point(c0), PropertyMap{{"name:", PropertyValue{StructValue{{"en", val}, {"de", val}}}}})},
                      defaultExtent),
                cfgWithFsst()),
        };

        auto sharedDictOneChild = GeneratedTile{
            .name = "props_shared_dict_one_child",
            .bytes = encode(layer(defaultLayerName,
                                  {feat(point(c0),
                                        PropertyMap{{"name:en", PropertyValue{StructValue{{"", val}}}},
                                                    {"place", PropertyValue{val}}})},
                                  defaultExtent),
                            cfg([](auto& c) { c.forceNullableColumns = true; })),
        };

        auto sharedDictOneChildFsst = GeneratedTile{
            .name = "props_shared_dict_one_child_fsst",
            .bytes = encode(layer(defaultLayerName,
                                  {feat(point(c0),
                                        PropertyMap{{"name:en", PropertyValue{StructValue{{"", val}}}},
                                                    {"place", PropertyValue{val}}})},
                                  defaultExtent),
                            cfg([](auto& c) {
                                c.forceNullableColumns = true;
                                c.useFsst = true;
                            })),
        };

        auto sharedDictNoStructName = GeneratedTile{
            .name = "props_shared_dict_no_struct_name",
            .bytes = encode(
                layer(defaultLayerName,
                      {feat(point(c0), PropertyMap{{"", PropertyValue{StructValue{{"a", val}, {"b", val}}}}})},
                      defaultExtent),
                cfg()),
        };

        auto sharedDictNoStructNameFsst = GeneratedTile{
            .name = "props_shared_dict_no_struct_name_fsst",
            .bytes = encode(
                layer(defaultLayerName,
                      {feat(point(c0), PropertyMap{{"", PropertyValue{StructValue{{"a", val}, {"b", val}}}}})},
                      defaultExtent),
                cfgWithFsst()),
        };

        auto sharedDictNoChildName = GeneratedTile{
            .name = "props_shared_dict_no_child_name",
            .bytes = encode(layer(defaultLayerName,
                                  {feat(point(c0), PropertyMap{{"a", PropertyValue{StructValue{{"", val}}}}})},
                                  defaultExtent),
                            cfg()),
        };

        auto sharedDictNoChildNameFsst = GeneratedTile{
            .name = "props_shared_dict_no_child_name_fsst",
            .bytes = encode(layer(defaultLayerName,
                                  {feat(point(c0), PropertyMap{{"a", PropertyValue{StructValue{{"", val}}}}})},
                                  defaultExtent),
                            cfgWithFsst()),
        };

        auto sharedDictTwoSamePrefix = GeneratedTile{
            .name = "props_shared_dict_2_same_prefix",
            .bytes = encode(
                layer(defaultLayerName,
                      {feat(point(c0),
                            PropertyMap{
                                {"name_group0", PropertyValue{StructValue{{"name:de", val}, {"name_en", val}}}},
                                {"name_group1", PropertyValue{StructValue{{"name:he", val}, {"name_fr", val}}}}})},
                      defaultExtent),
                cfg()),
        };

        return {
            std::move(noSharedDict),
            std::move(sharedDict),
            std::move(sharedDictFsst),
            std::move(sharedDictOneChild),
            std::move(sharedDictOneChildFsst),
            std::move(sharedDictNoStructName),
            std::move(sharedDictNoStructNameFsst),
            std::move(sharedDictNoChildName),
            std::move(sharedDictNoChildNameFsst),
            std::move(sharedDictTwoSamePrefix),
        };
    }
};

} // namespace mlt::test
