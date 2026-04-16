#include "synthetic_mlt_generator.hpp"

#include <gtest/gtest.h>

#include <mlt/coordinate.hpp>
#include <mlt/decoder.hpp>
#include <mlt/geometry.hpp>
#include <mlt/metadata/tileset.hpp>

#include <fstream>

using namespace mlt::test;

namespace {

constexpr auto basePath = "../test/synthetic/0x01/";

std::vector<uint8_t> readBinaryFile(const std::string& filePath) {
    std::ifstream file(filePath, std::ios::binary);
    if (file.is_open()) {
        return std::vector<uint8_t>((std::istreambuf_iterator<char>(file)), std::istreambuf_iterator<char>());
    } else {
        throw std::runtime_error("Unable to open file: " + filePath);
    }
}

void checkSynthetic(const SyntheticMltGenerator::GeneratedTile& generated) {
    const auto filePath = basePath + generated.name + ".mlt";
    const auto fileContents = readBinaryFile(filePath);
    EXPECT_EQ(fileContents, generated.bytes);
}

TEST(SyntheticMltGenerator, GeneratePoints) {
    checkSynthetic(SyntheticMltGenerator::generatePoints());
}

} // namespace
