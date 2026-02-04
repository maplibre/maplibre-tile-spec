#include <gtest/gtest.h>

#include <mlt/decoder.hpp>
#include <mlt/encoder.hpp>
#include <mlt/geometry.hpp>
#include <mlt/metadata/stream.hpp>
#include <mlt/metadata/tileset.hpp>
#include <mlt/util/buffer_stream.hpp>
#include <mlt/util/encoding/varint.hpp>
#include <mlt/util/encoding/zigzag.hpp>
#include <mlt/util/varint.hpp>
#include <mlt/util/zigzag.hpp>

#include <cstdint>
#include <filesystem>
#include <fstream>
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

TEST(Encode, NullableIntProperty) {
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "test";
    layer.extent = 4096;

    {
        Encoder::Feature f;
        f.id = 1;
        f.geometry.type = metadata::tileset::GeometryType::POINT;
        f.geometry.coordinates = {{10, 20}};
        f.properties["pop"] = std::int32_t{100};
        layer.features.push_back(std::move(f));
    }
    {
        Encoder::Feature f;
        f.id = 2;
        f.geometry.type = metadata::tileset::GeometryType::POINT;
        f.geometry.coordinates = {{30, 40}};
        // missing "pop"
        layer.features.push_back(std::move(f));
    }
    {
        Encoder::Feature f;
        f.id = 3;
        f.geometry.type = metadata::tileset::GeometryType::POINT;
        f.geometry.coordinates = {{50, 60}};
        f.properties["pop"] = std::int32_t{200};
        layer.features.push_back(std::move(f));
    }

    auto tileData = encoder.encode({layer});
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("test");
    ASSERT_TRUE(decoded);
    ASSERT_EQ(decoded->getFeatures().size(), 3u);

    const auto& props = decoded->getProperties();
    EXPECT_TRUE(props.contains("pop"));

    const auto& popProp = props.at("pop");
    auto v0 = popProp.getProperty(0);
    auto v1 = popProp.getProperty(1);
    auto v2 = popProp.getProperty(2);
    ASSERT_TRUE(v0.has_value());
    EXPECT_EQ(std::get<std::int32_t>(*v0), 100);
    EXPECT_FALSE(v1.has_value());
    ASSERT_TRUE(v2.has_value());
    EXPECT_EQ(std::get<std::int32_t>(*v2), 200);
}

TEST(Encode, PropertyValueTypes) {
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "types";
    layer.extent = 4096;

    Encoder::Feature f;
    f.id = 1;
    f.geometry.type = metadata::tileset::GeometryType::POINT;
    f.geometry.coordinates = {{100, 200}};
    f.properties["bool_val"] = true;
    f.properties["int32_val"] = std::int32_t{-42};
    f.properties["int64_val"] = std::int64_t{9999999999LL};
    f.properties["float_val"] = 1.5f;
    f.properties["string_val"] = std::string("hello world");
    layer.features.push_back(std::move(f));

    auto tileData = encoder.encode({layer});
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("types");
    ASSERT_TRUE(decoded);
    ASSERT_EQ(decoded->getFeatures().size(), 1u);

    // Verify each property type was decoded
    const auto& props = decoded->getProperties();
    EXPECT_TRUE(props.contains("bool_val"));
    EXPECT_TRUE(props.contains("int32_val"));
    EXPECT_TRUE(props.contains("int64_val"));
    EXPECT_TRUE(props.contains("float_val"));
    EXPECT_TRUE(props.contains("string_val"));

    // Check int32 value
    const auto& intProp = props.at("int32_val");
    EXPECT_EQ(intProp.getType(), metadata::tileset::ScalarType::INT_32);
    auto intVal = intProp.getProperty(0);
    ASSERT_TRUE(intVal.has_value());
    EXPECT_EQ(std::get<std::int32_t>(*intVal), -42);

    // Check boolean value
    const auto& boolProp = props.at("bool_val");
    EXPECT_EQ(boolProp.getType(), metadata::tileset::ScalarType::BOOLEAN);
    auto boolVal = boolProp.getProperty(0);
    ASSERT_TRUE(boolVal.has_value());
    EXPECT_EQ(std::get<bool>(*boolVal), true);

    // Check int64 value
    const auto& longProp = props.at("int64_val");
    EXPECT_EQ(longProp.getType(), metadata::tileset::ScalarType::INT_64);

    // Check float value
    const auto& floatProp = props.at("float_val");
    auto floatVal = floatProp.getProperty(0);
    ASSERT_TRUE(floatVal.has_value());
    EXPECT_FLOAT_EQ(std::get<float>(*floatVal), 1.5f);

    // Check string value
    const auto& strProp = props.at("string_val");
    EXPECT_EQ(strProp.getType(), metadata::tileset::ScalarType::STRING);
    auto strVal = strProp.getProperty(0);
    ASSERT_TRUE(strVal.has_value());
    EXPECT_EQ(std::get<std::string_view>(*strVal), "hello world");
}

