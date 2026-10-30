// Tests for decoding invalid tiles.
//
// Tiles come from arbitrary sources, so every length, count, offset and index in them is untrusted. Decoding an
// invalid tile has to fail with a `std::runtime_error`, and not crash, read or write out of bounds, or use enormous
// amounts of memory. Run these under ASan/UBSan to catch accesses that are out of bounds but don't crash.
//
// Most of the cases here were found by fuzzing, see `cpp/fuzz`.

#include <gtest/gtest.h>

#include <mlt/decode/geometry.hpp>
#include <mlt/decode/int.hpp>
#include <mlt/decode/int_template.hpp> // NOLINT(misc-include-cleaner)
#include <mlt/decode/string.hpp>
#include <mlt/decoder.hpp>
#include <mlt/geometry.hpp>
#include <mlt/geometry_vector.hpp>
#include <mlt/metadata/stream.hpp>
#include <mlt/metadata/tileset.hpp>
#include <mlt/util/buffer_stream.hpp>
#include <mlt/util/memory_budget.hpp>
#include <mlt/util/rle.hpp>
#include <mlt/util/varint.hpp>
#include <mlt/util/vectorized.hpp>

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <iterator>
#include <limits>
#include <optional>
#include <stdexcept>
#include <string>
#include <string_view>
#include <vector>

namespace {

using namespace mlt;
using namespace mlt::metadata::stream;
using metadata::tileset::GeometryType;

using Bytes = std::vector<std::uint8_t>;

void putVarint(Bytes& out, std::uint32_t value) {
    while (value >= 0x80) {
        out.push_back(static_cast<std::uint8_t>(value | 0x80));
        value >>= 7;
    }
    out.push_back(static_cast<std::uint8_t>(value));
}

void putString(Bytes& out, const std::string_view string) {
    putVarint(out, static_cast<std::uint32_t>(string.size()));
    out.insert(out.end(), string.begin(), string.end());
}

// A tile with one layer, which has the given contents after the layer tag
Bytes tileWithLayer(const Bytes& contents) {
    Bytes layer;
    putVarint(layer, 1); // the tag of a layer
    layer.insert(layer.end(), contents.begin(), contents.end());

    Bytes tile;
    putVarint(tile, static_cast<std::uint32_t>(layer.size()));
    tile.insert(tile.end(), layer.begin(), layer.end());
    return tile;
}

MapLibreTile decodeTile(const Bytes& tile) {
    return Decoder().decode({reinterpret_cast<const char*>(tile.data()), tile.size()});
}

DataView view(const Bytes& bytes) {
    return {reinterpret_cast<const char*>(bytes.data()), bytes.size()};
}

StreamMetadata makeMetadata(PhysicalLevelTechnique physical, std::uint32_t numValues, std::uint32_t byteLength) {
    return {PhysicalStreamType::DATA,
            std::nullopt,
            LogicalLevelTechnique::NONE,
            LogicalLevelTechnique::NONE,
            physical,
            numValues,
            byteLength};
}

} // namespace

#pragma region Tile structure

TEST(Robustness, LayerWithoutGeometryColumn) {
    Bytes contents;
    putString(contents, "layer");
    putVarint(contents, 4096); // extent
    putVarint(contents, 0);    // columns
    EXPECT_THROW(decodeTile(tileWithLayer(contents)), std::runtime_error);
}

TEST(Robustness, ColumnCountExceedsData) {
    Bytes contents;
    putString(contents, "layer");
    putVarint(contents, 4096);
    putVarint(contents, 0xffffffffu);
    EXPECT_THROW(decodeTile(tileWithLayer(contents)), std::runtime_error);
}

TEST(Robustness, StringLengthExceedsData) {
    Bytes contents;
    putVarint(contents, 0xffffffffu); // the length of the layer name
    contents.push_back('a');
    EXPECT_THROW(decodeTile(tileWithLayer(contents)), std::runtime_error);
}

// Each level of nesting is a recursive call
TEST(Robustness, DeeplyNestedColumns) {
    Bytes contents;
    putString(contents, "layer");
    putVarint(contents, 4096);
    putVarint(contents, 1);
    for (int i = 0; i < 500000; ++i) {
        putVarint(contents, 30); // a struct, with a name and children
        putVarint(contents, 0);  // empty name
        putVarint(contents, 1);  // one child
    }
    EXPECT_THROW(decodeTile(tileWithLayer(contents)), std::runtime_error);
}

