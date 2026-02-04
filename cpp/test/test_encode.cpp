#include <gtest/gtest.h>

#include <mlt/decoder.hpp>
#include <mlt/encoder.hpp>
#include <mlt/metadata/stream.hpp>
#include <mlt/metadata/tileset.hpp>
#include <mlt/util/buffer_stream.hpp>
#include <mlt/util/encoding/varint.hpp>
#include <mlt/util/encoding/zigzag.hpp>
#include <mlt/util/varint.hpp>
#include <mlt/util/zigzag.hpp>

#include <cstdint>
#include <vector>

using namespace mlt;

// --- Encoding primitive roundtrip tests ---

TEST(EncodePrimitives, ZigZagRoundtrip) {
    for (std::int32_t v : {0, 1, -1, 42, -42, 127, -128, 65535, -65536, 2147483647, -2147483647}) {
        auto encoded = util::encoding::encodeZigZag(v);
        auto decoded = util::decoding::decodeZigZag(encoded);
        EXPECT_EQ(v, decoded) << "Failed for value " << v;
    }
    for (std::int64_t v : {0L, 1L, -1L, 42L, -42L, 4294967296L, -4294967296L}) {
        auto encoded = util::encoding::encodeZigZag(v);
        auto decoded = util::decoding::decodeZigZag(encoded);
        EXPECT_EQ(v, decoded) << "Failed for value " << v;
    }
}

TEST(EncodePrimitives, VarintRoundtrip) {
    for (std::uint32_t v : {0u, 1u, 127u, 128u, 16384u, 2097152u, 268435456u, 4294967295u}) {
        std::vector<std::uint8_t> buf;
        util::encoding::encodeVarint(v, buf);

        BufferStream stream({reinterpret_cast<const char*>(buf.data()), buf.size()});
        auto decoded = util::decoding::decodeVarint<std::uint32_t>(stream);
        EXPECT_EQ(v, decoded) << "Failed for value " << v;
        EXPECT_EQ(stream.getRemaining(), 0u);
    }
    for (std::uint64_t v : {0ULL, 1ULL, 127ULL, 128ULL, 0xFFFFFFFFULL, 0xFFFFFFFFFFFFFFFFULL}) {
        std::vector<std::uint8_t> buf;
        util::encoding::encodeVarint(v, buf);

        BufferStream stream({reinterpret_cast<const char*>(buf.data()), buf.size()});
        auto decoded = util::decoding::decodeVarint<std::uint64_t>(stream);
        EXPECT_EQ(v, decoded) << "Failed for value " << v;
        EXPECT_EQ(stream.getRemaining(), 0u);
    }
}

// --- Stream metadata roundtrip tests ---

TEST(EncodeMetadata, StreamMetadataRoundtrip) {
    using namespace metadata::stream;

    StreamMetadata original(
        PhysicalStreamType::DATA,
        LogicalStreamType{DictionaryType::SINGLE},
        LogicalLevelTechnique::DELTA,
        LogicalLevelTechnique::NONE,
        PhysicalLevelTechnique::VARINT,
        42, 100);

    auto encoded = original.encode();
    BufferStream stream({reinterpret_cast<const char*>(encoded.data()), encoded.size()});
    auto decoded = StreamMetadata::decode(stream);

    ASSERT_TRUE(decoded);
    EXPECT_EQ(decoded->getPhysicalStreamType(), PhysicalStreamType::DATA);
    EXPECT_EQ(decoded->getLogicalLevelTechnique1(), LogicalLevelTechnique::DELTA);
    EXPECT_EQ(decoded->getLogicalLevelTechnique2(), LogicalLevelTechnique::NONE);
    EXPECT_EQ(decoded->getPhysicalLevelTechnique(), PhysicalLevelTechnique::VARINT);
    EXPECT_EQ(decoded->getNumValues(), 42u);
    EXPECT_EQ(decoded->getByteLength(), 100u);
}

TEST(EncodeMetadata, RleStreamMetadataRoundtrip) {
    using namespace metadata::stream;

    RleEncodedStreamMetadata original(
        PhysicalStreamType::DATA, std::nullopt,
        LogicalLevelTechnique::RLE, LogicalLevelTechnique::NONE,
        PhysicalLevelTechnique::VARINT,
        10, 50, 3, 100);

    auto encoded = original.encode();
    BufferStream stream({reinterpret_cast<const char*>(encoded.data()), encoded.size()});
    auto decoded = StreamMetadata::decode(stream);

    ASSERT_TRUE(decoded);
    EXPECT_EQ(decoded->getMetadataType(), LogicalLevelTechnique::RLE);
    auto* rle = dynamic_cast<RleEncodedStreamMetadata*>(decoded.get());
    ASSERT_TRUE(rle);
    EXPECT_EQ(rle->getRuns(), 3u);
    EXPECT_EQ(rle->getNumRleValues(), 100u);
}

