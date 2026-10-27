#include <gtest/gtest.h>

#include <mlt/coordinate.hpp>
#include <mlt/decoder.hpp>
#include <mlt/encode/winding.hpp>
#include <mlt/encoder.hpp>
#include <mlt/geometry.hpp>
#include <mlt/layer.hpp>
#include <mlt/metadata/tileset.hpp>
#include <mlt/tile.hpp>

#include <cstddef>
#include <cstdint>
#include <limits>
#include <utility>
#include <vector>

using namespace mlt;

using GeometryType = metadata::tileset::GeometryType;

namespace {

using TestRing = std::vector<std::pair<int, int>>;

// The decoder closes rings, but the encoder takes them open
TestRing openRing(const CoordVec& ring) {
    TestRing result;
    for (std::size_t i = 0; i + 1 < ring.size(); ++i) {
        result.emplace_back(static_cast<int>(ring[i].x), static_cast<int>(ring[i].y));
    }
    return result;
}

void appendRing(const TestRing& ring, std::vector<Encoder::Vertex>& vertices, std::vector<std::uint32_t>& ringSizes) {
    for (const auto& [x, y] : ring) {
        vertices.push_back({.x = x, .y = y});
    }
    ringSizes.push_back(static_cast<std::uint32_t>(ring.size()));
}

Encoder::Feature polygonFeature(const std::vector<TestRing>& rings) {
    Encoder::Feature f;
    f.geometry.type = GeometryType::POLYGON;
    for (const auto& ring : rings) {
        appendRing(ring, f.geometry.coordinates, f.geometry.ringSizes);
    }
    return f;
}

/// Encode a layer of the one feature and decode it again
MapLibreTile encodeDecode(const Encoder::Feature& feature, bool tessellate) {
    Encoder::Layer layer;
    layer.name = "winding";
    layer.features = {feature};

    EncoderConfig config;
    config.sortFeatures = false;
    config.preTessellate = tessellate;
    const auto bytes = Encoder().encode({layer}, config);
    EXPECT_FALSE(bytes.empty());
    return Decoder(/*supportFastPFOR=*/true).decode({reinterpret_cast<const char*>(bytes.data()), bytes.size()});
}

std::vector<TestRing> encodeDecodePolygon(const Encoder::Feature& feature, bool tessellate) {
    const auto tile = encodeDecode(feature, tessellate);
    const auto* decoded = tile.getLayer("winding");
    EXPECT_TRUE(decoded);
    if (!decoded) {
        return {};
    }
    std::vector<TestRing> result;
    for (const auto& ring :
         dynamic_cast<const geometry::Polygon&>(decoded->getFeatures()[0].getGeometry()).getRings()) {
        result.push_back(openRing(ring));
    }
    return result;
}

// Positive area by the surveyor's formula, i.e. clockwise with the y-axis pointing down
// NOLINTNEXTLINE(bugprone-throwing-static-initialization)
const TestRing mvtExterior{{0, 0}, {10, 0}, {10, 10}, {0, 10}};
// NOLINTNEXTLINE(bugprone-throwing-static-initialization)
const TestRing mvtHole{{2, 2}, {2, 8}, {8, 8}, {8, 2}};

TEST(Winding, KeepsRingsWoundAsMvt) {
    for (const bool tessellate : {false, true}) {
        EXPECT_EQ(encodeDecodePolygon(polygonFeature({mvtExterior, mvtHole}), tessellate),
                  (std::vector<TestRing>{mvtExterior, mvtHole}))
            << "tessellate=" << tessellate;
    }
}

TEST(Winding, ReversesExteriorAndHoleAgainstMvtKeepingTheirFirstVertex) {
    const TestRing exterior{{0, 0}, {0, 10}, {10, 10}, {10, 0}};
    const TestRing hole{{2, 2}, {8, 2}, {8, 8}, {2, 8}};
    for (const bool tessellate : {false, true}) {
        EXPECT_EQ(encodeDecodePolygon(polygonFeature({exterior, hole}), tessellate),
                  (std::vector<TestRing>{mvtExterior, mvtHole}))
            << "tessellate=" << tessellate;
    }
}

TEST(Winding, WindsEachRingOnItsOwn) {
    const TestRing hole{{2, 2}, {8, 2}, {8, 8}, {2, 8}};
    EXPECT_EQ(encodeDecodePolygon(polygonFeature({mvtExterior, hole}), false),
              (std::vector<TestRing>{mvtExterior, mvtHole}));
}

TEST(Winding, WindsEveryPolygonOfMultiPolygon) {
    Encoder::Feature f;
    f.geometry.type = GeometryType::MULTIPOLYGON;
    f.geometry.parts.resize(2);
    f.geometry.partRingSizes.resize(2);
    appendRing(mvtExterior, f.geometry.parts[0], f.geometry.partRingSizes[0]);
    appendRing({{20, 20}, {20, 30}, {30, 30}, {30, 20}}, f.geometry.parts[1], f.geometry.partRingSizes[1]);

    for (const bool tessellate : {false, true}) {
        const auto tile = encodeDecode(f, tessellate);
        const auto* decoded = tile.getLayer("winding");
        ASSERT_TRUE(decoded);
        const auto& polygons =
            dynamic_cast<const geometry::MultiPolygon&>(decoded->getFeatures()[0].getGeometry()).getPolygons();
        ASSERT_EQ(polygons.size(), 2u) << "tessellate=" << tessellate;
        EXPECT_EQ(openRing(polygons[0].at(0)), mvtExterior) << "tessellate=" << tessellate;
        EXPECT_EQ(openRing(polygons[1].at(0)), (TestRing{{20, 20}, {30, 20}, {30, 30}, {20, 30}}))
            << "tessellate=" << tessellate;
    }
}

TEST(Winding, KeepsRingWithoutArea) {
    const TestRing line{{0, 0}, {10, 0}, {20, 0}};
    EXPECT_EQ(encodeDecodePolygon(polygonFeature({line}), false), (std::vector<TestRing>{line}));
}

TEST(Winding, SignOfAreaIsExactForLargeCoordinates) {
    using Vertex = Encoder::Vertex;
    constexpr auto max = std::numeric_limits<std::int32_t>::max();
    constexpr auto min = std::numeric_limits<std::int32_t>::min();
    // The sum of these products does not fit in 64 bits
    const std::vector<Vertex> positive{
        {.x = min, .y = min}, {.x = max, .y = min}, {.x = max, .y = max}, {.x = min, .y = max}};
    const std::vector<Vertex> negative{
        {.x = min, .y = min}, {.x = min, .y = max}, {.x = max, .y = max}, {.x = max, .y = min}};
    EXPECT_EQ(encoder::PolygonWinding::areaSign<Vertex>(positive), 1);
    EXPECT_EQ(encoder::PolygonWinding::areaSign<Vertex>(negative), -1);
}

TEST(Winding, IgnoresRingSizesBeyondTheVertices) {
    std::vector<Encoder::Vertex> vertices{{.x = 0, .y = 0}, {.x = 0, .y = 10}, {.x = 10, .y = 10}};
    const auto original = vertices;
    const std::vector<std::uint32_t> ringSizes{4};
    encoder::PolygonWinding::wind<Encoder::Vertex>(vertices, ringSizes);
    EXPECT_EQ(vertices.size(), original.size());
    for (std::size_t i = 0; i < vertices.size(); ++i) {
        EXPECT_EQ(vertices[i].x, original[i].x);
        EXPECT_EQ(vertices[i].y, original[i].y);
    }
}

} // namespace
