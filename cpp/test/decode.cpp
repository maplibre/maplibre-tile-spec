#include <gtest/gtest.h>

#include <decoder.hpp>
#include <metadata/tileset.hpp>

#include <iostream>
#include <filesystem>
#include <fstream>
#include <string>
using namespace std::string_literals;
using namespace std::string_view_literals;

namespace {
std::vector<std::ifstream::char_type> loadFile(const std::filesystem::path& path) {
    std::ifstream file(path, std::ios::binary | std::ios::ate);
    if (file.is_open()) {
        const std::size_t size = file.tellg();
        file.seekg(0);

        std::vector<std::ifstream::char_type> buffer(size);
        if (file.read(buffer.data(), size)) {
            return buffer;
        }
    }
    return {};
}

const auto basePath = "../test/expected"s;

auto loadTile(std::string path) {
    auto buffer = loadFile(path);
    EXPECT_FALSE(buffer.empty());

    auto metadataBuffer = loadFile(path + ".meta.pbf");
    EXPECT_FALSE(metadataBuffer.empty());

    using mlt::metadata::tileset::TileSetMetadata;
    TileSetMetadata metadata;
    EXPECT_TRUE(metadata.read({metadataBuffer.data(), metadataBuffer.size()}));

    mlt::decoder::Decoder decoder;
    return decoder.decode({buffer.data(), buffer.size()}, metadata);
}

} // namespace

TEST(Decode, SimplePointBoolean) {
    const auto tile = loadTile(basePath + "/simple/point-boolean.mlt");

    const auto* mltLayer = tile.getLayer("layer");
    EXPECT_TRUE(mltLayer);
    EXPECT_EQ(mltLayer->getVersion(), 1); // TODO: doesn't match JS version
    EXPECT_EQ(mltLayer->getName(), "layer");
    EXPECT_EQ(mltLayer->getTileExtent(), 4096);
    EXPECT_EQ(mltLayer->getFeatures().size(), 1);

    const auto& feature = mltLayer->getFeatures().front();
    EXPECT_EQ(feature.getID(), 1);
    EXPECT_EQ(feature.getExtent(), 4096);
    // TODO: geometry
    // TODO: properties
    // TODO: GeoJSON
}

TEST(Decode, SimpleLineBoolean) {
    const auto tile = loadTile(basePath + "/simple/line-boolean.mlt");
}

TEST(Decode, SimplePolygonBoolean) {
    const auto tile = loadTile(basePath + "/simple/polygon-boolean.mlt");
}

TEST(Decode, SimpleMultiPointBoolean) {
    const auto tile = loadTile(basePath + "/simple/multipoint-boolean.mlt");
}

TEST(Decode, SimpleMultiLineBoolean) {
    const auto tile = loadTile(basePath + "/simple/multiline-boolean.mlt");
}

TEST(Decode, SimpleMultiPolygonBoolean) {
    // TODO: not fully supported
    // const auto tile = loadTile(basePath + "/simple/multipolygon-boolean.mlt");
}

TEST(Decode, Bing) {
    // TODO: not fully supported
    // const auto tile = loadTile(basePath + "/bing/4-13-6.mlt");
}

TEST(Decode, OMT) {
    // TODO: not fully supported
    // const auto tile = loadTile(basePath + "/omt/2_2_2.mlt");
}
