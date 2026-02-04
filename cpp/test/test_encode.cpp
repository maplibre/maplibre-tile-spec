#include <gtest/gtest.h>

#include <mlt/decoder.hpp>
#include <mlt/encoder.hpp>
#include <mlt/geometry.hpp>
#include <mlt/metadata/stream.hpp>
#include <mlt/metadata/tileset.hpp>
#include <mlt/util/buffer_stream.hpp>
#include <mlt/util/encoding/fsst.hpp>
#include <mlt/util/encoding/varint.hpp>
#include <mlt/util/encoding/zigzag.hpp>
#include <mlt/util/varint.hpp>
#include <mlt/util/zigzag.hpp>

#include <mlt/decode/string.hpp>
#include <mlt/util/hilbert_curve.hpp>

#include <cstdint>
#include <filesystem>
#include <fstream>
#include <vector>

using namespace mlt;

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

TEST(EncodeMetadata, StreamMetadataRoundtrip) {
    using namespace metadata::stream;

    StreamMetadata original(PhysicalStreamType::DATA,
                            LogicalStreamType{DictionaryType::SINGLE},
                            LogicalLevelTechnique::DELTA,
                            LogicalLevelTechnique::NONE,
                            PhysicalLevelTechnique::VARINT,
                            42,
                            100);

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

    RleEncodedStreamMetadata original(PhysicalStreamType::DATA,
                                      std::nullopt,
                                      LogicalLevelTechnique::RLE,
                                      LogicalLevelTechnique::NONE,
                                      PhysicalLevelTechnique::VARINT,
                                      10,
                                      50,
                                      3,
                                      100);

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

    Column idCol;
    idCol.nullable = false;
    idCol.columnScope = ColumnScope::FEATURE;
    idCol.type = ScalarColumn{.type = LogicalScalarType::ID, .hasLongID = false};
    table.columns.push_back(std::move(idCol));

    Column geomCol;
    geomCol.nullable = false;
    geomCol.columnScope = ColumnScope::FEATURE;
    geomCol.type = ComplexColumn{.type = ComplexType::GEOMETRY};
    table.columns.push_back(std::move(geomCol));

    Column intCol;
    intCol.name = "population";
    intCol.nullable = true;
    intCol.columnScope = ColumnScope::FEATURE;
    intCol.type = ScalarColumn{.type = ScalarType::INT_32};
    table.columns.push_back(std::move(intCol));

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

    std::set<std::uint64_t> decodedIds;
    for (const auto& f : decoded->getFeatures()) {
        decodedIds.insert(f.getID());
    }
    for (int i = 0; i < 100; ++i) {
        EXPECT_TRUE(decodedIds.count(i)) << "missing feature id " << i;
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

    const auto& props = decoded->getProperties();
    EXPECT_TRUE(props.contains("bool_val"));
    EXPECT_TRUE(props.contains("int32_val"));
    EXPECT_TRUE(props.contains("int64_val"));
    EXPECT_TRUE(props.contains("float_val"));
    EXPECT_TRUE(props.contains("string_val"));

    const auto& intProp = props.at("int32_val");
    EXPECT_EQ(intProp.getType(), metadata::tileset::ScalarType::INT_32);
    auto intVal = intProp.getProperty(0);
    ASSERT_TRUE(intVal.has_value());
    EXPECT_EQ(std::get<std::int32_t>(*intVal), -42);

    const auto& boolProp = props.at("bool_val");
    EXPECT_EQ(boolProp.getType(), metadata::tileset::ScalarType::BOOLEAN);
    auto boolVal = boolProp.getProperty(0);
    ASSERT_TRUE(boolVal.has_value());
    EXPECT_EQ(std::get<bool>(*boolVal), true);

    const auto& longProp = props.at("int64_val");
    EXPECT_EQ(longProp.getType(), metadata::tileset::ScalarType::INT_64);

    const auto& floatProp = props.at("float_val");
    auto floatVal = floatProp.getProperty(0);
    ASSERT_TRUE(floatVal.has_value());
    EXPECT_FLOAT_EQ(std::get<float>(*floatVal), 1.5f);

    const auto& strProp = props.at("string_val");
    EXPECT_EQ(strProp.getType(), metadata::tileset::ScalarType::STRING);
    auto strVal = strProp.getProperty(0);
    ASSERT_TRUE(strVal.has_value());
    EXPECT_EQ(std::get<std::string_view>(*strVal), "hello world");
}

namespace {
template <typename T>
T unwrapProperty(const Property& prop) {
    return std::visit(
        [](const auto& v) -> T {
            using V = std::decay_t<decltype(v)>;
            if constexpr (std::is_same_v<V, T>) {
                return v;
            } else if constexpr (std::is_same_v<V, std::optional<T>>) {
                return v.value();
            } else {
                throw std::bad_variant_access();
            }
        },
        prop);
}
} // namespace

TEST(Encode, AllPropertyTypes) {
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "all_types";
    layer.extent = 4096;

    for (int i = 0; i < 10; ++i) {
        Encoder::Feature f;
        f.id = i;
        f.geometry.type = metadata::tileset::GeometryType::POINT;
        f.geometry.coordinates = {{i * 100, i * 100}};
        f.properties["bool_val"] = (i % 2 == 0);
        f.properties["int32_val"] = std::int32_t{-100 + i * 20};
        f.properties["int64_val"] = std::int64_t{-9999999999LL + i};
        f.properties["uint32_val"] = std::uint32_t(3000000000u + i);
        f.properties["uint64_val"] = std::uint64_t(18000000000000000000ULL + i);
        f.properties["float_val"] = float(i) * 0.5f;
        f.properties["double_val"] = double(i) * 0.5;
        f.properties["string_val"] = std::string("str_") + std::to_string(i);
        layer.features.push_back(std::move(f));
    }

    auto tileData = encoder.encode({layer});
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("all_types");
    ASSERT_TRUE(decoded);
    ASSERT_EQ(decoded->getFeatures().size(), 10u);

    const auto& props = decoded->getProperties();
    for (int i = 0; i < 10; ++i) {
        auto boolVal = props.at("bool_val").getProperty(i);
        ASSERT_TRUE(boolVal.has_value());
        EXPECT_EQ(unwrapProperty<bool>(*boolVal), (i % 2 == 0));

        auto i32Val = props.at("int32_val").getProperty(i);
        ASSERT_TRUE(i32Val.has_value());
        EXPECT_EQ(unwrapProperty<std::int32_t>(*i32Val), -100 + i * 20);

        auto i64Val = props.at("int64_val").getProperty(i);
        ASSERT_TRUE(i64Val.has_value());
        EXPECT_EQ(unwrapProperty<std::int64_t>(*i64Val), -9999999999LL + i);

        auto u32Val = props.at("uint32_val").getProperty(i);
        ASSERT_TRUE(u32Val.has_value());
        EXPECT_EQ(unwrapProperty<std::uint32_t>(*u32Val), 3000000000u + i);

        auto u64Val = props.at("uint64_val").getProperty(i);
        ASSERT_TRUE(u64Val.has_value());
        EXPECT_EQ(unwrapProperty<std::uint64_t>(*u64Val), 18000000000000000000ULL + i);

        auto fVal = props.at("float_val").getProperty(i);
        ASSERT_TRUE(fVal.has_value());
        EXPECT_FLOAT_EQ(unwrapProperty<float>(*fVal), float(i) * 0.5f);

        auto dVal = props.at("double_val").getProperty(i);
        ASSERT_TRUE(dVal.has_value());
        EXPECT_FLOAT_EQ(unwrapProperty<float>(*dVal), float(i) * 0.5f);

        auto sVal = props.at("string_val").getProperty(i);
        ASSERT_TRUE(sVal.has_value());
        EXPECT_EQ(std::get<std::string_view>(*sVal), std::string("str_") + std::to_string(i));
    }
}

TEST(Encode, NullableAllTypes) {
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "nullable";
    layer.extent = 4096;

    for (int i = 0; i < 6; ++i) {
        Encoder::Feature f;
        f.id = i;
        f.geometry.type = metadata::tileset::GeometryType::POINT;
        f.geometry.coordinates = {{i * 100, i * 100}};
        if (i % 2 == 0) {
            f.properties["int32_val"] = std::int32_t{i};
            f.properties["int64_val"] = std::int64_t{i};
            f.properties["uint32_val"] = std::uint32_t(i);
            f.properties["uint64_val"] = std::uint64_t(i);
            f.properties["float_val"] = float(i);
            f.properties["double_val"] = double(i);
            f.properties["bool_val"] = true;
        }
        layer.features.push_back(std::move(f));
    }

    auto tileData = encoder.encode({layer});
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("nullable");
    ASSERT_TRUE(decoded);
    ASSERT_EQ(decoded->getFeatures().size(), 6u);

    for (const auto& [name, pp] : decoded->getProperties()) {
        for (int i = 0; i < 6; ++i) {
            auto val = pp.getProperty(i);
            if (i % 2 == 0) {
                EXPECT_TRUE(val.has_value()) << name << " at " << i << " should be present";
            } else {
                EXPECT_FALSE(val.has_value()) << name << " at " << i << " should be null";
            }
        }
    }
}

TEST(Encode, EmptyLayer) {
    Encoder encoder;

    Encoder::Layer empty;
    empty.name = "empty";
    empty.extent = 4096;

    Encoder::Layer nonempty;
    nonempty.name = "nonempty";
    nonempty.extent = 4096;
    Encoder::Feature f;
    f.id = 1;
    f.geometry.type = metadata::tileset::GeometryType::POINT;
    f.geometry.coordinates = {{50, 50}};
    nonempty.features.push_back(std::move(f));

    auto tileData = encoder.encode({empty, nonempty});
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    EXPECT_FALSE(tile.getLayer("empty"));
    EXPECT_TRUE(tile.getLayer("nonempty"));
}

TEST(Encode, SingleVertexLineString) {
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "degenerate";
    layer.extent = 4096;

    Encoder::Feature f;
    f.id = 1;
    f.geometry.type = metadata::tileset::GeometryType::LINESTRING;
    f.geometry.coordinates = {{100, 200}};
    layer.features.push_back(std::move(f));

    auto tileData = encoder.encode({layer});
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("degenerate");
    ASSERT_TRUE(decoded);
    ASSERT_EQ(decoded->getFeatures().size(), 1u);
    const auto& ls = dynamic_cast<const geometry::LineString&>(decoded->getFeatures()[0].getGeometry());
    EXPECT_EQ(ls.getCoordinates().size(), 1u);
}

TEST(Encode, BoundaryCoordinates) {
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "boundary";
    layer.extent = 4096;

    std::vector<std::pair<std::int32_t, std::int32_t>> coords = {
        {0, 0},
        {4096, 4096},
        {-4096, -4096},
        {4096, 0},
        {0, 4096},
    };

    for (std::size_t i = 0; i < coords.size(); ++i) {
        Encoder::Feature f;
        f.id = i;
        f.geometry.type = metadata::tileset::GeometryType::POINT;
        f.geometry.coordinates = {{coords[i].first, coords[i].second}};
        layer.features.push_back(std::move(f));
    }

    EncoderConfig config;
    config.sortFeatures = false;
    auto tileData = encoder.encode({layer}, config);
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("boundary");
    ASSERT_TRUE(decoded);
    ASSERT_EQ(decoded->getFeatures().size(), coords.size());

    for (std::size_t i = 0; i < coords.size(); ++i) {
        const auto& pt = dynamic_cast<const geometry::Point&>(decoded->getFeatures()[i].getGeometry());
        EXPECT_FLOAT_EQ(pt.getCoordinate().x, static_cast<float>(coords[i].first));
        EXPECT_FLOAT_EQ(pt.getCoordinate().y, static_cast<float>(coords[i].second));
    }
}

TEST(Encode, MaxUint64Id) {
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "big_ids";
    layer.extent = 4096;

    std::vector<std::uint64_t> testIds = {
        0,
        1,
        std::numeric_limits<std::uint32_t>::max(),
        static_cast<std::uint64_t>(std::numeric_limits<std::uint32_t>::max()) + 1,
        std::numeric_limits<std::uint64_t>::max() / 2,
    };

    for (auto id : testIds) {
        Encoder::Feature f;
        f.id = id;
        f.geometry.type = metadata::tileset::GeometryType::POINT;
        f.geometry.coordinates = {{50, 50}};
        layer.features.push_back(std::move(f));
    }

    auto tileData = encoder.encode({layer});
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("big_ids");
    ASSERT_TRUE(decoded);
    ASSERT_EQ(decoded->getFeatures().size(), testIds.size());

    for (std::size_t i = 0; i < testIds.size(); ++i) {
        EXPECT_EQ(decoded->getFeatures()[i].getID(), testIds[i]);
    }
}

TEST(Encode, LongStrings) {
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "strings";
    layer.extent = 4096;

    for (int i = 0; i < 10; ++i) {
        Encoder::Feature f;
        f.id = i;
        f.geometry.type = metadata::tileset::GeometryType::POINT;
        f.geometry.coordinates = {{i, i}};
        f.properties["long_str"] = std::string(10000 + i * 1000, 'a' + (i % 26));
        f.properties["unicode_str"] = std::string("Ünïcödé_τεστ_") + std::to_string(i) + "_日本語";
        layer.features.push_back(std::move(f));
    }

    auto tileData = encoder.encode({layer});
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("strings");
    ASSERT_TRUE(decoded);
    ASSERT_EQ(decoded->getFeatures().size(), 10u);

    const auto& longProp = decoded->getProperties().at("long_str");
    for (int i = 0; i < 10; ++i) {
        auto val = longProp.getProperty(i);
        ASSERT_TRUE(val.has_value());
        auto sv = std::get<std::string_view>(*val);
        EXPECT_EQ(sv.size(), 10000u + i * 1000u);
        EXPECT_EQ(sv[0], 'a' + (i % 26));
    }

    const auto& uniProp = decoded->getProperties().at("unicode_str");
    for (int i = 0; i < 10; ++i) {
        auto val = uniProp.getProperty(i);
        ASSERT_TRUE(val.has_value());
        auto expected = std::string("Ünïcödé_τεστ_") + std::to_string(i) + "_日本語";
        EXPECT_EQ(std::get<std::string_view>(*val), expected);
    }
}

TEST(Encode, DegeneratePolygon) {
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "degenerate_poly";
    layer.extent = 4096;

    Encoder::Feature f;
    f.id = 1;
    f.geometry.type = metadata::tileset::GeometryType::POLYGON;
    f.geometry.coordinates = {{0, 0}, {100, 0}, {100, 100}};
    f.geometry.ringSizes = {3};
    layer.features.push_back(std::move(f));

    auto tileData = encoder.encode({layer});
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("degenerate_poly");
    ASSERT_TRUE(decoded);
    ASSERT_EQ(decoded->getFeatures().size(), 1u);
    const auto& poly = dynamic_cast<const geometry::Polygon&>(decoded->getFeatures()[0].getGeometry());
    ASSERT_EQ(poly.getRings().size(), 1u);
    EXPECT_GE(poly.getRings()[0].size(), 3u);
}

TEST(Encode, ManyFeatures) {
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "many";
    layer.extent = 4096;

    constexpr int N = 10000;
    for (int i = 0; i < N; ++i) {
        Encoder::Feature f;
        f.id = i;
        f.geometry.type = metadata::tileset::GeometryType::POINT;
        f.geometry.coordinates = {{i % 4096, i / 4096}};
        f.properties["idx"] = std::int32_t{i};
        layer.features.push_back(std::move(f));
    }

    EncoderConfig config;
    config.sortFeatures = false;
    auto tileData = encoder.encode({layer}, config);
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("many");
    ASSERT_TRUE(decoded);
    ASSERT_EQ(decoded->getFeatures().size(), N);

    const auto& idxProp = decoded->getProperties().at("idx");
    for (int i = 0; i < N; ++i) {
        auto val = idxProp.getProperty(i);
        ASSERT_TRUE(val.has_value());
        EXPECT_EQ(unwrapProperty<std::int32_t>(*val), i);
    }
}

TEST(Encode, MultiPolygonManyParts) {
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "multi_many";
    layer.extent = 4096;

    Encoder::Feature f;
    f.id = 1;
    f.geometry.type = metadata::tileset::GeometryType::MULTIPOLYGON;
    for (int p = 0; p < 20; ++p) {
        int ox = (p % 5) * 800;
        int oy = (p / 5) * 800;
        f.geometry.parts.push_back({{ox, oy}, {ox + 100, oy}, {ox + 100, oy + 100}, {ox, oy + 100}});
        f.geometry.partRingSizes.push_back({4});
    }
    layer.features.push_back(std::move(f));

    auto tileData = encoder.encode({layer});
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("multi_many");
    ASSERT_TRUE(decoded);
    ASSERT_EQ(decoded->getFeatures().size(), 1u);
    const auto& mp = dynamic_cast<const geometry::MultiPolygon&>(decoded->getFeatures()[0].getGeometry());
    EXPECT_EQ(mp.getPolygons().size(), 20u);
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

    for (int i = 0; i < 100; ++i) {
        EXPECT_EQ(decoded->getFeatures()[i].getID(), static_cast<uint64_t>(i));
    }

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
        {0, 0},
        {1000, 0},
        {1000, 1000},
        {0, 1000},
        {200, 200},
        {800, 200},
        {800, 800},
        {200, 800},
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
    EXPECT_EQ(poly.getRings()[0].size(), 5u);
    EXPECT_EQ(poly.getRings()[1].size(), 5u);
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
} // namespace

Encoder::Vertex toEncVertex(const Coordinate& c) {
    return {static_cast<std::int32_t>(c.x), static_cast<std::int32_t>(c.y)};
}

Encoder::Layer decodedToEncoderLayer(const Layer& decoded) {
    Encoder::Layer layer;
    layer.name = decoded.getName();
    layer.extent = decoded.getExtent();

    const auto& props = decoded.getProperties();
    std::vector<std::string> propNames;
    for (const auto& [name, _] : props) {
        propNames.push_back(name);
    }

    for (std::size_t fi = 0; fi < decoded.getFeatures().size(); ++fi) {
        const auto& feat = decoded.getFeatures()[fi];
        Encoder::Feature ef;
        ef.id = feat.getID();

        const auto& geom = feat.getGeometry();
        ef.geometry.type = geom.type;

        switch (geom.type) {
            case metadata::tileset::GeometryType::POINT: {
                const auto& pt = dynamic_cast<const geometry::Point&>(geom);
                ef.geometry.coordinates = {toEncVertex(pt.getCoordinate())};
                break;
            }
            case metadata::tileset::GeometryType::LINESTRING: {
                const auto& ls = dynamic_cast<const geometry::LineString&>(geom);
                for (const auto& c : ls.getCoordinates()) {
                    ef.geometry.coordinates.push_back(toEncVertex(c));
                }
                break;
            }
            case metadata::tileset::GeometryType::POLYGON: {
                const auto& poly = dynamic_cast<const geometry::Polygon&>(geom);
                for (const auto& ring : poly.getRings()) {
                    auto count = ring.size();
                    if (count > 1 && ring.front() == ring.back()) --count;
                    ef.geometry.ringSizes.push_back(static_cast<std::uint32_t>(count));
                    for (std::size_t j = 0; j < count; ++j) {
                        ef.geometry.coordinates.push_back(toEncVertex(ring[j]));
                    }
                }
                break;
            }
            case metadata::tileset::GeometryType::MULTIPOINT: {
                const auto& mp = dynamic_cast<const geometry::MultiPoint&>(geom);
                for (const auto& c : mp.getCoordinates()) {
                    ef.geometry.coordinates.push_back(toEncVertex(c));
                }
                break;
            }
            case metadata::tileset::GeometryType::MULTILINESTRING: {
                const auto& mls = dynamic_cast<const geometry::MultiLineString&>(geom);
                for (const auto& ls : mls.getLineStrings()) {
                    std::vector<Encoder::Vertex> part;
                    for (const auto& c : ls) {
                        part.push_back(toEncVertex(c));
                    }
                    ef.geometry.parts.push_back(std::move(part));
                }
                break;
            }
            case metadata::tileset::GeometryType::MULTIPOLYGON: {
                const auto& mpoly = dynamic_cast<const geometry::MultiPolygon&>(geom);
                for (const auto& polygon : mpoly.getPolygons()) {
                    std::vector<Encoder::Vertex> partVerts;
                    std::vector<std::uint32_t> ringSizes;
                    for (const auto& ring : polygon) {
                        auto count = ring.size();
                        if (count > 1 && ring.front() == ring.back()) --count;
                        ringSizes.push_back(static_cast<std::uint32_t>(count));
                        for (std::size_t j = 0; j < count; ++j) {
                            partVerts.push_back(toEncVertex(ring[j]));
                        }
                    }
                    ef.geometry.parts.push_back(std::move(partVerts));
                    ef.geometry.partRingSizes.push_back(std::move(ringSizes));
                }
                break;
            }
        }

        for (const auto& name : propNames) {
            const auto& pp = props.at(name);
            auto val = pp.getProperty(static_cast<std::uint32_t>(fi));
            if (!val.has_value()) continue;
            std::visit(
                [&](const auto& v) {
                    using T = std::decay_t<decltype(v)>;
                    if constexpr (std::is_same_v<T, std::nullptr_t>) {
                    } else if constexpr (std::is_same_v<T, std::string_view>) {
                        ef.properties[name] = std::string(v);
                    } else if constexpr (std::is_same_v<T, std::optional<bool>>) {
                        if (v) ef.properties[name] = *v;
                    } else if constexpr (std::is_same_v<T, std::optional<std::int32_t>>) {
                        if (v) ef.properties[name] = *v;
                    } else if constexpr (std::is_same_v<T, std::optional<std::int64_t>>) {
                        if (v) ef.properties[name] = *v;
                    } else if constexpr (std::is_same_v<T, std::optional<std::uint32_t>>) {
                        if (v) ef.properties[name] = *v;
                    } else if constexpr (std::is_same_v<T, std::optional<std::uint64_t>>) {
                        if (v) ef.properties[name] = *v;
                    } else if constexpr (std::is_same_v<T, std::optional<float>>) {
                        if (v) ef.properties[name] = *v;
                    } else if constexpr (std::is_same_v<T, std::optional<double>>) {
                        if (v) ef.properties[name] = *v;
                    } else {
                        ef.properties[name] = v;
                    }
                },
                *val);
        }

        layer.features.push_back(std::move(ef));
    }
    return layer;
}

void compareDecodedTiles(const Layer& a, const Layer& b, bool sortedByEncoder) {
    ASSERT_EQ(a.getName(), b.getName());
    ASSERT_EQ(a.getExtent(), b.getExtent());
    ASSERT_EQ(a.getFeatures().size(), b.getFeatures().size());

    std::map<std::uint64_t, std::size_t> bById;
    bool hasDuplicateIds = false;
    for (std::size_t i = 0; i < b.getFeatures().size(); ++i) {
        auto [_, inserted] = bById.try_emplace(b.getFeatures()[i].getID(), i);
        if (!inserted) hasDuplicateIds = true;
    }

    for (std::size_t ai = 0; ai < a.getFeatures().size(); ++ai) {
        const auto& fa = a.getFeatures()[ai];
        std::size_t bi;
        if (hasDuplicateIds || sortedByEncoder) {
            bi = ai;
        } else {
            auto it = bById.find(fa.getID());
            ASSERT_TRUE(it != bById.end()) << "missing feature id " << fa.getID();
            bi = it->second;
        }
        const auto& fb = b.getFeatures()[bi];

        ASSERT_EQ(fa.getGeometry().type, fb.getGeometry().type) << "geometry type mismatch for id=" << fa.getID();

        switch (fa.getGeometry().type) {
            case metadata::tileset::GeometryType::POINT: {
                const auto& pa = dynamic_cast<const geometry::Point&>(fa.getGeometry());
                const auto& pb = dynamic_cast<const geometry::Point&>(fb.getGeometry());
                EXPECT_FLOAT_EQ(pa.getCoordinate().x, pb.getCoordinate().x);
                EXPECT_FLOAT_EQ(pa.getCoordinate().y, pb.getCoordinate().y);
                break;
            }
            case metadata::tileset::GeometryType::LINESTRING: {
                const auto& la = dynamic_cast<const geometry::LineString&>(fa.getGeometry());
                const auto& lb = dynamic_cast<const geometry::LineString&>(fb.getGeometry());
                ASSERT_EQ(la.getCoordinates().size(), lb.getCoordinates().size());
                for (std::size_t j = 0; j < la.getCoordinates().size(); ++j) {
                    EXPECT_FLOAT_EQ(la.getCoordinates()[j].x, lb.getCoordinates()[j].x);
                    EXPECT_FLOAT_EQ(la.getCoordinates()[j].y, lb.getCoordinates()[j].y);
                }
                break;
            }
            case metadata::tileset::GeometryType::POLYGON: {
                const auto& pa = dynamic_cast<const geometry::Polygon&>(fa.getGeometry());
                const auto& pb = dynamic_cast<const geometry::Polygon&>(fb.getGeometry());
                ASSERT_EQ(pa.getRings().size(), pb.getRings().size());
                for (std::size_t r = 0; r < pa.getRings().size(); ++r) {
                    ASSERT_EQ(pa.getRings()[r].size(), pb.getRings()[r].size());
                    auto count = pa.getRings()[r].size();
                    if (count > 1 && pa.getRings()[r].front() == pa.getRings()[r].back()) --count;
                    for (std::size_t j = 0; j < count; ++j) {
                        EXPECT_FLOAT_EQ(pa.getRings()[r][j].x, pb.getRings()[r][j].x);
                        EXPECT_FLOAT_EQ(pa.getRings()[r][j].y, pb.getRings()[r][j].y);
                    }
                }
                break;
            }
            default:
                break;
        }

        for (const auto& [name, ppA] : a.getProperties()) {
            ASSERT_TRUE(b.getProperties().contains(name)) << "missing property " << name;
            const auto& ppB = b.getProperties().at(name);
            auto valA = ppA.getProperty(static_cast<std::uint32_t>(ai));
            auto valB = ppB.getProperty(static_cast<std::uint32_t>(bi));
            EXPECT_EQ(valA.has_value(), valB.has_value())
                << "property " << name << " presence mismatch for id=" << fa.getID();
            if (valA.has_value() && valB.has_value()) {
                EXPECT_EQ(valA->index(), valB->index())
                    << "property " << name << " type mismatch for id=" << fa.getID();
            }
        }
    }
}

TEST(CrossValidate, JavaPointBoolean) {
    auto fixture = loadFixture("simple/point-boolean.mlt");
    if (fixture.empty()) {
        GTEST_SKIP() << "Fixture not found";
    }

    auto javaTile = Decoder().decode({fixture.data(), fixture.size()});
    const auto* javaLayer = javaTile.getLayer("layer");
    ASSERT_TRUE(javaLayer);
    EXPECT_EQ(javaLayer->getExtent(), 4096u);
    ASSERT_EQ(javaLayer->getFeatures().size(), 1u);
    EXPECT_EQ(javaLayer->getFeatures()[0].getID(), 1u);

    const auto& geom = javaLayer->getFeatures()[0].getGeometry();
    EXPECT_EQ(geom.type, metadata::tileset::GeometryType::POINT);

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
    EXPECT_EQ(poly.getRings()[0].size(), 4u);
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

TEST(CrossValidate, RoundtripPointBoolean) {
    auto fixture = loadFixture("simple/point-boolean.mlt");
    if (fixture.empty()) GTEST_SKIP() << "Fixture not found";

    auto javaTile = Decoder().decode({fixture.data(), fixture.size()});
    const auto* javaLayer = javaTile.getLayer("layer");
    ASSERT_TRUE(javaLayer);

    auto encLayer = decodedToEncoderLayer(*javaLayer);
    Encoder encoder;
    auto reencoded = encoder.encode({encLayer});
    ASSERT_FALSE(reencoded.empty());

    auto cppTile = Decoder().decode({reinterpret_cast<const char*>(reencoded.data()), reencoded.size()});
    const auto* cppLayer = cppTile.getLayer("layer");
    ASSERT_TRUE(cppLayer);
    compareDecodedTiles(*javaLayer, *cppLayer, true);
}

TEST(CrossValidate, RoundtripLineBoolean) {
    auto fixture = loadFixture("simple/line-boolean.mlt");
    if (fixture.empty()) GTEST_SKIP() << "Fixture not found";

    auto javaTile = Decoder().decode({fixture.data(), fixture.size()});
    const auto* javaLayer = javaTile.getLayer("layer");
    ASSERT_TRUE(javaLayer);

    auto encLayer = decodedToEncoderLayer(*javaLayer);
    Encoder encoder;
    auto reencoded = encoder.encode({encLayer});
    ASSERT_FALSE(reencoded.empty());

    auto cppTile = Decoder().decode({reinterpret_cast<const char*>(reencoded.data()), reencoded.size()});
    const auto* cppLayer = cppTile.getLayer("layer");
    ASSERT_TRUE(cppLayer);
    compareDecodedTiles(*javaLayer, *cppLayer, true);
}

TEST(CrossValidate, RoundtripPolygonBoolean) {
    auto fixture = loadFixture("simple/polygon-boolean.mlt");
    if (fixture.empty()) GTEST_SKIP() << "Fixture not found";

    auto javaTile = Decoder().decode({fixture.data(), fixture.size()});
    const auto* javaLayer = javaTile.getLayer("layer");
    ASSERT_TRUE(javaLayer);

    auto encLayer = decodedToEncoderLayer(*javaLayer);
    Encoder encoder;
    auto reencoded = encoder.encode({encLayer});
    ASSERT_FALSE(reencoded.empty());

    auto cppTile = Decoder().decode({reinterpret_cast<const char*>(reencoded.data()), reencoded.size()});
    const auto* cppLayer = cppTile.getLayer("layer");
    ASSERT_TRUE(cppLayer);
    compareDecodedTiles(*javaLayer, *cppLayer, true);
}

TEST(CrossValidate, RoundtripMultiPointBoolean) {
    auto fixture = loadFixture("simple/multipoint-boolean.mlt");
    if (fixture.empty()) GTEST_SKIP() << "Fixture not found";

    auto javaTile = Decoder().decode({fixture.data(), fixture.size()});
    const auto* javaLayer = javaTile.getLayer("layer");
    ASSERT_TRUE(javaLayer);

    auto encLayer = decodedToEncoderLayer(*javaLayer);
    Encoder encoder;
    auto reencoded = encoder.encode({encLayer});
    ASSERT_FALSE(reencoded.empty());

    auto cppTile = Decoder().decode({reinterpret_cast<const char*>(reencoded.data()), reencoded.size()});
    const auto* cppLayer = cppTile.getLayer("layer");
    ASSERT_TRUE(cppLayer);
    compareDecodedTiles(*javaLayer, *cppLayer, false);
}

TEST(CrossValidate, RoundtripMultiLineBoolean) {
    auto fixture = loadFixture("simple/multiline-boolean.mlt");
    if (fixture.empty()) GTEST_SKIP() << "Fixture not found";

    auto javaTile = Decoder().decode({fixture.data(), fixture.size()});
    const auto* javaLayer = javaTile.getLayer("layer");
    ASSERT_TRUE(javaLayer);

    auto encLayer = decodedToEncoderLayer(*javaLayer);
    Encoder encoder;
    auto reencoded = encoder.encode({encLayer});
    ASSERT_FALSE(reencoded.empty());

    auto cppTile = Decoder().decode({reinterpret_cast<const char*>(reencoded.data()), reencoded.size()});
    const auto* cppLayer = cppTile.getLayer("layer");
    ASSERT_TRUE(cppLayer);
    compareDecodedTiles(*javaLayer, *cppLayer, false);
}

TEST(CrossValidate, RoundtripMultiPolygonBoolean) {
    auto fixture = loadFixture("simple/multipolygon-boolean.mlt");
    if (fixture.empty()) GTEST_SKIP() << "Fixture not found";

    auto javaTile = Decoder().decode({fixture.data(), fixture.size()});
    const auto* javaLayer = javaTile.getLayer("layer");
    ASSERT_TRUE(javaLayer);

    auto encLayer = decodedToEncoderLayer(*javaLayer);
    Encoder encoder;
    auto reencoded = encoder.encode({encLayer});
    ASSERT_FALSE(reencoded.empty());

    auto cppTile = Decoder().decode({reinterpret_cast<const char*>(reencoded.data()), reencoded.size()});
    const auto* cppLayer = cppTile.getLayer("layer");
    ASSERT_TRUE(cppLayer);
    compareDecodedTiles(*javaLayer, *cppLayer, false);
}

void byteCompareFixtureTest(const std::string& fixturePath) {
    auto fixture = loadFixture(fixturePath);
    if (fixture.empty()) GTEST_SKIP() << "Fixture not found: " << fixturePath;

    auto javaTile = Decoder().decode({fixture.data(), fixture.size()});

    Encoder encoder;
    std::vector<Encoder::Layer> encoderLayers;
    for (const auto& layer : javaTile.getLayers()) {
        encoderLayers.push_back(decodedToEncoderLayer(layer));
    }

    EncoderConfig config;
    config.sortFeatures = false;
    auto reencoded = encoder.encode(encoderLayers, config);

    EXPECT_LE(reencoded.size(), fixture.size()) << "C++ encoder output larger than Java for " << fixturePath;

    auto cppTile = Decoder().decode({reinterpret_cast<const char*>(reencoded.data()), reencoded.size()});
    for (const auto& javaLayer : javaTile.getLayers()) {
        const auto* cppLayer = cppTile.getLayer(javaLayer.getName());
        ASSERT_TRUE(cppLayer);
        compareDecodedTiles(javaLayer, *cppLayer, true);
    }
}

TEST(ByteCompare, PointBoolean) {
    byteCompareFixtureTest("simple/point-boolean.mlt");
}
TEST(ByteCompare, LineBoolean) {
    byteCompareFixtureTest("simple/line-boolean.mlt");
}
TEST(ByteCompare, PolygonBoolean) {
    byteCompareFixtureTest("simple/polygon-boolean.mlt");
}
TEST(ByteCompare, MultiPointBoolean) {
    byteCompareFixtureTest("simple/multipoint-boolean.mlt");
}
TEST(ByteCompare, MultiLineBoolean) {
    byteCompareFixtureTest("simple/multiline-boolean.mlt");
}
TEST(ByteCompare, MultiPolygonBoolean) {
    byteCompareFixtureTest("simple/multipolygon-boolean.mlt");
}

TEST(FSST, EncodeDecodeRoundtrip) {
    std::string input = "AAAAAAABBBAAACCdddddEEEEEEfffEEEEAAAAAddddCC";
    std::vector<std::uint8_t> data(input.begin(), input.end());

    auto result = mlt::util::encoding::fsst::encode(data);

    EXPECT_FALSE(result.symbols.empty());
    EXPECT_FALSE(result.symbolLengths.empty());
    EXPECT_FALSE(result.compressedData.empty());
    EXPECT_EQ(result.decompressedLength, data.size());
    EXPECT_LT(result.compressedData.size(), data.size());

    auto decoded = mlt::decoder::StringDecoder::decodeFSST(
        result.symbols, result.symbolLengths, result.compressedData, data.size());

    EXPECT_EQ(decoded.size(), data.size());
    EXPECT_EQ(0, memcmp(data.data(), decoded.data(), data.size()));
}

TEST(FSST, EncodeDecodeRealisticStrings) {
    std::vector<std::string> strings;
    for (int i = 0; i < 100; ++i) {
        strings.push_back("residential");
        strings.push_back("secondary");
        strings.push_back("tertiary");
        strings.push_back("primary");
        strings.push_back("unclassified");
        strings.push_back("service");
        strings.push_back("footway");
        strings.push_back("track");
        strings.push_back("path");
        strings.push_back("cycleway");
    }

    std::vector<std::uint8_t> joined;
    for (const auto& s : strings) {
        joined.insert(joined.end(), s.begin(), s.end());
    }

    auto result = mlt::util::encoding::fsst::encode(joined);

    EXPECT_LT(result.compressedData.size(), joined.size());

    auto decoded = mlt::decoder::StringDecoder::decodeFSST(
        result.symbols, result.symbolLengths, result.compressedData, joined.size());

    EXPECT_EQ(decoded.size(), joined.size());
    EXPECT_EQ(0, memcmp(joined.data(), decoded.data(), joined.size()));
}

TEST(Encode, FsstStringRoundtrip) {
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "roads";
    layer.extent = 4096;

    std::vector<std::string> roadTypes = {"residential",
                                          "secondary",
                                          "tertiary",
                                          "primary",
                                          "unclassified",
                                          "service",
                                          "footway",
                                          "track",
                                          "path",
                                          "cycleway"};

    for (int i = 0; i < 200; ++i) {
        Encoder::Feature f;
        f.id = i;
        f.geometry.type = metadata::tileset::GeometryType::LINESTRING;
        f.geometry.coordinates = {{i * 10, i * 10}, {i * 10 + 100, i * 10 + 100}};
        f.properties["highway"] = roadTypes[i % roadTypes.size()];
        f.properties["name"] = std::string("Road ") + std::to_string(i);
        layer.features.push_back(std::move(f));
    }

    auto tileData = encoder.encode({layer});
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("roads");
    ASSERT_TRUE(decoded);
    ASSERT_EQ(decoded->getFeatures().size(), 200u);

    const auto& props = decoded->getProperties();
    ASSERT_TRUE(props.contains("highway"));
    ASSERT_TRUE(props.contains("name"));

    for (int i = 0; i < 200; ++i) {
        auto highway = props.at("highway").getProperty(i);
        ASSERT_TRUE(highway.has_value());
        auto sv = std::get<std::string_view>(*highway);
        EXPECT_EQ(sv, roadTypes[i % roadTypes.size()]) << "Mismatch at feature " << i;

        auto name = props.at("name").getProperty(i);
        ASSERT_TRUE(name.has_value());
        auto nameSv = std::get<std::string_view>(*name);
        EXPECT_EQ(nameSv, std::string("Road ") + std::to_string(i)) << "Mismatch at feature " << i;
    }
}

TEST(HilbertCurve, JavaCrossValidation) {
    using mlt::util::HilbertCurve;

    struct TestCase {
        std::uint32_t bits, x, y;
        std::uint32_t expected;
    };

    const TestCase cases[] = {
        {2, 0, 0, 0},
        {2, 1, 0, 1},
        {2, 1, 1, 2},
        {2, 0, 1, 3},
        {2, 0, 2, 4},
        {2, 0, 3, 5},
        {2, 1, 3, 6},
        {2, 1, 2, 7},
        {2, 2, 2, 8},
        {2, 2, 3, 9},
        {2, 3, 3, 10},
        {2, 3, 2, 11},
        {2, 3, 1, 12},
        {2, 2, 1, 13},
        {2, 2, 0, 14},
        {2, 3, 0, 15},
        {3, 0, 0, 0},
        {3, 7, 7, 42},
        {3, 4, 4, 32},
        {3, 3, 3, 10},
        {3, 1, 6, 23},
        {3, 5, 2, 55},
        {5, 0, 0, 0},
        {5, 31, 31, 682},
        {5, 16, 16, 512},
        {5, 3, 4, 31},
        {5, 10, 20, 476},
        {5, 25, 7, 982},
        {13, 0, 0, 0},
        {13, 4095, 4095, 11184810},
        {13, 2048, 2048, 8388608},
        {13, 100, 200, 52442},
        {13, 3000, 1000, 4889386},
        {13, 500, 4000, 16519952},
        {14, 0, 0, 0},
        {14, 8191, 8191, 44739242},
        {14, 4096, 4096, 33554432},
        {14, 1000, 2000, 3147584},
    };

    for (const auto& tc : cases) {
        auto actual = HilbertCurve::xy2d(tc.bits, tc.x, tc.y);
        EXPECT_EQ(actual, tc.expected) << "bits=" << tc.bits << " x=" << tc.x << " y=" << tc.y;
    }
}

TEST(HilbertCurve, RoundtripThroughSpaceFillingCurve) {
    mlt::util::HilbertCurve curve(0, 4095);

    for (int x = 0; x < 4096; x += 512) {
        for (int y = 0; y < 4096; y += 512) {
            auto d = curve.encode({static_cast<float>(x), static_cast<float>(y)});
            auto pt = curve.decode(d);
            EXPECT_EQ(static_cast<int>(pt.x), x) << "x=" << x << " y=" << y;
            EXPECT_EQ(static_cast<int>(pt.y), y) << "x=" << x << " y=" << y;
        }
    }
}

TEST(Encode, VertexDictionaryRoundtrip) {
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "dense";
    layer.extent = 4096;

    std::vector<Encoder::Vertex> sharedVerts = {
        {100, 200},
        {300, 400},
        {500, 600},
        {700, 800},
        {900, 1000},
        {1200, 1400},
        {1600, 1800},
        {2000, 2200},
        {2500, 2800},
        {3000, 3200},
    };

    for (int i = 0; i < 200; ++i) {
        Encoder::Feature f;
        f.id = i;
        f.geometry.type = metadata::tileset::GeometryType::LINESTRING;
        auto& v1 = sharedVerts[i % sharedVerts.size()];
        auto& v2 = sharedVerts[(i + 1) % sharedVerts.size()];
        auto& v3 = sharedVerts[(i + 2) % sharedVerts.size()];
        f.geometry.coordinates = {v1, v2, v3};
        layer.features.push_back(std::move(f));
    }

    auto tileData = encoder.encode({layer});
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("dense");
    ASSERT_TRUE(decoded);
    ASSERT_EQ(decoded->getFeatures().size(), 200u);

    std::map<std::uint64_t, const mlt::Feature*> byId;
    for (const auto& f : decoded->getFeatures()) {
        byId[f.getID()] = &f;
    }
    for (int i = 0; i < 200; ++i) {
        auto it = byId.find(i);
        ASSERT_TRUE(it != byId.end()) << "missing feature id " << i;
        const auto& geom = it->second->getGeometry();
        ASSERT_EQ(geom.type, metadata::tileset::GeometryType::LINESTRING);
        const auto& ls = dynamic_cast<const geometry::LineString&>(geom);
        ASSERT_EQ(ls.getCoordinates().size(), 3u);
        auto& v1 = sharedVerts[i % sharedVerts.size()];
        auto& v2 = sharedVerts[(i + 1) % sharedVerts.size()];
        auto& v3 = sharedVerts[(i + 2) % sharedVerts.size()];
        EXPECT_EQ(static_cast<int>(ls.getCoordinates()[0].x), v1.x) << "feature " << i;
        EXPECT_EQ(static_cast<int>(ls.getCoordinates()[0].y), v1.y) << "feature " << i;
        EXPECT_EQ(static_cast<int>(ls.getCoordinates()[1].x), v2.x) << "feature " << i;
        EXPECT_EQ(static_cast<int>(ls.getCoordinates()[1].y), v2.y) << "feature " << i;
        EXPECT_EQ(static_cast<int>(ls.getCoordinates()[2].x), v3.x) << "feature " << i;
        EXPECT_EQ(static_cast<int>(ls.getCoordinates()[2].y), v3.y) << "feature " << i;
    }
}

TEST(Encode, FeatureSortingPoints) {
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "sorted_points";
    layer.extent = 4096;

    std::vector<Encoder::Vertex> positions = {
        {3000, 3000},
        {100, 100},
        {2000, 500},
        {500, 3500},
        {1500, 1500},
        {3500, 100},
        {200, 2000},
        {2500, 2500},
        {800, 800},
        {3200, 1800},
    };

    for (int i = 0; i < static_cast<int>(positions.size()); ++i) {
        Encoder::Feature f;
        f.id = i + 1;
        f.geometry.type = metadata::tileset::GeometryType::POINT;
        f.geometry.coordinates = {positions[i]};
        f.properties["name"] = std::string("P" + std::to_string(i));
        layer.features.push_back(std::move(f));
    }

    auto tileData = encoder.encode({layer});
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("sorted_points");
    ASSERT_TRUE(decoded);
    ASSERT_EQ(decoded->getFeatures().size(), positions.size());

    std::map<std::uint64_t, const mlt::Feature*> byId;
    for (const auto& f : decoded->getFeatures()) {
        byId[f.getID()] = &f;
    }
    for (int i = 0; i < static_cast<int>(positions.size()); ++i) {
        auto it = byId.find(i + 1);
        ASSERT_TRUE(it != byId.end()) << "missing id " << (i + 1);
        const auto& pt = dynamic_cast<const geometry::Point&>(it->second->getGeometry());
        EXPECT_EQ(static_cast<int>(pt.getCoordinate().x), positions[i].x);
        EXPECT_EQ(static_cast<int>(pt.getCoordinate().y), positions[i].y);
    }

    {
        int32_t minV = INT32_MAX, maxV = INT32_MIN;
        for (const auto& p : positions) {
            minV = std::min({minV, p.x, p.y});
            maxV = std::max({maxV, p.x, p.y});
        }
        mlt::util::HilbertCurve curve(minV, maxV);
        std::uint32_t prevHilbert = 0;
        for (const auto& f : decoded->getFeatures()) {
            const auto& pt = dynamic_cast<const geometry::Point&>(f.getGeometry());
            auto h = curve.encode({pt.getCoordinate().x, pt.getCoordinate().y});
            EXPECT_GE(h, prevHilbert) << "features not in Hilbert order at id=" << f.getID();
            prevHilbert = h;
        }
    }
}

TEST(Encode, FeatureSortingLineStrings) {
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "sorted_lines";
    layer.extent = 4096;

    std::vector<std::pair<Encoder::Vertex, Encoder::Vertex>> segments = {
        {{3000, 3000}, {3100, 3100}},
        {{100, 100}, {200, 200}},
        {{2000, 500}, {2100, 600}},
        {{500, 3500}, {600, 3600}},
        {{1500, 1500}, {1600, 1600}},
    };

    for (int i = 0; i < static_cast<int>(segments.size()); ++i) {
        Encoder::Feature f;
        f.id = i + 1;
        f.geometry.type = metadata::tileset::GeometryType::LINESTRING;
        f.geometry.coordinates = {segments[i].first, segments[i].second};
        layer.features.push_back(std::move(f));
    }

    auto tileData = encoder.encode({layer});
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("sorted_lines");
    ASSERT_TRUE(decoded);
    ASSERT_EQ(decoded->getFeatures().size(), segments.size());

    std::map<std::uint64_t, const mlt::Feature*> byId;
    for (const auto& f : decoded->getFeatures()) {
        byId[f.getID()] = &f;
    }
    for (int i = 0; i < static_cast<int>(segments.size()); ++i) {
        auto it = byId.find(i + 1);
        ASSERT_TRUE(it != byId.end());
        const auto& ls = dynamic_cast<const geometry::LineString&>(it->second->getGeometry());
        ASSERT_EQ(ls.getCoordinates().size(), 2u);
        EXPECT_EQ(static_cast<int>(ls.getCoordinates()[0].x), segments[i].first.x);
        EXPECT_EQ(static_cast<int>(ls.getCoordinates()[0].y), segments[i].first.y);
        EXPECT_EQ(static_cast<int>(ls.getCoordinates()[1].x), segments[i].second.x);
        EXPECT_EQ(static_cast<int>(ls.getCoordinates()[1].y), segments[i].second.y);
    }

    {
        int32_t minV = INT32_MAX, maxV = INT32_MIN;
        for (const auto& [a, b] : segments) {
            minV = std::min({minV, a.x, a.y, b.x, b.y});
            maxV = std::max({maxV, a.x, a.y, b.x, b.y});
        }
        mlt::util::HilbertCurve curve(minV, maxV);
        std::uint32_t prevHilbert = 0;
        for (const auto& f : decoded->getFeatures()) {
            const auto& ls = dynamic_cast<const geometry::LineString&>(f.getGeometry());
            auto h = curve.encode(ls.getCoordinates()[0]);
            EXPECT_GE(h, prevHilbert) << "lines not in Hilbert order at id=" << f.getID();
            prevHilbert = h;
        }
    }
}

TEST(Encode, NoSortingForMixedTypes) {
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "mixed";
    layer.extent = 4096;

    Encoder::Feature f1;
    f1.id = 1;
    f1.geometry.type = metadata::tileset::GeometryType::POINT;
    f1.geometry.coordinates = {{3000, 3000}};
    layer.features.push_back(std::move(f1));

    Encoder::Feature f2;
    f2.id = 2;
    f2.geometry.type = metadata::tileset::GeometryType::LINESTRING;
    f2.geometry.coordinates = {{100, 100}, {200, 200}};
    layer.features.push_back(std::move(f2));

    auto tileData = encoder.encode({layer});
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("mixed");
    ASSERT_TRUE(decoded);
    ASSERT_EQ(decoded->getFeatures().size(), 2u);
    EXPECT_EQ(decoded->getFeatures()[0].getID(), 1u);
    EXPECT_EQ(decoded->getFeatures()[1].getID(), 2u);
}

TEST(Encode, StructColumnRoundtrip) {
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "roads";
    layer.extent = 4096;

    for (int i = 0; i < 50; ++i) {
        Encoder::Feature f;
        f.id = i;
        f.geometry.type = metadata::tileset::GeometryType::LINESTRING;
        f.geometry.coordinates = {{i * 10, i * 20}, {i * 10 + 5, i * 20 + 5}};

        Encoder::StructValue names;
        names["default"] = "Road " + std::to_string(i);
        if (i % 3 == 0) {
            names["en"] = "Road " + std::to_string(i);
        }
        if (i % 5 == 0) {
            names["de"] = "Strasse " + std::to_string(i);
        }
        f.properties["name"] = std::move(names);
        f.properties["class"] = std::string(i % 2 == 0 ? "primary" : "secondary");

        layer.features.push_back(std::move(f));
    }

    auto tileData = encoder.encode({layer});
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("roads");
    ASSERT_TRUE(decoded);
    ASSERT_EQ(decoded->getFeatures().size(), 50u);

    const auto& props = decoded->getProperties();
    EXPECT_TRUE(props.contains("class"));
    EXPECT_TRUE(props.contains("namedefault"));
    EXPECT_TRUE(props.contains("nameen"));
    EXPECT_TRUE(props.contains("namede"));

    std::map<std::uint64_t, std::size_t> idToIdx;
    for (std::size_t i = 0; i < decoded->getFeatures().size(); ++i) {
        idToIdx[decoded->getFeatures()[i].getID()] = i;
    }

    for (int i = 0; i < 50; ++i) {
        auto idx = idToIdx.at(i);

        auto defaultName = props.at("namedefault").getProperty(static_cast<std::uint32_t>(idx));
        ASSERT_TRUE(defaultName.has_value()) << "feature " << i;
        EXPECT_EQ(std::get<std::string_view>(*defaultName), "Road " + std::to_string(i));

        auto enName = props.at("nameen").getProperty(static_cast<std::uint32_t>(idx));
        if (i % 3 == 0) {
            ASSERT_TRUE(enName.has_value()) << "feature " << i;
            EXPECT_EQ(std::get<std::string_view>(*enName), "Road " + std::to_string(i));
        } else {
            EXPECT_FALSE(enName.has_value()) << "feature " << i;
        }

        auto deName = props.at("namede").getProperty(static_cast<std::uint32_t>(idx));
        if (i % 5 == 0) {
            ASSERT_TRUE(deName.has_value()) << "feature " << i;
            EXPECT_EQ(std::get<std::string_view>(*deName), "Strasse " + std::to_string(i));
        } else {
            EXPECT_FALSE(deName.has_value()) << "feature " << i;
        }
    }
}

TEST(CrossValidate, StructColumnOMTRoundtrip) {
    auto fixture = loadFixture("omt/2_2_2.mlt");
    if (fixture.empty()) GTEST_SKIP() << "Fixture not found";

    auto javaTile = Decoder().decode({fixture.data(), fixture.size()});
    const auto* javaLayer = javaTile.getLayer("water_name");
    if (!javaLayer) GTEST_SKIP() << "water_name layer not found";
    ASSERT_GT(javaLayer->getFeatures().size(), 0u);

    const auto& javaProps = javaLayer->getProperties();
    ASSERT_TRUE(javaProps.contains("class"));

    std::set<std::string> structChildNames;
    for (const auto& [name, _] : javaProps) {
        if (name.starts_with("name")) {
            structChildNames.insert(name.substr(4));
        }
    }
    ASSERT_GT(structChildNames.size(), 5u);

    Encoder::Layer encLayer;
    encLayer.name = javaLayer->getName();
    encLayer.extent = javaLayer->getExtent();

    for (std::size_t fi = 0; fi < javaLayer->getFeatures().size(); ++fi) {
        const auto& feat = javaLayer->getFeatures()[fi];
        Encoder::Feature ef;
        ef.id = feat.getID();

        const auto& geom = feat.getGeometry();
        ef.geometry.type = geom.type;
        switch (geom.type) {
            case metadata::tileset::GeometryType::POINT: {
                const auto& pt = dynamic_cast<const geometry::Point&>(geom);
                ef.geometry.coordinates = {toEncVertex(pt.getCoordinate())};
                break;
            }
            case metadata::tileset::GeometryType::LINESTRING: {
                const auto& ls = dynamic_cast<const geometry::LineString&>(geom);
                for (const auto& c : ls.getCoordinates()) ef.geometry.coordinates.push_back(toEncVertex(c));
                break;
            }
            default:
                break;
        }

        Encoder::StructValue nameStruct;
        for (const auto& childName : structChildNames) {
            const auto propName = "name" + childName;
            auto it = javaProps.find(propName);
            if (it == javaProps.end()) continue;
            auto val = it->second.getProperty(static_cast<std::uint32_t>(fi));
            if (!val.has_value()) continue;
            if (auto* sv = std::get_if<std::string_view>(&*val)) {
                nameStruct[childName] = std::string(*sv);
            }
        }
        if (!nameStruct.empty()) {
            ef.properties["name"] = std::move(nameStruct);
        }

        if (javaProps.contains("class")) {
            auto val = javaProps.at("class").getProperty(static_cast<std::uint32_t>(fi));
            if (val.has_value()) {
                if (auto* sv = std::get_if<std::string_view>(&*val)) {
                    ef.properties["class"] = std::string(*sv);
                }
            }
        }
        if (javaProps.contains("intermittent")) {
            auto val = javaProps.at("intermittent").getProperty(static_cast<std::uint32_t>(fi));
            if (val.has_value()) {
                if (auto* bv = std::get_if<bool>(&*val)) {
                    ef.properties["intermittent"] = *bv;
                }
            }
        }

        encLayer.features.push_back(std::move(ef));
    }

    Encoder encoder;
    EncoderConfig config;
    config.sortFeatures = false;
    auto reencoded = encoder.encode({encLayer}, config);
    ASSERT_FALSE(reencoded.empty());

    auto cppTile = Decoder().decode({reinterpret_cast<const char*>(reencoded.data()), reencoded.size()});
    const auto* cppLayer = cppTile.getLayer("water_name");
    ASSERT_TRUE(cppLayer);
    ASSERT_EQ(cppLayer->getFeatures().size(), javaLayer->getFeatures().size());

    const auto& cppProps = cppLayer->getProperties();

    for (const auto& childName : structChildNames) {
        const auto propName = "name" + childName;
        ASSERT_TRUE(cppProps.contains(propName)) << "missing struct child property: " << propName;

        const auto& javaPP = javaProps.at(propName);
        const auto& cppPP = cppProps.at(propName);

        for (std::size_t fi = 0; fi < javaLayer->getFeatures().size(); ++fi) {
            auto javaId = javaLayer->getFeatures()[fi].getID();
            std::size_t cppIdx = 0;
            for (std::size_t ci = 0; ci < cppLayer->getFeatures().size(); ++ci) {
                if (cppLayer->getFeatures()[ci].getID() == javaId) {
                    cppIdx = ci;
                    break;
                }
            }

            auto javaVal = javaPP.getProperty(static_cast<std::uint32_t>(fi));
            auto cppVal = cppPP.getProperty(static_cast<std::uint32_t>(cppIdx));
            EXPECT_EQ(javaVal.has_value(), cppVal.has_value()) << propName << " id=" << javaId;
            if (javaVal.has_value() && cppVal.has_value()) {
                auto jSV = std::get_if<std::string_view>(&*javaVal);
                auto cSV = std::get_if<std::string_view>(&*cppVal);
                ASSERT_TRUE(jSV && cSV) << propName << " id=" << javaId;
                EXPECT_EQ(*jSV, *cSV) << propName << " id=" << javaId;
            }
        }
    }
}

TEST(Encode, PretessellatedPolygonRoundtrip) {
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "buildings";
    layer.extent = 4096;

    {
        Encoder::Feature f;
        f.id = 1;
        f.geometry.type = metadata::tileset::GeometryType::POLYGON;
        f.geometry.coordinates = {{100, 100}, {200, 100}, {200, 200}, {100, 200}};
        f.geometry.ringSizes = {4};
        f.properties["height"] = std::int32_t{10};
        layer.features.push_back(std::move(f));
    }

    {
        Encoder::Feature f;
        f.id = 2;
        f.geometry.type = metadata::tileset::GeometryType::POLYGON;
        f.geometry.coordinates = {
            {0, 0}, {400, 0}, {400, 400}, {0, 400}, {100, 100}, {300, 100}, {300, 300}, {100, 300}};
        f.geometry.ringSizes = {4, 4};
        f.properties["height"] = std::int32_t{20};
        layer.features.push_back(std::move(f));
    }

    EncoderConfig config;
    config.preTessellate = true;
    config.sortFeatures = false;
    auto tileData = encoder.encode({layer}, config);
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("buildings");
    ASSERT_TRUE(decoded);
    ASSERT_EQ(decoded->getFeatures().size(), 2u);

    const auto& features = decoded->getFeatures();

    // Simple quad → 2 triangles = 6 indices
    const auto& simplePoly = features[0].getGeometry();
    EXPECT_EQ(simplePoly.getTriangles().size(), 6u);
    for (auto idx : simplePoly.getTriangles()) {
        EXPECT_LT(idx, 4u);
    }

    // Quad with hole → 8 triangles = 24 indices
    const auto& holedPoly = features[1].getGeometry();
    EXPECT_EQ(holedPoly.getTriangles().size(), 24u);
    for (auto idx : holedPoly.getTriangles()) {
        EXPECT_LT(idx, 8u);
    }
}

TEST(Encode, PretessellatedMultiPolygonCrossValidation) {
    // Cross-validates against Java TessellationUtilsTest.tessellateMultiPolygon_PolygonsWithoutHoles
    // Expected: 4 triangles total, indices [3,0,1, 1,2,3, 7,4,5, 5,6,7]
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "landuse";
    layer.extent = 4096;

    Encoder::Feature f;
    f.id = 1;
    f.geometry.type = metadata::tileset::GeometryType::MULTIPOLYGON;
    f.geometry.parts = {
        {{0, 0}, {10, 0}, {10, 10}, {0, 10}},
        {{20, 20}, {40, 20}, {40, 40}, {20, 40}},
    };
    f.geometry.partRingSizes = {{4}, {4}};
    layer.features.push_back(std::move(f));

    EncoderConfig config;
    config.preTessellate = true;
    config.sortFeatures = false;
    auto tileData = encoder.encode({layer}, config);
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("landuse");
    ASSERT_TRUE(decoded);
    ASSERT_EQ(decoded->getFeatures().size(), 1u);

    const auto& features = decoded->getFeatures();
    ASSERT_EQ(features.size(), 1u);

    auto triangles = features[0].getGeometry().getTriangles();

    // 4 triangles = 12 indices (Java expected: [3,0,1, 1,2,3, 7,4,5, 5,6,7])
    EXPECT_EQ(triangles.size(), 12u);

    for (auto idx : triangles) {
        EXPECT_LT(idx, 8u);
    }
}

TEST(Encode, PretessellatedMultiPolygonWithHoles) {
    // Cross-validates against Java TessellationUtilsTest.tessellateMultiPolygon_PolygonsWithHoles
    // Expected: 10 triangles total
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "landuse";
    layer.extent = 4096;

    Encoder::Feature f;
    f.id = 1;
    f.geometry.type = metadata::tileset::GeometryType::MULTIPOLYGON;
    f.geometry.parts = {
        {{0, 0}, {10, 0}, {10, 10}, {0, 10}, {5, 5}, {5, 7}, {7, 7}, {7, 5}},
        {{20, 20}, {40, 20}, {40, 40}, {20, 40}},
    };
    f.geometry.partRingSizes = {{4, 4}, {4}};
    layer.features.push_back(std::move(f));

    EncoderConfig config;
    config.preTessellate = true;
    config.sortFeatures = false;
    auto tileData = encoder.encode({layer}, config);
    ASSERT_FALSE(tileData.empty());

    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("landuse");
    ASSERT_TRUE(decoded);

    const auto& features = decoded->getFeatures();
    ASSERT_EQ(features.size(), 1u);

    auto triangles = features[0].getGeometry().getTriangles();

    // 10 triangles = 30 indices (Java TessellationUtilsTest expected: 10)
    EXPECT_EQ(triangles.size() / 3, 10u);
}

TEST(Encode, PretessellatedSkippedForMixedGeometry) {
    // Pre-tessellation should NOT activate when layer has mixed geometry types
    Encoder encoder;
    Encoder::Layer layer;
    layer.name = "mixed";
    layer.extent = 4096;

    {
        Encoder::Feature f;
        f.id = 1;
        f.geometry.type = metadata::tileset::GeometryType::POLYGON;
        f.geometry.coordinates = {{0, 0}, {10, 0}, {10, 10}, {0, 10}};
        f.geometry.ringSizes = {4};
        layer.features.push_back(std::move(f));
    }
    {
        Encoder::Feature f;
        f.id = 2;
        f.geometry.type = metadata::tileset::GeometryType::LINESTRING;
        f.geometry.coordinates = {{0, 0}, {10, 10}};
        layer.features.push_back(std::move(f));
    }

    EncoderConfig config;
    config.preTessellate = true;
    config.sortFeatures = false;
    auto tileData = encoder.encode({layer}, config);
    ASSERT_FALSE(tileData.empty());

    // Should still decode fine — just no tessellation data
    auto tile = Decoder().decode({reinterpret_cast<const char*>(tileData.data()), tileData.size()});
    const auto* decoded = tile.getLayer("mixed");
    ASSERT_TRUE(decoded);
    ASSERT_EQ(decoded->getFeatures().size(), 2u);
}

std::vector<std::string> discoverFixtures(const std::string& subdir) {
    std::vector<std::string> result;
    for (const auto& prefix : {"../test/expected/tag0x01/",
                               "../../test/expected/tag0x01/",
                               "../../../test/expected/tag0x01/",
                               "test/expected/tag0x01/"}) {
        std::error_code ec;
        for (const auto& entry : std::filesystem::directory_iterator(prefix + subdir, ec)) {
            if (!ec && entry.path().extension() == ".mlt") {
                result.push_back(entry.path().filename().string());
            }
        }
        if (!result.empty()) break;
    }
    std::sort(result.begin(), result.end());
    return result;
}

void reencodeRoundtrip(const std::string& subdir, const std::string& filename) {
    auto fixture = loadFixture(subdir + "/" + filename);
    if (fixture.empty()) GTEST_SKIP() << "Fixture not found: " << filename;

    auto javaTile = Decoder().decode({fixture.data(), fixture.size()});

    Encoder encoder;
    std::vector<Encoder::Layer> encoderLayers;
    for (const auto& layer : javaTile.getLayers()) {
        encoderLayers.push_back(decodedToEncoderLayer(layer));
    }
    ASSERT_FALSE(encoderLayers.empty()) << "No layers in " << filename;

    EncoderConfig config;
    config.sortFeatures = false;
    auto encoded = encoder.encode(encoderLayers, config);
    ASSERT_FALSE(encoded.empty()) << "Encode produced empty output for " << filename;

    auto redecodedTile = Decoder().decode({reinterpret_cast<const char*>(encoded.data()), encoded.size()});

    for (const auto& javaLayer : javaTile.getLayers()) {
        const auto* reLayer = redecodedTile.getLayer(javaLayer.getName());
        ASSERT_TRUE(reLayer) << "Missing layer " << javaLayer.getName() << " in re-encoded " << filename;
        ASSERT_EQ(javaLayer.getFeatures().size(), reLayer->getFeatures().size())
            << "Feature count mismatch in layer " << javaLayer.getName() << " of " << filename;
        compareDecodedTiles(javaLayer, *reLayer, false);
    }
}

class ReencodeOMT : public ::testing::TestWithParam<std::string> {};
TEST_P(ReencodeOMT, Roundtrip) {
    reencodeRoundtrip("omt", GetParam());
}
INSTANTIATE_TEST_SUITE_P(AllOMT, ReencodeOMT, ::testing::ValuesIn(discoverFixtures("omt")), [](const auto& info) {
    auto name = info.param;
    std::replace(name.begin(), name.end(), '.', '_');
    std::replace(name.begin(), name.end(), '-', '_');
    return name;
});

class ReencodeBing : public ::testing::TestWithParam<std::string> {};
TEST_P(ReencodeBing, Roundtrip) {
    reencodeRoundtrip("bing", GetParam());
}
INSTANTIATE_TEST_SUITE_P(AllBing, ReencodeBing, ::testing::ValuesIn(discoverFixtures("bing")), [](const auto& info) {
    auto name = info.param;
    std::replace(name.begin(), name.end(), '.', '_');
    std::replace(name.begin(), name.end(), '-', '_');
    return name;
});

class ReencodeAmazon : public ::testing::TestWithParam<std::string> {};
TEST_P(ReencodeAmazon, Roundtrip) {
    reencodeRoundtrip("amazon", GetParam());
}
INSTANTIATE_TEST_SUITE_P(AllAmazon,
                         ReencodeAmazon,
                         ::testing::ValuesIn(discoverFixtures("amazon")),
                         [](const auto& info) {
                             auto name = info.param;
                             std::replace(name.begin(), name.end(), '.', '_');
                             std::replace(name.begin(), name.end(), '-', '_');
                             return name;
                         });

class ReencodeAmazonHere : public ::testing::TestWithParam<std::string> {};
TEST_P(ReencodeAmazonHere, Roundtrip) {
    reencodeRoundtrip("amazon_here", GetParam());
}
INSTANTIATE_TEST_SUITE_P(AllAmazonHere,
                         ReencodeAmazonHere,
                         ::testing::ValuesIn(discoverFixtures("amazon_here")),
                         [](const auto& info) {
                             auto name = info.param;
                             std::replace(name.begin(), name.end(), '.', '_');
                             std::replace(name.begin(), name.end(), '-', '_');
                             return name;
                         });

std::string featureFingerprint(const Encoder::Feature& f) {
    std::string fp = std::to_string(f.id) + "|" + std::to_string(static_cast<int>(f.geometry.type));
    for (const auto& v : f.geometry.coordinates) {
        fp += "|" + std::to_string(v.x) + "," + std::to_string(v.y);
    }
    return fp;
}

void compareEncoderLayersSorted(const Encoder::Layer& a, const Encoder::Layer& b) {
    ASSERT_EQ(a.name, b.name);
    ASSERT_EQ(a.extent, b.extent);
    ASSERT_EQ(a.features.size(), b.features.size());

    std::multimap<std::string, const Encoder::Feature*> aByFp, bByFp;
    for (const auto& f : a.features) aByFp.emplace(featureFingerprint(f), &f);
    for (const auto& f : b.features) bByFp.emplace(featureFingerprint(f), &f);

    ASSERT_EQ(aByFp.size(), bByFp.size());

    for (auto itA = aByFp.begin(), itB = bByFp.begin(); itA != aByFp.end(); ++itA, ++itB) {
        ASSERT_EQ(itA->first, itB->first) << "Feature fingerprint mismatch in " << a.name;
        ASSERT_EQ(itA->second->properties.size(), itB->second->properties.size())
            << "Property count mismatch for " << itA->first;
    }
}

void reencodeRoundtripSorted(const std::string& subdir, const std::string& filename) {
    auto fixture = loadFixture(subdir + "/" + filename);
    if (fixture.empty()) GTEST_SKIP() << "Fixture not found: " << filename;

    auto javaTile = Decoder().decode({fixture.data(), fixture.size()});

    Encoder encoder;
    std::vector<Encoder::Layer> originalLayers;
    for (const auto& layer : javaTile.getLayers()) {
        originalLayers.push_back(decodedToEncoderLayer(layer));
    }
    ASSERT_FALSE(originalLayers.empty()) << "No layers in " << filename;

    EncoderConfig config;
    config.sortFeatures = true;
    auto encoded = encoder.encode(originalLayers, config);
    ASSERT_FALSE(encoded.empty()) << "Encode produced empty output for " << filename;

    auto redecodedTile = Decoder().decode({reinterpret_cast<const char*>(encoded.data()), encoded.size()});

    for (const auto& origLayer : originalLayers) {
        const auto* reDecodedLayer = redecodedTile.getLayer(origLayer.name);
        ASSERT_TRUE(reDecodedLayer) << "Missing layer " << origLayer.name << " in re-encoded " << filename;
        auto reEncoderLayer = decodedToEncoderLayer(*reDecodedLayer);
        compareEncoderLayersSorted(origLayer, reEncoderLayer);
    }
}

class ReencodeOMTSorted : public ::testing::TestWithParam<std::string> {};
TEST_P(ReencodeOMTSorted, Roundtrip) {
    reencodeRoundtripSorted("omt", GetParam());
}
INSTANTIATE_TEST_SUITE_P(AllOMTSorted,
                         ReencodeOMTSorted,
                         ::testing::ValuesIn(discoverFixtures("omt")),
                         [](const auto& info) {
                             auto name = info.param;
                             std::replace(name.begin(), name.end(), '.', '_');
                             std::replace(name.begin(), name.end(), '-', '_');
                             return name;
                         });

void reencodeTessellated(const std::string& subdir, const std::string& filename) {
    auto fixture = loadFixture(subdir + "/" + filename);
    if (fixture.empty()) GTEST_SKIP() << "Fixture not found: " << filename;

    auto javaTile = Decoder().decode({fixture.data(), fixture.size()});

    Encoder encoder;
    std::vector<Encoder::Layer> encoderLayers;
    for (const auto& layer : javaTile.getLayers()) {
        encoderLayers.push_back(decodedToEncoderLayer(layer));
    }
    ASSERT_FALSE(encoderLayers.empty()) << "No layers in " << filename;

    EncoderConfig config;
    config.sortFeatures = false;
    config.preTessellate = true;
    auto encoded = encoder.encode(encoderLayers, config);
    ASSERT_FALSE(encoded.empty()) << "Encode produced empty output for " << filename;

    auto redecodedTile = Decoder().decode({reinterpret_cast<const char*>(encoded.data()), encoded.size()});

    using GT = metadata::tileset::GeometryType;
    for (std::size_t li = 0; li < encoderLayers.size(); ++li) {
        const auto& origLayer = encoderLayers[li];
        const auto* reLayer = redecodedTile.getLayer(origLayer.name);
        ASSERT_TRUE(reLayer) << "Missing layer " << origLayer.name;
        ASSERT_EQ(origLayer.features.size(), reLayer->getFeatures().size());

        bool allPoly = std::ranges::all_of(origLayer.features, [](const auto& f) {
            return f.geometry.type == GT::POLYGON || f.geometry.type == GT::MULTIPOLYGON;
        });

        if (allPoly && !origLayer.features.empty()) {
            for (const auto& feat : reLayer->getFeatures()) {
                const auto& geom = feat.getGeometry();
                EXPECT_FALSE(geom.getTriangles().empty())
                    << "Expected triangles for polygon feature in layer " << origLayer.name << " of " << filename;
            }
        }

        compareDecodedTiles(*javaTile.getLayer(origLayer.name), *reLayer, false);
    }
}

class ReencodeOMTTessellated : public ::testing::TestWithParam<std::string> {};
TEST_P(ReencodeOMTTessellated, Roundtrip) {
    reencodeTessellated("omt", GetParam());
}
INSTANTIATE_TEST_SUITE_P(AllOMTTessellated,
                         ReencodeOMTTessellated,
                         ::testing::ValuesIn(discoverFixtures("omt")),
                         [](const auto& info) {
                             auto name = info.param;
                             std::replace(name.begin(), name.end(), '.', '_');
                             std::replace(name.begin(), name.end(), '-', '_');
                             return name;
                         });