TEST(Robustness, LayerLengthExceedsData) {
    Bytes tile;
    putVarint(tile, 0xffffffffu);
    tile.push_back(1);
    EXPECT_THROW(decodeTile(tile), std::runtime_error);
}

// Found by fuzzing: the values of an RLE stream were read beyond the end of the stream
TEST(Robustness, FuzzerRleOutOfBounds) {
    const Bytes tile = {0x2e, 0x01, 0x06, 0x6c, 0x61, 0x79, 0x65, 0x72, 0x31, 0x40, 0x02, 0x04, 0x16, 0x03, 0x76, 0x61,
                        0x6c, 0x02, 0x30, 0x62, 0x02, 0x02, 0x01, 0x04, 0x04, 0x00, 0x13, 0x42, 0x08, 0x08, 0x1a, 0x54,
                        0x03, 0x14, 0x78, 0x28, 0x13, 0x63, 0x10, 0x62, 0x02, 0x03, 0x10, 0x01, 0x04, 0x04, 0xa8, 0x46};
    EXPECT_THROW(decodeTile(tile), std::runtime_error);
}

// Found by fuzzing: a column inside a struct column was assumed to have a physical type, but it can be a logical type.
// The failure was an exception of another type than `std::runtime_error`.
TEST(Robustness, FuzzerStructChildOfLogicalType) {
    const Bytes tile = {
        0xcf, 0x01, 0x01, 0x06, 0x6c, 0x61, 0x79, 0x65, 0x72, 0x31, 0x40, 0x04, 0x04, 0x13, 0x06, 0x62, 0x65, 0x66,
        0x6f, 0x72, 0x65, 0x1e, 0x05, 0x6e, 0x61, 0x6d, 0x65, 0x3a, 0x03, 0x1d, 0x22, 0xe4, 0x65, 0x1d, 0x02, 0x65,
        0x6e, 0x1c, 0x02, 0x66, 0x72, 0x13, 0x05, 0x61, 0x66, 0x74, 0x65, 0x72, 0x02, 0x30, 0x62, 0x02, 0x02, 0x01,
        0x06, 0x06, 0x00, 0x13, 0x42, 0x0c, 0x0c, 0x1a, 0x54, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x60, 0x06, 0x02, 0xff, 0x29, 0x10, 0x02, 0x03, 0x03, 0x00, 0x03, 0x05, 0x07, 0x36, 0x02, 0x02,
        0x02, 0x1e, 0x1e, 0x12, 0x00, 0x02, 0x3c, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41, 0x41,
        0x06, 0x02, 0xff, 0x29, 0x22, 0x02, 0x03, 0x03, 0x00, 0x23, 0x10, 0x02, 0x03, 0x03, 0x00, 0x01, 0x05, 0x01,
        0x00, 0x01, 0x00, 0x01, 0x02, 0xff, 0x23, 0x10, 0x02, 0x03, 0x03, 0x00, 0x01, 0x05, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x72, 0x73, 0x74, 0x75,
        0x36, 0x02, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0xff, 0x29, 0x22, 0x02,
        0x03, 0x03, 0x00, 0x0e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x60,
        0x06, 0x02, 0xff, 0x23, 0x10, 0x02, 0x03, 0x03, 0x00, 0x01, 0x05,
    };
    EXPECT_THROW(decodeTile(tile), std::runtime_error);
}

TEST(Robustness, MemoryLimit) {
    // A tile of any size which needs memory for its geometry
    const std::filesystem::path path = "../test/expected/0x01/simple/line-boolean.mlt";
    std::ifstream file(path, std::ios::binary);
    ASSERT_TRUE(file.is_open());
    const std::string data{std::istreambuf_iterator<char>(file), std::istreambuf_iterator<char>()};

    Decoder decoder;
    EXPECT_NO_THROW(decoder.decode({data.data(), data.size()}));
    decoder.setMemoryLimit(1);
    EXPECT_THROW(decoder.decode({data.data(), data.size()}), std::runtime_error);
    // The limit applies to each tile
    decoder.setMemoryLimit(util::MemoryBudget::defaultLimit);
    EXPECT_NO_THROW(decoder.decode({data.data(), data.size()}));
    EXPECT_NO_THROW(decoder.decode({data.data(), data.size()}));
}

#pragma endregion

#pragma region Streams

