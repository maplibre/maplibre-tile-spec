#include <mlt/decoder.hpp>
#include <mlt/metadata/tileset.hpp>

#include <cstring>
#include <iostream>
#include <fstream>
#include <string>

using namespace std::string_literals;
using namespace std::string_view_literals;

#include <mlt/json.hpp>
#include "synthetic-geojson.hpp"

namespace {

std::vector<std::ifstream::char_type> loadFile(const std::string& path) {
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

int main(int argc, char** argv) {
    if ((argc != 2 && argc != 5) || !std::strcmp(argv[1], "--help")) {
        std::cerr << "Decode a MapLibre Vector Tile and output it as JSON or GeoJSON.\n"
                  << "GeoJSON is used if tile coordinates are provided.\n\n"
                  << "Usage: " << argv[0] << " <tile file> [<z> <x> <y>]\n";
        return 1;
    }

    std::uint32_t x = 0;
    std::uint32_t y = 0;
    std::uint32_t z = 0;
    const bool geoJSON = (argc == 5);
    if (geoJSON) {
        z = static_cast<std::uint32_t>(std::stoul(argv[2]));
        x = static_cast<std::uint32_t>(std::stoul(argv[3]));
        y = static_cast<std::uint32_t>(std::stoul(argv[4]));
    }

    const std::string baseName = argv[1];
    auto buffer = loadFile(baseName);
    if (buffer.empty()) {
        std::cerr << "Failed to load " + baseName + "\n";
        return 1;
    }

    mlt::Decoder decoder;
    const auto tileData = decoder.decode({buffer.data(), buffer.size()});

    nlohmann::json tileJSON;
    if (geoJSON) {
        tileJSON = mlt::json::toJSON(tileData, {.x = x, .y = y, .z = z}, true);
    } else {
        tileJSON = mlt::test::toFeatureCollection(tileData);
    }
    std::cout << tileJSON.dump(2, ' ', false, nlohmann::json::error_handler_t::replace);

    return 0;
}
