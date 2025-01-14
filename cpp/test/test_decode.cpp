#include <gtest/gtest.h>

#include <tileset_metadata.hpp>

#include <filesystem>
#include <fstream>
#include <regex>

TEST(Decode, metadata) {
    constexpr auto path = "../test/expected/simple";
    const std::regex metadataFilePattern{".*\\.mlt.meta.pbf"};
    for (const auto &entry : std::filesystem::directory_iterator(path)) {
        std::smatch match;
        const auto fileName = entry.path().filename().string();
        if (!entry.is_regular_file() || !std::regex_match(fileName, match, metadataFilePattern)) {
            continue;
        }

        std::cerr << "  Loading " << fileName << " ... ";

        std::ifstream file(entry.path(), std::ios::binary | std::ios::ate);
        EXPECT_TRUE(file.is_open());

        std::size_t size = file.tellg();
        file.seekg(0);

        std::vector<std::ifstream::char_type> buffer(size);
        EXPECT_TRUE(file.read(buffer.data(), size));

        std::cerr << "  Parsing... " << std::endl;

        using mlt::tileset_metadata::TileSetMetadata;
        EXPECT_TRUE(TileSetMetadata{}.read({buffer.data(), buffer.size()}));
    }
}