TEST(Robustness, BufferStream) {
    const Bytes data = {1, 2, 3};
    BufferStream stream(view(data));
    EXPECT_THROW(stream.getSubStream(std::numeric_limits<std::size_t>::max(), 2), std::runtime_error);
    EXPECT_THROW(stream.getSubStream(2, std::numeric_limits<std::size_t>::max()), std::runtime_error);
    EXPECT_THROW(stream.getSubStream(0, 4), std::runtime_error);
    EXPECT_NO_THROW(stream.getSubStream(1, 2));
}

TEST(Robustness, Varint64) {
    using util::decoding::decodeVarint;
    {
        const Bytes data;
        BufferStream stream(view(data));
        EXPECT_THROW(decodeVarint<std::uint64_t>(stream), std::runtime_error);
    }
    {
        const Bytes data = {0x80, 0x80};
        BufferStream stream(view(data));
        EXPECT_THROW(decodeVarint<std::uint64_t>(stream), std::runtime_error);
    }
    {
        const Bytes data = {0xac, 0x02};
        BufferStream stream(view(data));
        EXPECT_EQ(300u, decodeVarint<std::uint64_t>(stream));
    }
}

TEST(Robustness, RleInt) {
    using util::decoding::rle::decodeInt;
    // Two runs: the lengths are followed by the values
    const std::vector<std::uint32_t> runs = {2, 3, 7, 8};
    std::vector<std::uint32_t> out(5);
    decodeInt<std::uint32_t>(runs.data(), runs.size(), out.data(), out.size(), 2);
    EXPECT_EQ((std::vector<std::uint32_t>{7, 7, 8, 8, 8}), out);

    // The values of the runs are missing, or there are more runs than data
    EXPECT_THROW(decodeInt<std::uint32_t>(runs.data(), 3, out.data(), out.size(), 2), std::runtime_error);
    EXPECT_THROW(decodeInt<std::uint32_t>(runs.data(), runs.size(), out.data(), out.size(), 3), std::runtime_error);
    EXPECT_THROW(decodeInt<std::uint32_t>(runs.data(), runs.size(), out.data(), out.size(), 0x7fffffffu),
                 std::runtime_error);
    // The runs don't fit the output, or don't fill it
    EXPECT_THROW(decodeInt<std::uint32_t>(runs.data(), runs.size(), out.data(), 4, 2), std::runtime_error);
    EXPECT_THROW(decodeInt<std::uint32_t>(runs.data(), runs.size(), out.data(), 6, 2), std::runtime_error);
    const std::vector<std::uint32_t> huge = {0xffffffffu, 1};
    EXPECT_THROW(decodeInt<std::uint32_t>(huge.data(), huge.size(), out.data(), out.size(), 1), std::runtime_error);

    const std::vector<std::uint64_t> huge64 = {std::uint64_t{1} << 63, 1};
    std::vector<std::uint64_t> out64(5);
    EXPECT_THROW(decodeInt<std::uint64_t>(huge64.data(), huge64.size(), out64.data(), out64.size(), 1),
                 std::runtime_error);

    const std::vector<std::int32_t> negative = {-1, 5};
    std::vector<std::int32_t> outSigned(5);
    EXPECT_THROW(decodeInt<std::int32_t>(negative.data(), negative.size(), outSigned.data(), outSigned.size(), 1),
                 std::runtime_error);
}

TEST(Robustness, RleBoolean) {
    // Many bits from a few bytes, which would take lots of memory
    {
        const Bytes data = {0x00, 0x01};
        BufferStream stream(view(data));
        std::vector<std::uint8_t> out;
        EXPECT_THROW(util::decoding::rle::decodeBoolean(
                         stream, out, makeMetadata(PhysicalLevelTechnique::NONE, 0xffffffffu, 2), true),
                     std::runtime_error);
        EXPECT_EQ(0u, out.capacity());
    }
    // The stream is longer than the data
    {
        const Bytes data = {0x00, 0x01};
        BufferStream stream(view(data));
        std::vector<std::uint8_t> out;
        EXPECT_THROW(
            util::decoding::rle::decodeBoolean(stream, out, makeMetadata(PhysicalLevelTechnique::NONE, 8, 100), true),
            std::runtime_error);
    }
    // A run of 3 bytes of 0xa5
    {
        const Bytes data = {0x00, 0xa5};
        BufferStream stream(view(data));
        std::vector<std::uint8_t> out;
        util::decoding::rle::decodeBoolean(stream, out, makeMetadata(PhysicalLevelTechnique::NONE, 8, 2), true);
        EXPECT_EQ((std::vector<std::uint8_t>{0xa5}), out);
        EXPECT_EQ(0u, stream.getRemaining());
    }
}