TEST(Encode, LargeIntegerEncoding) {
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "test";
    layer.extent = 4096;

    // Test delta encoding (sequential values)
    for (int i = 0; i < 50; ++i) {
        Encoder::Feature f;
        f.id = i;
        f.geometry.type = metadata::tileset::GeometryType::POINT;
        f.geometry.coordinates = {{i * 10, i * 10}};
        f.properties["seq"] = std::int32_t{i * 100};
        layer.features.push_back(std::move(f));
    }

    // Test RLE encoding (constant values)
    for (int i = 50; i < 100; ++i) {
        Encoder::Feature f;
        f.id = i;
        f.geometry.type = metadata::tileset::GeometryType::POINT;
        f.geometry.coordinates = {{i * 10, i * 10}};
        f.properties["seq"] = std::int32_t{999};
        layer.features.push_back(std::move(f));
    }

    auto tileData = encoder.encode({layer});
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("test");
    ASSERT_TRUE(decoded);
    ASSERT_EQ(decoded->getFeatures().size(), 100u);

    // Verify sequential IDs survived
    for (int i = 0; i < 100; ++i) {
        EXPECT_EQ(decoded->getFeatures()[i].getID(), static_cast<uint64_t>(i));
    }

    // Verify integer property values
    const auto& seqProp = decoded->getProperties().at("seq");
    for (int i = 0; i < 50; ++i) {
        auto val = seqProp.getProperty(i);
        ASSERT_TRUE(val.has_value()) << "Missing value at index " << i;
        EXPECT_EQ(std::get<std::int32_t>(*val), i * 100) << "Wrong value at index " << i;
    }
    for (int i = 50; i < 100; ++i) {
        auto val = seqProp.getProperty(i);
        ASSERT_TRUE(val.has_value()) << "Missing value at index " << i;
        EXPECT_EQ(std::get<std::int32_t>(*val), 999) << "Wrong value at index " << i;
    }
}

TEST(Encode, LongIdRoundtrip) {
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "longids";
    layer.extent = 4096;

    Encoder::Feature f;
    f.id = 0xFFFFFFFF00000001ULL;
    f.geometry.type = metadata::tileset::GeometryType::POINT;
    f.geometry.coordinates = {{10, 20}};
    layer.features.push_back(std::move(f));

    auto tileData = encoder.encode({layer});
    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("longids");
    ASSERT_TRUE(decoded);
    ASSERT_EQ(decoded->getFeatures().size(), 1u);
    EXPECT_EQ(decoded->getFeatures()[0].getID(), 0xFFFFFFFF00000001ULL);
}

TEST(Encode, MultiPointRoundtrip) {
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "layer";
    layer.extent = 4096;

    Encoder::Feature f;
    f.id = 1;
    f.geometry.type = metadata::tileset::GeometryType::MULTIPOINT;
    f.geometry.coordinates = {{100, 200}, {300, 400}};
    f.properties["key"] = true;
    layer.features.push_back(std::move(f));

    auto tileData = encoder.encode({layer});
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("layer");
    ASSERT_TRUE(decoded);
    ASSERT_EQ(decoded->getFeatures().size(), 1u);
    EXPECT_EQ(decoded->getFeatures()[0].getID(), 1u);

    const auto& geom = decoded->getFeatures()[0].getGeometry();
    EXPECT_EQ(geom.type, metadata::tileset::GeometryType::MULTIPOINT);
    const auto& mp = dynamic_cast<const geometry::MultiPoint&>(geom);
    ASSERT_EQ(mp.getCoordinates().size(), 2u);
    EXPECT_EQ(mp.getCoordinates()[0].x, 100.0f);
    EXPECT_EQ(mp.getCoordinates()[0].y, 200.0f);
    EXPECT_EQ(mp.getCoordinates()[1].x, 300.0f);
    EXPECT_EQ(mp.getCoordinates()[1].y, 400.0f);
}

