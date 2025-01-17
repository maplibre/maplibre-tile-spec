#include <gtest/gtest.h>

#include <decoder.hpp>
#include <metadata/tileset.hpp>

#include <iostream>
#include <filesystem>
#include <fstream>
#include <regex>

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
} // namespace

TEST(Decode, metadata) {
    constexpr auto path = "../test/expected/simple";
    const std::regex metadataFilePattern{".*\\.mlt.meta.pbf"};
    for (const auto& path : findFiles(path, metadataFilePattern)) {
        std::cerr << "  Loading " << path.filename().string() << " ... ";
        const auto buffer = loadFile(path);
        EXPECT_FALSE(buffer.empty());

        std::cerr << "  Parsing... \n";

        using mlt::metadata::tileset::TileSetMetadata;
        EXPECT_TRUE(TileSetMetadata{}.read({buffer.data(), buffer.size()}));
    }
}

TEST(Decode, tile) {
    constexpr auto path = "../test/expected/simple";
    const std::regex metadataFilePattern{".*\\.mlt"};
    for (const auto& path : findFiles(path, metadataFilePattern)) {
        std::cerr << "  Loading " << path.filename().string() << " ... ";
        auto buffer = loadFile(path);
        EXPECT_FALSE(buffer.empty());

        auto metadataBuffer = loadFile(path.string() + ".meta.pbf");
        EXPECT_FALSE(metadataBuffer.empty());

        using mlt::metadata::tileset::TileSetMetadata;
        TileSetMetadata metadata;
        EXPECT_TRUE(metadata.read({metadataBuffer.data(), metadataBuffer.size()}));

        std::cerr << "  Parsing... \n";

        mlt::decoder::Decoder decoder;
        auto tile = decoder.decode({buffer.data(), buffer.size()}, metadata);
    }
}