TEST(Robustness, IntegerStreamLengths) {
    decoder::IntegerDecoder intDecoder;
    const Bytes data(8, 1);
    const auto decode = [&](const StreamMetadata& metadata) {
        BufferStream stream(view(data));
        std::vector<std::uint32_t> out;
        intDecoder.decodeIntStream<std::uint32_t>(stream, out, metadata);
        return out;
    };

    // Unencoded values beyond the data
    EXPECT_THROW(decode(makeMetadata(PhysicalLevelTechnique::NONE, 1000, 4000)), std::runtime_error);
    EXPECT_EQ((std::vector<std::uint32_t>{0x01010101u, 0x01010101u}),
              decode(makeMetadata(PhysicalLevelTechnique::NONE, 2, 8)));
    // More varints than there are bytes
    EXPECT_THROW(decode(makeMetadata(PhysicalLevelTechnique::VARINT, 0x7fffffffu, 8)), std::runtime_error);
    EXPECT_THROW(decode(makeMetadata(PhysicalLevelTechnique::VARINT, 9, 8)), std::runtime_error);
    // More values than blocks of 256 could describe in so few bytes
    EXPECT_THROW(decode(makeMetadata(PhysicalLevelTechnique::FAST_PFOR, 100000, 8)), std::runtime_error);
    EXPECT_THROW(decode(makeMetadata(PhysicalLevelTechnique::FAST_PFOR, 0xffffffffu, 8)), std::runtime_error);
}

// A few bytes of RLE can describe billions of values
TEST(Robustness, RleExpansion) {
    decoder::IntegerDecoder intDecoder;
    const Bytes data = {5, 7}; // a run of 5 values, which are 7
    const RleEncodedStreamMetadata metadata(PhysicalStreamType::DATA,
                                            std::nullopt,
                                            LogicalLevelTechnique::RLE,
                                            LogicalLevelTechnique::NONE,
                                            PhysicalLevelTechnique::VARINT,
                                            /*numValues=*/2,
                                            /*byteLength=*/2,
                                            /*runs=*/1,
                                            /*numRleValues=*/0x7fffffffu);
    BufferStream stream(view(data));
    std::vector<std::uint32_t> out;
    EXPECT_THROW(intDecoder.decodeIntStream<std::uint32_t>(stream, out, metadata), std::runtime_error);
    EXPECT_TRUE(out.empty());

    // The same without the expansion
    const RleEncodedStreamMetadata valid(PhysicalStreamType::DATA,
                                         std::nullopt,
                                         LogicalLevelTechnique::RLE,
                                         LogicalLevelTechnique::NONE,
                                         PhysicalLevelTechnique::VARINT,
                                         2,
                                         2,
                                         1,
                                         5);
    BufferStream validStream(view(data));
    intDecoder.decodeIntStream<std::uint32_t>(validStream, out, valid);
    EXPECT_EQ(std::vector<std::uint32_t>(5, 7), out);
}

TEST(Robustness, MortonParameters) {
    const auto roundTrip = [](unsigned numBits, unsigned coordinateShift) {
        const MortonEncodedStreamMetadata metadata(PhysicalStreamType::DATA,
                                                   LogicalStreamType(DictionaryType::MORTON),
                                                   LogicalLevelTechnique::MORTON,
                                                   LogicalLevelTechnique::NONE,
                                                   PhysicalLevelTechnique::VARINT,
                                                   1,
                                                   1,
                                                   numBits,
                                                   coordinateShift);
        const auto encoded = metadata.encode();
        BufferStream stream({reinterpret_cast<const char*>(encoded.data()), encoded.size()});
        return StreamMetadata::decode(stream);
    };
    EXPECT_NO_THROW(roundTrip(16, 0));
    EXPECT_NO_THROW(roundTrip(13, 0x7fffffffu));
    // The number of bits is used as a shift
    EXPECT_THROW(roundTrip(17, 0), std::runtime_error);
    EXPECT_THROW(roundTrip(64, 0), std::runtime_error);
    EXPECT_THROW(roundTrip(0xffffffffu, 0), std::runtime_error);
    EXPECT_THROW(roundTrip(13, 0x80000000u), std::runtime_error);
}