TEST(Encode, MultiLineStringRoundtrip) {
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "layer";
    layer.extent = 4096;

    Encoder::Feature f;
    f.id = 1;
    f.geometry.type = metadata::tileset::GeometryType::MULTILINESTRING;
    f.geometry.parts = {
        {{0, 0}, {100, 100}, {200, 50}},
        {{300, 300}, {400, 200}},
    };
    f.properties["key"] = true;
    layer.features.push_back(std::move(f));

    auto tileData = encoder.encode({layer});
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("layer");
    ASSERT_TRUE(decoded);
    ASSERT_EQ(decoded->getFeatures().size(), 1u);

    const auto& geom = decoded->getFeatures()[0].getGeometry();
    EXPECT_EQ(geom.type, metadata::tileset::GeometryType::MULTILINESTRING);
    const auto& mls = dynamic_cast<const geometry::MultiLineString&>(geom);
    ASSERT_EQ(mls.getLineStrings().size(), 2u);
    EXPECT_EQ(mls.getLineStrings()[0].size(), 3u);
    EXPECT_EQ(mls.getLineStrings()[1].size(), 2u);
}

TEST(Encode, PolygonWithHoleRoundtrip) {
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "layer";
    layer.extent = 4096;

    Encoder::Feature f;
    f.id = 1;
    f.geometry.type = metadata::tileset::GeometryType::POLYGON;
    // Exterior ring + interior hole (closing vertex omitted)
    f.geometry.coordinates = {
        {0, 0}, {1000, 0}, {1000, 1000}, {0, 1000},
        {200, 200}, {800, 200}, {800, 800}, {200, 800},
    };
    f.geometry.ringSizes = {4, 4};
    layer.features.push_back(std::move(f));

    auto tileData = encoder.encode({layer});
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("layer");
    ASSERT_TRUE(decoded);
    ASSERT_EQ(decoded->getFeatures().size(), 1u);

    const auto& geom = decoded->getFeatures()[0].getGeometry();
    EXPECT_EQ(geom.type, metadata::tileset::GeometryType::POLYGON);
    const auto& poly = dynamic_cast<const geometry::Polygon&>(geom);
    ASSERT_EQ(poly.getRings().size(), 2u);
    EXPECT_EQ(poly.getRings()[0].size(), 4u);
    EXPECT_EQ(poly.getRings()[1].size(), 4u);
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

// --- Cross-validation: decode Java-generated fixtures ---

namespace {
std::vector<char> loadFile(const std::filesystem::path& path) {
    std::ifstream file(path, std::ios::binary | std::ios::ate);
    if (!file.is_open()) return {};
    const auto size = file.tellg();
    file.seekg(0);
    std::vector<char> buffer(size);
    file.read(buffer.data(), size);
    return buffer;
}

// Try multiple base paths since tests may run from different directories
std::vector<char> loadFixture(const std::string& relativePath) {
    for (const auto& base : {"../test/expected/tag0x01/",
                              "../../test/expected/tag0x01/",
                              "../../../test/expected/tag0x01/",
                              "test/expected/tag0x01/"}) {
        auto data = loadFile(std::string(base) + relativePath);
        if (!data.empty()) return data;
    }
    return {};
}
}

TEST(CrossValidate, JavaPointBoolean) {
    auto fixture = loadFixture("simple/point-boolean.mlt");
    if (fixture.empty()) {
        GTEST_SKIP() << "Fixture not found";
    }

    // Decode the Java-generated fixture
    auto javaTile = Decoder().decode({fixture.data(), fixture.size()});
    const auto* javaLayer = javaTile.getLayer("layer");
    ASSERT_TRUE(javaLayer);
    EXPECT_EQ(javaLayer->getExtent(), 4096u);
    ASSERT_EQ(javaLayer->getFeatures().size(), 1u);
    EXPECT_EQ(javaLayer->getFeatures()[0].getID(), 1u);

    // Verify geometry type
    const auto& geom = javaLayer->getFeatures()[0].getGeometry();
    EXPECT_EQ(geom.type, metadata::tileset::GeometryType::POINT);

    // Verify property
    const auto& props = javaLayer->getProperties();
    EXPECT_TRUE(props.contains("key"));
    auto keyVal = props.at("key").getProperty(0);
    ASSERT_TRUE(keyVal.has_value());
    EXPECT_EQ(std::get<bool>(*keyVal), true);
}

TEST(CrossValidate, JavaLineBoolean) {
    auto fixture = loadFixture("simple/line-boolean.mlt");
    if (fixture.empty()) {
        GTEST_SKIP() << "Fixture not found";
    }

    auto javaTile = Decoder().decode({fixture.data(), fixture.size()});
    const auto* javaLayer = javaTile.getLayer("layer");
    ASSERT_TRUE(javaLayer);
    ASSERT_EQ(javaLayer->getFeatures().size(), 1u);

    const auto& geom = javaLayer->getFeatures()[0].getGeometry();
    EXPECT_EQ(geom.type, metadata::tileset::GeometryType::LINESTRING);
    const auto& ls = dynamic_cast<const geometry::LineString&>(geom);
    EXPECT_EQ(ls.getCoordinates().size(), 3u);
}

TEST(CrossValidate, JavaPolygonBoolean) {
    auto fixture = loadFixture("simple/polygon-boolean.mlt");
    if (fixture.empty()) {
        GTEST_SKIP() << "Fixture not found";
    }

    auto javaTile = Decoder().decode({fixture.data(), fixture.size()});
    const auto* javaLayer = javaTile.getLayer("layer");
    ASSERT_TRUE(javaLayer);
    ASSERT_EQ(javaLayer->getFeatures().size(), 1u);

    const auto& geom = javaLayer->getFeatures()[0].getGeometry();
    EXPECT_EQ(geom.type, metadata::tileset::GeometryType::POLYGON);
    const auto& poly = dynamic_cast<const geometry::Polygon&>(geom);
    EXPECT_EQ(poly.getRings().size(), 1u);
    EXPECT_EQ(poly.getRings()[0].size(), 3u); // 3 unique vertices (closing vertex omitted)
}

TEST(CrossValidate, JavaMultiPointBoolean) {
    auto fixture = loadFixture("simple/multipoint-boolean.mlt");
    if (fixture.empty()) {
        GTEST_SKIP() << "Fixture not found";
    }

    auto javaTile = Decoder().decode({fixture.data(), fixture.size()});
    const auto* javaLayer = javaTile.getLayer("layer");
    ASSERT_TRUE(javaLayer);
    ASSERT_EQ(javaLayer->getFeatures().size(), 1u);

    const auto& geom = javaLayer->getFeatures()[0].getGeometry();
    EXPECT_EQ(geom.type, metadata::tileset::GeometryType::MULTIPOINT);
    const auto& mp = dynamic_cast<const geometry::MultiPoint&>(geom);
    EXPECT_EQ(mp.getCoordinates().size(), 2u);
}

TEST(CrossValidate, JavaMultiLineBoolean) {
    auto fixture = loadFixture("simple/multiline-boolean.mlt");
    if (fixture.empty()) {
        GTEST_SKIP() << "Fixture not found";
    }

    auto javaTile = Decoder().decode({fixture.data(), fixture.size()});
    const auto* javaLayer = javaTile.getLayer("layer");
    ASSERT_TRUE(javaLayer);
    ASSERT_EQ(javaLayer->getFeatures().size(), 1u);

    const auto& geom = javaLayer->getFeatures()[0].getGeometry();
    EXPECT_EQ(geom.type, metadata::tileset::GeometryType::MULTILINESTRING);
    const auto& mls = dynamic_cast<const geometry::MultiLineString&>(geom);
    EXPECT_EQ(mls.getLineStrings().size(), 2u);
    EXPECT_EQ(mls.getLineStrings()[0].size(), 3u);
    EXPECT_EQ(mls.getLineStrings()[1].size(), 2u);
}

TEST(CrossValidate, JavaMultiPolygonBoolean) {
    auto fixture = loadFixture("simple/multipolygon-boolean.mlt");
    if (fixture.empty()) {
        GTEST_SKIP() << "Fixture not found";
    }

    auto javaTile = Decoder().decode({fixture.data(), fixture.size()});
    const auto* javaLayer = javaTile.getLayer("layer");
    ASSERT_TRUE(javaLayer);
    ASSERT_EQ(javaLayer->getFeatures().size(), 1u);

    const auto& geom = javaLayer->getFeatures()[0].getGeometry();
    EXPECT_EQ(geom.type, metadata::tileset::GeometryType::MULTIPOLYGON);
    const auto& mpoly = dynamic_cast<const geometry::MultiPolygon&>(geom);
    EXPECT_EQ(mpoly.getPolygons().size(), 2u);
    EXPECT_EQ(mpoly.getPolygons()[0].size(), 1u); // first polygon: 1 ring
    EXPECT_EQ(mpoly.getPolygons()[1].size(), 2u); // second polygon: 2 rings (exterior + hole)
}
