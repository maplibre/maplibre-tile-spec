#include "synthetic_mlt_generator.hpp"

#include <mlt/coordinate.hpp>
#include <mlt/decoder.hpp>
#include <mlt/geometry.hpp>
#include <mlt/metadata/tileset.hpp>
#if MLT_WITH_JSON
#include <mlt/json.hpp>
#include <mlt/util/json_diff.hpp>
#endif

#include <gtest/gtest.h>

#include <algorithm>
#include <filesystem>
#include <fstream>
#include <functional>
#include <map>
#include <regex>
#include <set>
#include <sstream>

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

mlt::MapLibreTile decodeTile(const std::vector<std::uint8_t>& bytes) {
    constexpr bool supportFastPFOR = true;
    return mlt::Decoder(supportFastPFOR).decode({reinterpret_cast<const char*>(bytes.data()), bytes.size()});
}

#if MLT_WITH_JSON
void expectDecodedTilesEquivalent(const std::string& fixtureName,
                                  const std::vector<std::uint8_t>& expectedBytes,
                                  const std::vector<std::uint8_t>& generatedBytes) {
    const auto expectedTile = decodeTile(expectedBytes);
    const auto generatedTile = decodeTile(generatedBytes);

    // Keep tile coordinate aligned with other decode/json regression tests.
    const mlt::TileCoordinate tileCoordinate{.x = 3, .y = 5, .z = 7};
    const auto expectedJSON = mlt::json::toJSON(expectedTile, tileCoordinate, true);
    const auto generatedJSON = mlt::json::toJSON(generatedTile, tileCoordinate, true);
    const auto diffJSON = mlt::util::diff(expectedJSON, generatedJSON, {}, mlt::util::JSONComparer());

    ASSERT_TRUE(diffJSON.empty()) << "Decoded JSON mismatch for fixture '" << fixtureName
                                  << "'\nDiff: " << diffJSON.dump(2);
}
#endif

/// Registry entry: a generator plus an optional reason for skipping binary equality checks.
struct RegistryEntry {
    std::function<SyntheticMltGenerator::GeneratedTile()> generator;
    std::string binaryExceptionReason; ///< if non-empty, the binary check is skipped with this message
};
using GeneratorRegistry = std::map<std::string, RegistryEntry>;

/// A pattern entry: an ECMAScript regex matched against the fixture name, plus a skip reason.
struct ExceptionPattern {
    std::regex pattern;
    std::string reason;
};

/// These cases don't require the generated fixture bytes to match.
/// The decoded JSON still has to match.
const std::vector<ExceptionPattern>& binaryExceptionPatterns() {
    static const std::vector<ExceptionPattern> patterns = {
        {.pattern = std::regex{"(mix_.*_tes)"},
         .reason = "Tessellation triangle ordering is arbitrary; byte comparison is not reliable"},
        {.pattern = std::regex{"(poly(_.*)?_tes)"},
         .reason = "Tessellation triangle ordering is arbitrary; byte comparison is not reliable"},
        {.pattern = std::regex{"(.*_fsst$)"},
         .reason = "FSST symbol selection is heuristic; byte comparison is not reliable"},
    };
    return patterns;
}

/// Returns the skip reason for a fixture name, or empty string if the fixture is enabled.
std::string skipReasonFor(const std::string& name) {
    for (const auto& dp : binaryExceptionPatterns()) {
        if (std::regex_match(name, dp.pattern)) {
            return dp.reason;
        }
    }
    return {};
}

const GeneratorRegistry& generatorRegistry() {
    static const GeneratorRegistry registry = []() {
        GeneratorRegistry reg;

        // Insert a tile: check against the exceptions to set skipReason automatically.
        const auto add = [&](SyntheticMltGenerator::GeneratedTile tile) {
            auto name = tile.name;
            reg.emplace(std::move(name),
                        RegistryEntry{.generator = [t = std::move(tile)]() { return t; },
                                      .binaryExceptionReason = skipReasonFor(name)});
        };

        // Insert all tiles from a collection.
        const auto addAll = [&](SyntheticMltGenerator::GeneratedTileVec tiles) {
            std::ranges::for_each(tiles, add);
        };

        // Basic geometry
        add(SyntheticMltGenerator::generatePoints());
        add(SyntheticMltGenerator::generateLine());
        add(SyntheticMltGenerator::generateLineZeroLength());
        addAll(SyntheticMltGenerator::generateLineMorton());
        addAll(SyntheticMltGenerator::generatePolygons());
        addAll(SyntheticMltGenerator::generateMultiPoints());
        addAll(SyntheticMltGenerator::generateMultiPointsMorton());
        addAll(SyntheticMltGenerator::generateMultiLineStrings());
        addAll(SyntheticMltGenerator::generateMultiLineStringsMorton());
        addAll(SyntheticMltGenerator::generateExtents());

        // IDs
        addAll(SyntheticMltGenerator::generateIdsCollection());

        // Properties
        addAll(SyntheticMltGenerator::generateProperties());
        addAll(SyntheticMltGenerator::generateFpfAlignments());

        // Mixed geometry combinations (mix_2_*, mix_3_*, ..., A-A, A-B-A variants)
        addAll(SyntheticMltGenerator::generateMixed());

        // Shared dictionaries
        addAll(SyntheticMltGenerator::generateSharedDictionaries());

        return reg;
    }();
    return registry;
}

