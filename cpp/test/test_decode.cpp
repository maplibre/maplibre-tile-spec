#include <gtest/gtest.h>

#include <mlt/decoder.hpp>
#include <mlt/metadata/tileset.hpp>
#include <mlt/util/buffer_stream.hpp>
#include <mlt/projection.hpp>

#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <regex>
#include <string>

using namespace std::string_literals;
using namespace std::string_view_literals;

#if MLT_WITH_JSON
#include <mlt/geojson.hpp>
#include <mlt/util/json_diff.hpp>
#endif

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

bool writeFile(const std::filesystem::path& path, const std::string& data) {
    std::ofstream file(path, std::ios::binary | std::ios::out);
    if (file.is_open()) {
        file.write(data.data(), data.size());
        return !file.fail();
    }
    return false;
}

const auto basePath = "../test/expected"s;

#if MLT_WITH_JSON
auto dump(const nlohmann::json& json) {
    return json.dump(2, ' ', false, nlohmann::json::error_handler_t::replace);
}
#endif

std::pair<std::optional<mlt::MapLibreTile>, std::string> loadTile(const std::string& path) {
    const auto tileData = loadFile(path);
    if (tileData.empty()) {
        return {std::nullopt, "Failed to read tile data"};
    }

    auto tile = mlt::Decoder().decodeTile({(tileData.data()), tileData.size()});

#if MLT_WITH_JSON
    // Load the GeoJSON file, if present
    auto jsonBuffer = loadFile(path + ".geojson");
    if (!jsonBuffer.empty()) {
        using json = nlohmann::json;
        const json expectedJSON = json::parse(
            jsonBuffer, nullptr, /*allow_exceptions=*/false, /*ignore_comments=*/true);

        // Convert the tile we loaded to GeoJSON
        const auto actualJSON = mlt::geojson::toGeoJSON(tile, {.x = 3, .y = 5, .z = 7});

        // Compare the two
        const auto diffJSON = mlt::util::diff(expectedJSON, actualJSON, {});
        if (diffJSON.empty()) {
            std::error_code ec;
            std::filesystem::remove(path + ".diff.geojson", ec);
        } else {
            // Dump the unexpected results to files for review
            writeFile(path + ".new.geojson", dump(actualJSON));
            writeFile(path + ".diff.geojson", dump(diffJSON));
            std::cout << path << ":" << diffJSON.size() << " unexpected differences\n";
        }
    }
#endif // MLT_WITH_JSON

    return {std::move(tile), std::string()};
}

} // namespace

TEST(Decode, SimplePointBoolean) {
    const auto tile = loadTile(basePath + "/simple/point-boolean.mlt").first;
    ASSERT_TRUE(tile);

    const auto* mltLayer = tile->getLayer("layer");
    EXPECT_TRUE(mltLayer);
    EXPECT_EQ(mltLayer->getName(), "layer");
    EXPECT_EQ(mltLayer->getVersion(), 1); // TODO: doesn't match JS version
    EXPECT_EQ(mltLayer->getExtent(), 4096);
    EXPECT_EQ(mltLayer->getFeatures().size(), 1);

    const auto& feature = mltLayer->getFeatures().front();
    EXPECT_EQ(feature.getID(), 1);
}

TEST(Decode, SimpleLineBoolean) {
    const auto tile = loadTile(basePath + "/simple/line-boolean.mlt");
    // TODO: check properties, geometry, etc.
    ASSERT_TRUE(tile.first);
}

TEST(Decode, SimplePolygonBoolean) {
    const auto tile = loadTile(basePath + "/simple/polygon-boolean.mlt");
    ASSERT_TRUE(tile.first);
}

TEST(Decode, SimpleMultiPointBoolean) {
    const auto tile = loadTile(basePath + "/simple/multipoint-boolean.mlt");
    ASSERT_TRUE(tile.first);
}

TEST(Decode, SimpleMultiLineBoolean) {
    const auto tile = loadTile(basePath + "/simple/multiline-boolean.mlt");
    ASSERT_TRUE(tile.first);
}

TEST(Decode, SimpleMultiPolygonBoolean) {
    const auto tile = loadTile(basePath + "/simple/multipolygon-boolean.mlt");
    ASSERT_TRUE(tile.first);
}

TEST(Decode, Bing) {
    const auto tile = loadTile(basePath + "/bing/4-13-6.mlt");
    ASSERT_TRUE(tile.first) << tile.second;
}

TEST(Decode, OMT) {
    const auto tile = loadTile(basePath + "/omt/2_2_2.mlt");
    ASSERT_TRUE(tile.first);
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
    std::set<std::string> skipFiles = {"12_2132_2734.mlt",
                                       "7_66_84.mlt",
                                       "4_8_10.mlt",
                                       "11_1064_1367.mlt",
                                       "8_134_171.mlt",
                                       "5_16_21.mlt",
                                       "3_4_5.mlt",
                                       "6_32_41.mlt",
                                       "10_532_682.mlt"};
    for (const auto& path : findFiles(basePath + "/omt", metadataFilePattern)) {
        if (skipFiles.contains(path.filename().string())) {
            std::cout << "Skipped: " << path.filename().string() << "\n";
            continue;
        }
        try {
            if (auto result = loadTile(path); result.first) {
                std::cout << "Loaded: " << path.filename().string() << "\n";
            } else {
                std::cout << "Not Loaded: " << path.filename().string() << ": " << result.second << "\n";
            }
        } catch (const std::exception& e) {
            FAIL() << path.filename().string() << " Failed: " << e.what();
        }
    }
}
