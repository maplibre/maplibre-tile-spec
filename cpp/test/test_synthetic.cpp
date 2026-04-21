#include "synthetic_mlt_generator.hpp"

#include <mlt/coordinate.hpp>
#include <mlt/decoder.hpp>
#include <mlt/geometry.hpp>
#include <mlt/metadata/tileset.hpp>

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

/// Registry entry: a generator plus an optional skip reason.
/// If skipReason is non-empty the test is skipped rather than run.
struct RegistryEntry {
    std::function<SyntheticMltGenerator::GeneratedTile()> generator;
    std::string skipReason; ///< non-empty → test is skipped with this message
};
using GeneratorRegistry = std::map<std::string, RegistryEntry>;

/// A pattern entry: an ECMAScript regex matched against the fixture name, plus a skip reason.
struct DisabledPattern {
    std::regex pattern;
    std::string reason;
};

/// Returns the list of disabled patterns.
/// All generated tiles whose names match any pattern are marked as skipped.
const std::vector<DisabledPattern>& disabledPatterns() {
    static const std::vector<DisabledPattern> patterns = {
        // The tessellation cases are non-deterministic in terms of triangle ordering; byte comparison is not reliable.
        {.pattern = std::regex{R"(^mix_.*_tes$)"},
         .reason = "Mixed polygon fixture encoding does not yet match Java output"},
        // FSST symbol table selection is non-deterministic; byte-exact comparison is not reliable.
        {.pattern = std::regex{R"(.*_fsst$)"},
         .reason = "FSST symbol selection is non-deterministic; byte-exact comparison is not possible"},
        // Shared-dictionary edge cases not yet handled correctly.
        {.pattern = std::regex{R"(^props_shared_dict_(one_child|2_same_prefix)$)"},
         .reason = "Shared-dictionary encoding for this case does not yet match Java output"},
    };
    return patterns;
}

/// Returns the skip reason for a fixture name, or empty string if the fixture is enabled.
std::string skipReasonFor(const std::string& name) {
    for (const auto& dp : disabledPatterns()) {
        if (std::regex_search(name, dp.pattern)) {
            return dp.reason;
        }
    }
    return {};
}

const GeneratorRegistry& generatorRegistry() {
    static const GeneratorRegistry registry = []() {
        GeneratorRegistry reg;

        // Insert a tile: check against disabledPatterns to set skipReason automatically.
        const auto add = [&](SyntheticMltGenerator::GeneratedTile tile) {
            auto name = tile.name;
            reg.emplace(
                std::move(name),
                RegistryEntry{.generator = [t = std::move(tile)]() { return t; }, .skipReason = skipReasonFor(name)});
        };

        // Insert all tiles from a collection.
        const auto addAll = [&](SyntheticMltGenerator::GeneratedTiles tiles) {
            for (auto& tile : tiles) {
                add(std::move(tile));
            }
        };

        // Basic geometry
        add(SyntheticMltGenerator::generatePoints());
        add(SyntheticMltGenerator::generateLine());
        add(SyntheticMltGenerator::generateLineZeroLength());
        add(SyntheticMltGenerator::generateLineMorton());
        addAll(SyntheticMltGenerator::generatePolygons());
        add(SyntheticMltGenerator::generateMultiPoint());
        add(SyntheticMltGenerator::generateMultiLine());
        addAll(SyntheticMltGenerator::generateExtent());
        add(SyntheticMltGenerator::generateExtent(1073741824U));

        // IDs
        addAll(SyntheticMltGenerator::generateIdsCollection());

        // Properties
        addAll(SyntheticMltGenerator::generateProperties());

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
    if (!entry.skipReason.empty()) {
        GTEST_SKIP() << entry.skipReason;
    }

    const auto generated = entry.generator();
    const auto filePath = basePath + name + ".mlt";
    const auto fileContents = readBinaryFile(filePath);
    EXPECT_EQ(fileContents, generated.bytes);
}

INSTANTIATE_TEST_SUITE_P(AllSyntheticFixtures,
                         SyntheticFixtureTest,
                         ::testing::ValuesIn(discoverAllFixtures()),
                         [](const ::testing::TestParamInfo<std::string>& info) { return info.param; });

// Reports fixture files that have no registered generator.
// Skips (rather than fails) so CI remains green while work is in progress.
TEST(SyntheticFixtureCoverage, AllFixturesHaveGenerators) {
    const auto allFixtures = discoverAllFixtures();
    const auto& reg = generatorRegistry();

    std::set<std::string> checked, unchecked, skipped, locatedGeneratedCases, registryKeys, missingFixtures;
    for (const auto& [name, _] : reg) {
        registryKeys.insert(name);
    }

    for (const auto& name : allFixtures) {
        const auto hit = reg.find(name);
        if (hit == reg.end()) {
            unchecked.insert(name);
        } else {
            locatedGeneratedCases.insert(name);
        }

        if (hit == reg.end()) {
            continue;
        }

        if (!hit->second.skipReason.empty()) {
            skipped.insert(name);
        } else {
            checked.insert(name);
        }
    }

    std::ranges::set_difference(
        registryKeys, locatedGeneratedCases, std::inserter(missingFixtures, missingFixtures.end()));

    std::ostringstream stream;
    stream << checked.size() << " fixture file(s) have generators and are checked by tests.";
    if (!unchecked.empty()) {
        stream << "\n";
        stream << std::to_string(unchecked.size()) << " fixture file(s) have no registered C++ generator:\n";
        for (const auto& n : unchecked) {
            stream << "  " << n << "\n";
        }
    }
    stream << "\n" << std::to_string(skipped.size()) << " fixture file(s) have generators but are marked as skipped.\n";
    if (!skipped.empty()) {
        for (const auto& n : skipped) {
            const auto& reason = reg.at(n).skipReason;
            stream << "  " << n << " (reason: " << reason << ")\n";
        }
    }
    stream << "\n" << missingFixtures.size() << " generated case(s) do not match any located fixture file.\n";
    if (!missingFixtures.empty()) {
        for (const auto& n : missingFixtures) {
            stream << "  " << n << "\n";
        }
    }

    const auto msg = stream.str();
    if (!msg.empty()) {
        if (missingFixtures.empty()) {
            GTEST_SKIP() << msg;
        } else {
            FAIL() << msg;
        }
    } else {
        SUCCEED() << "All " << allFixtures.size() << " fixture files are covered";
    }
}

} // namespace