TEST(EncodeMetadata, FeatureTableRoundtrip) {
    using namespace metadata::tileset;

    FeatureTable table;
    table.name = "test_layer";
    table.extent = 4096;

    // ID column
    Column idCol;
    idCol.nullable = false;
    idCol.columnScope = ColumnScope::FEATURE;
    idCol.type = ScalarColumn{.type = LogicalScalarType::ID, .hasLongID = false};
    table.columns.push_back(std::move(idCol));

    // Geometry column
    Column geomCol;
    geomCol.nullable = false;
    geomCol.columnScope = ColumnScope::FEATURE;
    geomCol.type = ComplexColumn{.type = ComplexType::GEOMETRY};
    table.columns.push_back(std::move(geomCol));

    // Int property
    Column intCol;
    intCol.name = "population";
    intCol.nullable = true;
    intCol.columnScope = ColumnScope::FEATURE;
    intCol.type = ScalarColumn{.type = ScalarType::INT_32};
    table.columns.push_back(std::move(intCol));

    // String property
    Column strCol;
    strCol.name = "name";
    strCol.nullable = true;
    strCol.columnScope = ColumnScope::FEATURE;
    strCol.type = ScalarColumn{.type = ScalarType::STRING};
    table.columns.push_back(std::move(strCol));

    auto encoded = encodeFeatureTable(table);
    BufferStream stream({reinterpret_cast<const char*>(encoded.data()), encoded.size()});
    auto decoded = decodeFeatureTable(stream);

    EXPECT_EQ(decoded.name, "test_layer");
    EXPECT_EQ(decoded.extent, 4096u);
    ASSERT_EQ(decoded.columns.size(), 4u);

    EXPECT_TRUE(decoded.columns[0].isID());
    EXPECT_TRUE(decoded.columns[1].isGeometry());
    EXPECT_EQ(decoded.columns[2].name, "population");
    EXPECT_TRUE(decoded.columns[2].nullable);
    EXPECT_EQ(decoded.columns[2].getScalarType().getPhysicalType(), ScalarType::INT_32);
    EXPECT_EQ(decoded.columns[3].name, "name");
    EXPECT_TRUE(decoded.columns[3].nullable);
    EXPECT_EQ(decoded.columns[3].getScalarType().getPhysicalType(), ScalarType::STRING);
}

// --- Full encode→decode roundtrip tests ---

TEST(Encode, PointRoundtrip) {
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "layer";
    layer.extent = 4096;

    Encoder::Feature f;
    f.id = 1;
    f.geometry.type = metadata::tileset::GeometryType::POINT;
    f.geometry.coordinates = {{100, 200}};
    f.properties["flag"] = true;
    layer.features.push_back(std::move(f));

    auto tileData = encoder.encode({layer});
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("layer");
    ASSERT_TRUE(decoded);
    EXPECT_EQ(decoded->getName(), "layer");
    EXPECT_EQ(decoded->getExtent(), 4096u);
    ASSERT_EQ(decoded->getFeatures().size(), 1u);
    EXPECT_EQ(decoded->getFeatures()[0].getID(), 1u);
}

TEST(Encode, LineStringRoundtrip) {
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "roads";
    layer.extent = 4096;

    Encoder::Feature f;
    f.id = 42;
    f.geometry.type = metadata::tileset::GeometryType::LINESTRING;
    f.geometry.coordinates = {{0, 0}, {100, 100}, {200, 50}};
    f.properties["name"] = std::string("Main Street");
    f.properties["lanes"] = std::int32_t{4};
    layer.features.push_back(std::move(f));

    auto tileData = encoder.encode({layer});
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("roads");
    ASSERT_TRUE(decoded);
    ASSERT_EQ(decoded->getFeatures().size(), 1u);
    EXPECT_EQ(decoded->getFeatures()[0].getID(), 42u);
}

TEST(Encode, PolygonRoundtrip) {
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "buildings";
    layer.extent = 4096;

    Encoder::Feature f;
    f.id = 7;
    f.geometry.type = metadata::tileset::GeometryType::POLYGON;
    // Exterior ring (4 vertices, closing vertex omitted per MLT convention)
    f.geometry.coordinates = {{0, 0}, {100, 0}, {100, 100}, {0, 100}};
    f.geometry.ringSizes = {4};
    f.properties["height"] = 42.5f;
    layer.features.push_back(std::move(f));

    auto tileData = encoder.encode({layer});
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("buildings");
    ASSERT_TRUE(decoded);
    ASSERT_EQ(decoded->getFeatures().size(), 1u);
    EXPECT_EQ(decoded->getFeatures()[0].getID(), 7u);
}

TEST(Encode, MultipleFeatures) {
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "pois";
    layer.extent = 4096;

    for (int i = 0; i < 100; ++i) {
        Encoder::Feature f;
        f.id = i;
        f.geometry.type = metadata::tileset::GeometryType::POINT;
        f.geometry.coordinates = {{i * 10, i * 20}};
        f.properties["rank"] = std::int32_t{i};
        f.properties["name"] = std::string("POI #" + std::to_string(i));
        layer.features.push_back(std::move(f));
    }

    auto tileData = encoder.encode({layer});
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("pois");
    ASSERT_TRUE(decoded);
    EXPECT_EQ(decoded->getFeatures().size(), 100u);

    for (int i = 0; i < 100; ++i) {
        EXPECT_EQ(decoded->getFeatures()[i].getID(), static_cast<std::uint64_t>(i));
    }
}

TEST(Encode, MultipleLayers) {
    Encoder encoder;

    Encoder::Layer points;
    points.name = "points";
    points.extent = 4096;
    Encoder::Feature pf;
    pf.id = 1;
    pf.geometry.type = metadata::tileset::GeometryType::POINT;
    pf.geometry.coordinates = {{50, 50}};
    points.features.push_back(std::move(pf));

    Encoder::Layer lines;
    lines.name = "lines";
    lines.extent = 4096;
    Encoder::Feature lf;
    lf.id = 2;
    lf.geometry.type = metadata::tileset::GeometryType::LINESTRING;
    lf.geometry.coordinates = {{0, 0}, {100, 100}};
    lines.features.push_back(std::move(lf));

    auto tileData = encoder.encode({points, lines});
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    EXPECT_TRUE(tile.getLayer("points"));
    EXPECT_TRUE(tile.getLayer("lines"));
    EXPECT_EQ(tile.getLayer("points")->getFeatures().size(), 1u);
    EXPECT_EQ(tile.getLayer("lines")->getFeatures().size(), 1u);
}
