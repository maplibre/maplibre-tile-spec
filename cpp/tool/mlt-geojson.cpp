#include <mlt/decoder.hpp>
#include <mlt/metadata/tileset.hpp>

#include <protozero/pbf_message.hpp>
#include <mlt/metadata/tileset_protozero.hpp>

#include <cstring>
#include <iostream>
#include <fstream>
#include <string>

using namespace std::string_literals;
using namespace std::string_view_literals;

#include <mlt/geojson.hpp>

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
    if (argc != 2 || !std::strcmp(argv[1], "--help")) {
        std::cerr << "Usage: " << argv[0] << " <tile file>\n";
        return 1;
    }

    const std::string baseName = argv[1];
    auto metadataBuffer = loadFile(baseName + ".meta.pbf");
    if (metadataBuffer.empty()) {
        std::cout << "Failed to load " + baseName + ".meta.pbf\n";
        return 1;
    }

    // using mlt::metadata::tileset::TileSetMetadata;
    const auto metadata = mlt::metadata::tileset::read({metadataBuffer.data(), metadataBuffer.size()});
    if (!metadata) {
        std::cout << "Failed to parse " + baseName + ".meta.pbf\n";
        return 1;
    }

    auto buffer = loadFile(baseName);
    if (buffer.empty()) {
        std::cout << "Failed to load " + baseName + "\n";
        return 1;
    }

    mlt::decoder::Decoder decoder;
    const auto tileData = decoder.decode({buffer.data(), buffer.size()}, *metadata);
    const auto tileJSON = mlt::GeoJSON::toGeoJSON(tileData, {3, 5, 7});
    std::cout << tileJSON.dump(2, ' ', false, nlohmann::json::error_handler_t::replace);

    return 0;
}