TEST(Robustness, ComponentwiseDeltaOverflow) {
    // The sums wrap around, which is not an error
    std::vector<std::int32_t> values = {-2, -2, -2, -2, -2, -2, 0x7ffffffe, 0x7ffffffe, 0x7ffffffe, 0x7ffffffe};
    const auto zigZag = [](std::uint32_t x) {
        return (x >> 1) ^ (0u - (x & 1u));
    };
    std::vector<std::uint32_t> expected;
    for (std::size_t i = 0; i < values.size(); ++i) {
        const auto delta = zigZag(static_cast<std::uint32_t>(values[i]));
        expected.push_back(i < 2 ? delta : delta + expected[i - 2]);
    }
    util::decoding::vectorized::decodeComponentwiseDeltaVec2(values.data(), values.size());
    for (std::size_t i = 0; i < values.size(); ++i) {
        EXPECT_EQ(expected[i], static_cast<std::uint32_t>(values[i])) << i;
    }
}

#pragma endregion

#pragma region Strings

TEST(Robustness, FsstDecode) {
    const std::vector<std::uint8_t> symbols = {'a', 'b', 'c', 'd'};
    const std::vector<std::uint32_t> lengths = {2, 2};
    const auto decode = [&](const std::vector<std::uint8_t>& compressed,
                            const std::vector<std::uint32_t>& symbolLengths = {2, 2}) {
        return decoder::StringDecoder::decodeFSST(symbols, symbolLengths, compressed, 2 * compressed.size());
    };

    EXPECT_EQ((std::vector<std::uint8_t>{'a', 'b', 'c', 'd', 'z'}), decode({0, 1, 255, 'z'}));
    EXPECT_EQ((std::vector<std::uint8_t>{'z'}), decode({255, 'z'}));
    // An escape at the end, with nothing to escape
    EXPECT_THROW(decode({0, 255}), std::runtime_error);
    EXPECT_THROW(decode({255}), std::runtime_error);
    // A symbol which doesn't exist
    EXPECT_THROW(decode({2}), std::runtime_error);
    // Symbols which are not within the table, or too long
    EXPECT_THROW(decode({1}, {4, 4}), std::runtime_error);
    EXPECT_THROW(decode({0}, {0xffffffffu, 2}), std::runtime_error);
    EXPECT_THROW(decode({1}, {0xffffffffu, 0xffffffffu}), std::runtime_error);
    EXPECT_THROW(decode({0}, {9, 2}), std::runtime_error);
    // Too many symbols
    EXPECT_THROW(decode({0}, std::vector<std::uint32_t>(256, 0)), std::runtime_error);
    // Without any compressed data, even if it's expected
    EXPECT_TRUE(decoder::StringDecoder::decodeFSST(symbols, lengths, {}, 1000).empty());
}

#pragma endregion

#pragma region Geometry

namespace {

using metadata::tileset::GeometryType;

geometry::FlatGeometryVector makeVector(std::vector<GeometryType> types,
                                        std::vector<std::uint32_t> geometryOffsets,
                                        std::vector<std::uint32_t> partOffsets,
                                        std::vector<std::uint32_t> ringOffsets,
                                        std::vector<std::uint32_t> vertexOffsets,
                                        std::vector<std::int32_t> vertices) {
    return {std::move(types),
            geometry::TopologyVector(std::move(geometryOffsets), std::move(partOffsets), std::move(ringOffsets)),
            std::move(vertexOffsets),
            std::move(vertices),
            geometry::VertexBufferType::VEC_2,
            std::nullopt};
}

auto geometries(const geometry::GeometryVector& vector) {
    return vector.getGeometries(geometry::GeometryFactory{});
}

} // namespace

TEST(Robustness, GeometryVectorValid) {
    EXPECT_EQ(1u, geometries(makeVector({GeometryType::LINESTRING}, {}, {0, 2}, {}, {}, {1, 2, 3, 4})).size());
    EXPECT_EQ(1u, geometries(makeVector({GeometryType::MULTIPOINT}, {0, 2}, {}, {}, {}, {1, 2, 3, 4})).size());
    EXPECT_EQ(1u, geometries(makeVector({GeometryType::MULTIPOINT}, {0, 2}, {}, {}, {1, 0}, {1, 2, 3, 4})).size());
}

