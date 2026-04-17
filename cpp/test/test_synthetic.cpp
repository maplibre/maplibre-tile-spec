#include "synthetic_mlt_generator.hpp"

#include <gtest/gtest.h>

#include <algorithm>
#include <mlt/coordinate.hpp>
#include <mlt/decoder.hpp>
#include <mlt/geometry.hpp>
#include <mlt/metadata/tileset.hpp>

#include <filesystem>
#include <fstream>

using namespace mlt::test;

namespace {

/// Allow the tests to be run from a variety of working directories within the workspace
const std::string basePath = []() {
    std::string basePath = "test/synthetic/0x01/";
    for (int i = 0; i < 3; ++i) {
        if (std::filesystem::exists(basePath)) {
            return basePath;
        }
        basePath.insert(0, "../");
    }
    throw std::runtime_error("Fixture base path not found");
}();

/// Loads the contents of a binary file into a vector of bytes
std::vector<uint8_t> readBinaryFile(const std::string& filePath) {
    std::ifstream file(filePath, std::ios::binary);
    if (file.is_open()) {
        return std::vector<uint8_t>((std::istreambuf_iterator<char>(file)), std::istreambuf_iterator<char>());
    } else {
        throw std::runtime_error("Unable to open file: " + filePath);
    }
}

/// Compare a generated tile against a file
void checkSynthetic(const SyntheticMltGenerator::GeneratedTile& generated) {
    const auto filePath = basePath + generated.name + ".mlt";
    const auto fileContents = readBinaryFile(filePath);
    EXPECT_EQ(fileContents, generated.bytes);
}

/// Compare a collection of generated tiles against fixture files
void checkSynthetic(const SyntheticMltGenerator::GeneratedTiles& generatedTiles) {
    for (const auto& generated : generatedTiles) {
        checkSynthetic(generated);
    }
}

/// Compare a single tile from a collection
void checkSyntheticByName(const SyntheticMltGenerator::GeneratedTiles& generatedTiles, const std::string& name) {
    const auto it = std::ranges::find_if(generatedTiles, [&](const auto& t) { return t.name == name; });
    ASSERT_NE(it, generatedTiles.end()) << "Generated tile not found: " << name;
    checkSynthetic(*it);
}

TEST(SyntheticMltGenerator, GeneratePoints) {
    checkSynthetic(SyntheticMltGenerator::generatePoints());
}

TEST(SyntheticMltGenerator, GenerateLine) {
    checkSynthetic(SyntheticMltGenerator::generateLine());
}

TEST(SyntheticMltGenerator, GenerateLines) {
    checkSynthetic(SyntheticMltGenerator::generateLines());
}

TEST(SyntheticMltGenerator, GeneratePolyHole) {
    checkSynthetic(SyntheticMltGenerator::generatePolyHole());
}

TEST(SyntheticMltGenerator, GeneratePolygons) {
    checkSynthetic(SyntheticMltGenerator::generatePolygons());
}

TEST(SyntheticMltGenerator, GenerateMultiPoint) {
    checkSynthetic(SyntheticMltGenerator::generateMultiPoint());
}

TEST(SyntheticMltGenerator, GenerateMultiPoints) {
    checkSynthetic(SyntheticMltGenerator::generateMultiPoints());
}

TEST(SyntheticMltGenerator, GenerateMultiLine) {
    checkSynthetic(SyntheticMltGenerator::generateMultiLine());
}

TEST(SyntheticMltGenerator, GenerateMultiLineStrings) {
    checkSynthetic(SyntheticMltGenerator::generateMultiLineStrings());
}

TEST(SyntheticMltGenerator, GenerateExtent4096) {
    checkSynthetic(SyntheticMltGenerator::generateExtent4096());
}

TEST(SyntheticMltGenerator, GenerateExtent) {
    checkSynthetic(SyntheticMltGenerator::generateExtent());
}

// Why doesn't this work?
TEST(SyntheticMltGenerator, DISABLED_GenerateExtentLarge) {
    checkSynthetic(SyntheticMltGenerator::generateExtent(1073741824U));
}

TEST(SyntheticMltGenerator, GenerateIds) {
    checkSynthetic(SyntheticMltGenerator::generateIds());
}

TEST(SyntheticMltGenerator, GenerateIdsCollection) {
    checkSynthetic(SyntheticMltGenerator::generateIdsCollection());
}

// Currently not working because Java generates a nullable column and we don't.
// I think that's effectively a bug on the Java side because the column contains no nulls.
TEST(SyntheticMltGenerator, DISABLED_GeneratePropU64) {
    checkSynthetic(SyntheticMltGenerator::generatePropU64());
}

TEST(SyntheticMltGenerator, DISABLED_GenerateProperties) {
    checkSynthetic(SyntheticMltGenerator::generateProperties());
}

TEST(SyntheticMltGenerator, GenerateMixPtLine) {
    checkSynthetic(SyntheticMltGenerator::generateMixPtLine());
}

TEST(SyntheticMltGenerator, GenerateMixed) {
    checkSynthetic(SyntheticMltGenerator::generateMixed());
}

TEST(SyntheticMltGenerator, GenerateSharedDictNoSharedDict) {
    checkSyntheticByName(SyntheticMltGenerator::generateSharedDictionaries(), "props_no_shared_dict");
}

TEST(SyntheticMltGenerator, GenerateSharedDictBasic) {
    checkSyntheticByName(SyntheticMltGenerator::generateSharedDictionaries(), "props_shared_dict");
}

TEST(SyntheticMltGenerator, GenerateSharedDictNoStructName) {
    checkSyntheticByName(SyntheticMltGenerator::generateSharedDictionaries(), "props_shared_dict_no_struct_name");
}

TEST(SyntheticMltGenerator, GenerateSharedDictNoChildName) {
    checkSyntheticByName(SyntheticMltGenerator::generateSharedDictionaries(), "props_shared_dict_no_child_name");
}

// TODO: These cases still fail

TEST(SyntheticMltGenerator, DISABLED_GenerateSharedDictOneChild) {
    checkSyntheticByName(SyntheticMltGenerator::generateSharedDictionaries(), "props_shared_dict_one_child");
}

TEST(SyntheticMltGenerator, DISABLED_GenerateSharedDictTwoSamePrefix) {
    checkSyntheticByName(SyntheticMltGenerator::generateSharedDictionaries(), "props_shared_dict_2_same_prefix");
}

/// FSST string encoding is non-deterministic due to symbol selection heuristics.
/// We rely on round-trip testing for these.
TEST(SyntheticMltGenerator, DISABLED_GeneratePropsStrFsst) {
    checkSynthetic(SyntheticMltGenerator::generatePropsStrFsst());
}
TEST(SyntheticMltGenerator, DISABLED_GenerateSharedDictBasicFsst) {
    checkSyntheticByName(SyntheticMltGenerator::generateSharedDictionaries(), "props_shared_dict_fsst");
}
TEST(SyntheticMltGenerator, DISABLED_GenerateSharedDictOneChildFsst) {
    checkSyntheticByName(SyntheticMltGenerator::generateSharedDictionaries(), "props_shared_dict_one_child_fsst");
}
TEST(SyntheticMltGenerator, DISABLED_GenerateSharedDictNoStructNameFsst) {
    checkSyntheticByName(SyntheticMltGenerator::generateSharedDictionaries(), "props_shared_dict_no_struct_name_fsst");
}
TEST(SyntheticMltGenerator, DISABLED_GenerateSharedDictNoChildNameFsst) {
    checkSyntheticByName(SyntheticMltGenerator::generateSharedDictionaries(), "props_shared_dict_no_child_name_fsst");
}

} // namespace
