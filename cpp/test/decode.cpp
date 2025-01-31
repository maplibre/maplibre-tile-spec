#include <gtest/gtest.h>

#include <decoder.hpp>
#include <metadata/tileset.hpp>

#include <iostream>
#include <filesystem>
#include <fstream>
#include <regex>
#include <string>
using namespace std::string_literals;
using namespace std::string_view_literals;

namespace {
std::vector<std::filesystem::path> findFiles(const std::filesystem::path& base, const std::regex& pattern) {
    std::vector<std::filesystem::path> results;
    for (const auto& entry : std::filesystem::directory_iterator(base)) {
        std::smatch match;
        const auto fileName = entry.path().filename().string();
        if (entry.is_regular_file() && std::regex_match(fileName, match, pattern)) {
            results.push_back(entry.path());
        }
    }
    return results;
}

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

std::optional<mlt::MapLibreTile> loadTile(const std::string& path) {
    auto metadataBuffer = loadFile(path + ".meta.pbf");
    if (metadataBuffer.empty()) {
        std::cout << "    Failed to load " + path + ".meta.pbf\n";
        return {};
    }

    using mlt::metadata::tileset::TileSetMetadata;
    TileSetMetadata metadata;
    if (!metadata.read({metadataBuffer.data(), metadataBuffer.size()})) {
        std::cout << "    Failed to parse " + path + ".meta.pbf\n";
        return {};
    }

    auto buffer = loadFile(path);
    if (buffer.empty()) {
        std::cout << "    Failed to load " + path + "\n";
        return {};
    }

    mlt::decoder::Decoder decoder;
    return decoder.decode({buffer.data(), buffer.size()}, metadata);
}

} // namespace

TEST(Decode, SimplePointBoolean) {
    const auto tile = loadTile(basePath + "/simple/point-boolean.mlt");
    ASSERT_TRUE(tile);

    const auto* mltLayer = tile->getLayer("layer");
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
    ASSERT_TRUE(tile);
}

TEST(Decode, SimplePolygonBoolean) {
    const auto tile = loadTile(basePath + "/simple/polygon-boolean.mlt");
    ASSERT_TRUE(tile);
}

TEST(Decode, SimpleMultiPointBoolean) {
    const auto tile = loadTile(basePath + "/simple/multipoint-boolean.mlt");
    ASSERT_TRUE(tile);
}

TEST(Decode, SimpleMultiLineBoolean) {
    const auto tile = loadTile(basePath + "/simple/multiline-boolean.mlt");
    ASSERT_TRUE(tile);
}

TEST(Decode, SimpleMultiPolygonBoolean) {
    const auto tile = loadTile(basePath + "/simple/multipolygon-boolean.mlt");
    ASSERT_TRUE(tile);
}

TEST(Decode, Bing) {
    const auto tile = loadTile(basePath + "/bing/4-13-6.mlt");
    ASSERT_TRUE(tile);
}

TEST(Decode, OMT) {
    const auto tile = loadTile(basePath + "/omt/2_2_2.mlt");
    ASSERT_TRUE(tile);
}

// Make sure everything else loads without errors
TEST(Decode, AllBing) {
    const std::regex metadataFilePattern{".*\\.mlt"};
    for (const auto& path : findFiles(basePath + "/bing", metadataFilePattern)) {
        std::cout << "  Loading " << path.filename().string() << " ...\n";
        loadTile(path);
    }
}
TEST(Decode, AllOMT) {
    const std::regex metadataFilePattern{".*\\.mlt"};
    for (const auto& path : findFiles(basePath + "/omt", metadataFilePattern)) {
        std::cout << "  Loading " << path.filename().string() << " ...\n";
        loadTile(path);
    }
}