TEST(Robustness, GeometryVectorOffsets) {
    // More vertices than there is room for
    EXPECT_THROW(geometries(makeVector({GeometryType::LINESTRING}, {}, {0, 0xffffffffu}, {}, {}, {1, 2})),
                 std::runtime_error);
    // Enough vertices to look right if each took one value, but they take two
    EXPECT_THROW(geometries(makeVector({GeometryType::POLYGON}, {}, {0, 1}, {0, 5}, {}, {1, 2, 3, 4, 5, 6})),
                 std::runtime_error);
    EXPECT_THROW(geometries(makeVector({GeometryType::MULTIPOINT}, {0, 5}, {}, {}, {}, {1, 2, 3, 4, 5, 6})),
                 std::runtime_error);
    // Offsets which go backwards
    EXPECT_THROW(geometries(makeVector({GeometryType::LINESTRING, GeometryType::LINESTRING},
                                       {},
                                       {0, 5, 3},
                                       {},
                                       {},
                                       std::vector<std::int32_t>(20))),
                 std::runtime_error);
    // Offsets which are missing
    EXPECT_THROW(geometries(makeVector({GeometryType::POLYGON}, {}, {}, {}, {}, {1, 2})), std::runtime_error);
    EXPECT_THROW(geometries(makeVector({GeometryType::MULTIPOLYGON}, {0, 1}, {0, 1}, {}, {}, {1, 2})),
                 std::runtime_error);
}

TEST(Robustness, GeometryVectorVertexOffsets) {
    // Indexes of vertices which are not there
    EXPECT_THROW(geometries(makeVector({GeometryType::MULTIPOINT}, {0, 1}, {}, {}, {1000000}, {1, 2})),
                 std::runtime_error);
    EXPECT_THROW(geometries(makeVector({GeometryType::POINT}, {}, {}, {}, {0x80000000u}, {1, 2})), std::runtime_error);
    EXPECT_THROW(geometries(makeVector({GeometryType::POINT}, {}, {}, {}, {1}, {1, 2})), std::runtime_error);
    EXPECT_THROW(geometries(makeVector({GeometryType::LINESTRING}, {}, {0, 2}, {}, {0, 7}, {1, 2})),
                 std::runtime_error);
    EXPECT_THROW(geometries(makeVector({GeometryType::LINESTRING}, {}, {0, 3}, {}, {0, 0}, {1, 2})),
                 std::runtime_error);
}

TEST(Robustness, GeometryVectorUnusedData) {
    // Data which nothing refers to
    EXPECT_THROW(geometries(makeVector({GeometryType::LINESTRING}, {}, {0, 1}, {}, {}, {1, 2, 3, 4})),
                 std::runtime_error);
    EXPECT_THROW(geometries(makeVector({GeometryType::MULTIPOINT}, {0, 1}, {}, {}, {0, 1}, {1, 2, 3, 4})),
                 std::runtime_error);
}

TEST(Robustness, GeometryVectorTriangles) {
    const auto triangles = [](std::uint32_t count, std::vector<std::uint32_t> indexes) {
        return geometries(geometry::FlatGpuVector({GeometryType::POLYGON},
                                                  /*triangleCounts=*/{count},
                                                  std::move(indexes),
                                                  /*vertices=*/{0, 0, 1, 0, 1, 1},
                                                  geometry::TopologyVector({}, {0, 1}, {0, 3})));
    };
    EXPECT_EQ(1u, triangles(1, {0, 1, 2}).size());
    // Triangles for which there are no indexes
    EXPECT_THROW(triangles(2, {0, 1, 2}), std::runtime_error);
    EXPECT_THROW(triangles(0x55555556u, {0, 1, 2}), std::runtime_error);
    // Indexes of vertices which are not there
    EXPECT_THROW(triangles(1, {0, 1, 3}), std::runtime_error);
    EXPECT_THROW(triangles(1, {0xffffffffu, 1, 2}), std::runtime_error);
    // Indexes which nothing refers to
    EXPECT_THROW(triangles(1, {0, 1, 2, 0, 1, 2}), std::runtime_error);
}

TEST(Robustness, GeometryVectorAccessors) {
    const auto vector = makeVector({GeometryType::LINESTRING}, {}, {0, 2}, {}, {0, 5}, {1, 2, 3, 4});
    EXPECT_NO_THROW(vector.getVertex(0));
    EXPECT_ANY_THROW(vector.getVertex(1));
    EXPECT_ANY_THROW(vector.getVertex(2));
}