/// Enumerate every .mlt fixture file found under basePath, returning sorted stem names.
std::vector<std::string> discoverAllFixtures() {
    std::vector<std::string> result;
    std::error_code ec;
    if (std::filesystem::exists(basePath)) {
        for (const auto& entry : std::filesystem::directory_iterator(basePath, ec)) {
            if (!ec && entry.path().extension() == ".mlt") {
                const auto stem = entry.path().stem().string();
                result.push_back(stem);
            }
        }
    }
    std::ranges::sort(result);
    return result;
}

// ---------------------------------------------------------------------------
// Value-parameterized test: one test instance per fixture file.
// - If a C++ generator is registered for the fixture name: generate and compare.
// - If no generator is registered: SKIP (fixture is not yet checked).
// ---------------------------------------------------------------------------

class SyntheticFixtureTest : public ::testing::TestWithParam<std::string> {};

TEST_P(SyntheticFixtureTest, MatchesGenerator) {
    const auto& name = GetParam();
    const auto& reg = generatorRegistry();

    const auto it = reg.find(name);
    if (it == reg.end()) {
        GTEST_SKIP() << "No C++ generator registered for fixture: " << name;
    }

    const auto& entry = it->second;
    const auto generated = entry.generator();
    const auto filePath = basePath + name + ".mlt";
    const auto fileContents = readBinaryFile(filePath);

    // If the results are identical, there's no need to decode and compare JSON
    if (fileContents == generated.bytes) {
        SUCCEED();
        return;
    }

#if MLT_WITH_JSON
    expectDecodedTilesEquivalent(name, fileContents, generated.bytes);
#else
#warning "MLT_WITH_JSON is disabled; skipping decoded semantic comparison and relying on byte-level comparison only"
#endif

    if (!entry.binaryExceptionReason.empty()) {
        SUCCEED() << "Byte equality not checked: " << entry.binaryExceptionReason;
    } else {
        EXPECT_EQ(fileContents, generated.bytes);
    }
}

INSTANTIATE_TEST_SUITE_P(AllSyntheticFixtures,
                         SyntheticFixtureTest,
                         ::testing::ValuesIn(discoverAllFixtures()),
                         [](const ::testing::TestParamInfo<std::string>& info) { return info.param; });

// Reports fixture files that have no registered generator.
TEST(SyntheticFixtureCoverage, AllFixturesHaveGenerators) {
    const auto allFixtures = discoverAllFixtures();
    const auto& reg = generatorRegistry();

    // Split out the located fixtures into categories:
    // - checked: have a registered generator and are checked by tests
    // - jsonChecked: have a registered generator but binary equiality is not required
    // - unchecked: have no registered generator (and are therefore skipped by tests)
    // - allMatched: the union of checked and jsonChecked; used to find missing generated cases
    std::set<std::string> checked, unchecked, jsonChecked, allMatched;
    for (const auto& name : allFixtures) {
        const auto hit = reg.find(name);
        if (hit == reg.end()) {
            unchecked.insert(name);
            continue;
        }

        allMatched.insert(name);
        (hit->second.binaryExceptionReason.empty() ? checked : jsonChecked).insert(name);
    }

    // Create a set consisting of the names of all the generated cases
    std::set<std::string> allGeneratedNames;
    for (const auto& [name, _] : reg) {
        allGeneratedNames.insert(name);
    }

    // Find extra generated cases by subtracting the matched set from the generated set
    std::set<std::string> missing;
    std::ranges::set_difference(allGeneratedNames, allMatched, std::inserter(missing, missing.end()));

    // Report the results
    std::ostringstream stream;

    stream << checked.size() << " fixture file(s) have generators and are checked by tests.\n"
           << jsonChecked.size() << " fixture file(s) are not byte-exact, but JSON equivalent\n";

    stream << missing.size() << " generated case(s) do not match any located fixture file.\n";
    for (const auto& n : missing) {
        stream << "  " << n << "\n";
    }

    stream << unchecked.size() << " fixture file(s) have no registered C++ generator.\n";
    for (const auto& n : unchecked) {
        stream << "  " << n << "\n";
    }

    // The test fails only if there are generated cases with no matching fixture file.
    if (!missing.empty()) {
        FAIL() << stream.str();
    } else {
        GTEST_SKIP() << stream.str();
    }
}

} // namespace
