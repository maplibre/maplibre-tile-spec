#include <gtest/gtest.h>

#include <mlt/decoder.hpp>
#include <mlt/metadata/tileset.hpp>
#include <mlt/projection.hpp>

#include <iostream>
#include <filesystem>
#include <fstream>
#include <regex>
#include <string>

using namespace std::string_literals;
using namespace std::string_view_literals;

#if MLT_WITH_JSON
#include <mlt/geojson.hpp>
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
    auto tile = decoder.decode({buffer.data(), buffer.size()}, metadata);

#if MLT_WITH_JSON
    // Load the GeoJSON file, if present
    auto jsonBuffer = loadFile(path + ".geojson");
    if (!jsonBuffer.empty()) {
        using json = nlohmann::json;
        const json::parser_callback_t callback = nullptr;
        const json expectedJSON = json::parse(
            jsonBuffer, nullptr, /*allow_exceptions=*/false, /*ignore_comments=*/true);

        // Convert the tile we loaded to GeoJSON
        const auto actualJSON = mlt::GeoJSON::toGeoJSON(tile, {3, 5, 7});

        const auto actualJSONStr = actualJSON.dump(2, ' ', false, json::error_handler_t::replace);
        writeFile(path + ".new.geojson", actualJSONStr);

        // Compare the two
        auto diffJSON = json::diff(expectedJSON, actualJSON);

        // Eliminate differences due to extremely small floating-point value changes
        constexpr auto epsilon = 1.0e-15;
        for (auto i = diffJSON.begin(); i != diffJSON.end();) {
            auto& node = *i;
            assert(node.is_object());
            if (node["op"] == "replace" && node["value"].is_number()) {
                const auto& path = node["path"];
                if (path.is_string()) {
                    const auto ptr = json::json_pointer{path};
                    const auto& expectedNode = expectedJSON.at(ptr);
                    const auto& actualNode = actualJSON.at(ptr);
                    if (expectedNode.is_number_float() && actualNode.is_number_float()) {
                        const double expectedValue = expectedNode;
                        const double actualValue = actualNode;
                        const double error = std::fabs(expectedValue - actualValue) / actualValue;
                        if (error < epsilon) {
                            i = diffJSON.erase(i);
                            continue;
                        } else {
                            node["previous"] = expectedNode;
                            node["error"] = error;
                        }
                    }
                }
            }
            ++i;
        }

        if (diffJSON.empty()) {
            std::error_code ec;
            std::filesystem::remove(path + ".diff.geojson", ec);
        } else {
            const auto diffJSONStr = diffJSON.dump(2, ' ', false, json::error_handler_t::replace);
            writeFile(path + ".diff.geojson", diffJSONStr);

            std::cout << path << ":\n"
                      << "expected=" << expectedJSON.dump(2, ' ', false, json::error_handler_t::replace) << "\n"
                      << "actual=" << actualJSON.dump(2, ' ', false, json::error_handler_t::replace) << "\n"
                      << " diff=" << diffJSON.dump(2, ' ', false, json::error_handler_t::replace) << "\n";
        }
    }
#endif // MLT_WITH_JSON

    return tile;
}

} // namespace
/*
TEST(Decode, SimplePointBoolean) {
    const auto tile = loadTile(basePath + "/simple/point-boolean.mlt");
    ASSERT_TRUE(tile);

    const auto* mltLayer = tile->getLayer("layer");
    EXPECT_TRUE(mltLayer);
    EXPECT_EQ(mltLayer->getName(), "layer");
    EXPECT_EQ(mltLayer->getVersion(), 1); // TODO: doesn't match JS version
    EXPECT_EQ(mltLayer->getExtent(), 4096);
    EXPECT_EQ(mltLayer->getFeatures().size(), 1);

    const auto& feature = mltLayer->getFeatures().front();
    EXPECT_EQ(feature.getID(), 1);
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
*/
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