TEST(Robustness, TopologyLengths) {
    using decoder::GeometryDecoder;
    std::vector<std::uint32_t> offsets;
    const std::vector<GeometryType> types = {GeometryType::MULTIPOLYGON, GeometryType::MULTIPOLYGON};

    GeometryDecoder::decodeRootLengthStream(types, {1, 2}, GeometryType::POLYGON, offsets);
    EXPECT_EQ((std::vector<std::uint32_t>{0, 1, 3}), offsets);

    // The offsets don't fit in 32 bits
    EXPECT_THROW(GeometryDecoder::decodeRootLengthStream(types, {0xffffffffu, 2}, GeometryType::POLYGON, offsets),
                 std::runtime_error);
    // Lengths are missing
    EXPECT_THROW(GeometryDecoder::decodeRootLengthStream(types, {1}, GeometryType::POLYGON, offsets),
                 std::runtime_error);

    util::MemoryBudget budget(1 << 20);
    std::vector<std::uint32_t> level1;
    const std::vector<std::uint32_t> root = {0, 1, 3};
    GeometryDecoder::decodeLevel1LengthStream(types, root, {4, 5, 6}, false, level1, budget);
    EXPECT_EQ((std::vector<std::uint32_t>{0, 4, 9, 15}), level1);

    // Lengths are missing
    EXPECT_THROW(GeometryDecoder::decodeLevel1LengthStream(types, root, {4, 5}, false, level1, budget),
                 std::runtime_error);
    // The offsets don't belong to the types
    EXPECT_THROW(GeometryDecoder::decodeLevel1LengthStream(types, {0, 1}, {4, 5, 6}, false, level1, budget),
                 std::runtime_error);
    // The number of offsets to follow is enormous
    EXPECT_THROW(GeometryDecoder::decodeLevel1LengthStream(types, {0, 1, 0x7fffffffu}, {4}, false, level1, budget),
                 std::runtime_error);

    // (the outputs of failed calls are not meaningful)
    GeometryDecoder::decodeLevel1LengthStream(types, root, {4, 5, 6}, false, level1, budget);
    std::vector<std::uint32_t> level2;
    GeometryDecoder::decodeLevel2LengthStream(
        types, root, level1, {1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15}, level2, budget);
    EXPECT_EQ(16u, level2.size());
    EXPECT_EQ(120u, level2.back());
    EXPECT_THROW(GeometryDecoder::decodeLevel2LengthStream(types, root, level1, {1, 2}, level2, budget),
                 std::runtime_error);
    EXPECT_THROW(GeometryDecoder::decodeLevel2LengthStream(types, root, {0, 0x7fffffffu}, {1}, level2, budget),
                 std::runtime_error);
}

#pragma endregion

#pragma region Mutated tiles

// Change each byte of small tiles in turn. Any outcome is fine, other than a crash or another kind of exception.
TEST(Robustness, MutatedFixtures) {
    std::vector<std::filesystem::path> paths;
    for (const auto* directory : {"../test/expected/0x01/simple", "../test/synthetic/0x01", "../test/synthetic/0x02"}) {
        std::error_code error;
        for (const auto& entry : std::filesystem::directory_iterator(directory, error)) {
            if (entry.is_regular_file() && entry.path().extension() == ".mlt" && entry.file_size() <= 400) {
                paths.push_back(entry.path());
            }
        }
    }
    std::ranges::sort(paths);
    ASSERT_FALSE(paths.empty());
    if (paths.size() > 40) {
        paths.resize(40);
    }

    std::size_t decoded = 0;
    for (const auto& path : paths) {
        std::ifstream file(path, std::ios::binary);
        const Bytes original{std::istreambuf_iterator<char>(file), std::istreambuf_iterator<char>()};
        for (std::size_t i = 0; i < original.size(); ++i) {
            for (const std::uint8_t value : {0x00, 0x01, 0x7f, 0x80, 0xff, static_cast<int>(original[i] ^ 0x08)}) {
                auto mutated = original;
                mutated[i] = value;
                try {
                    decodeTile(mutated);
                } catch (const std::runtime_error&) {
                    // invalid tiles are rejected like this
                }
                ++decoded;
            }
        }
    }
    EXPECT_GT(decoded, 1000u);
}

#pragma endregion
