// Synthetic tests for MLT decoder
//
// Tests decode MLT files from test/synthetic/0x01/ and compare output against expected GeoJSON.
// Tests are discovered automatically from filesystem - no hardcoded test names.
// Known broken tests are listed in SKIPPED_TESTS map below.

#include <gtest/gtest.h>

#include <mlt/decoder.hpp>
#include <mlt/tile.hpp>

#include <cmath>
#include <filesystem>
#include <fstream>
#include <stdexcept>
#include <map>
#include <string>
#include <vector>
#include <set>
#include <algorithm>
#include "mlt/util/file_io.hpp"

#if MLT_WITH_JSON
#include "mlt/util/geojson.hpp"

#include <format>
#include <nlohmann/json.hpp>

// Tests that are known to be unimplemented or broken
const std::string numFeatError =
    "Decoder::Impl::makeFeatures incorrectly couples number of IDs to number of features. "
    "0 ids -> 0 features, despite there being a geometry and maybe properties"
    "number of features is surfaced by layer.getFeatures and is the only way for this test to know what the tile "
    "contains. ";
const std::map<std::string, std::string> SKIPPED_TESTS = {
    {"prop_str_empty", "Process finished with exit code 134 (interrupted by signal 6:SIGABRT)"},
    {"polygon_hole_fpf", "C++ exception with description '1 bytes trailing layer layer1' thrown in the test body"},
    {"polygon_morton_tes", "C++ exception with description '1 bytes trailing layer layer1' thrown in the test body"},
    {"polygon_fpf", "C++ exception with description '1 bytes trailing layer layer1' thrown in the test body"},
    {"polygon_multi_fpf", "C++ exception with description '1 bytes trailing layer layer1' thrown in the test body"},
    {"extent_1073741824", numFeatError},
    {"extent_131072", numFeatError},
    {"extent_4096", numFeatError},
    {"extent_512", numFeatError},
    {"extent_buf_1073741824", numFeatError},
    {"extent_buf_131072", numFeatError},
    {"extent_buf_4096", numFeatError},
    {"extent_buf_512", numFeatError},
    {"line", numFeatError},
    {"mixed_all", numFeatError},
    {"mixed_line_poly", numFeatError},
    {"mixed_pt_line", numFeatError},
    {"mixed_pt_mline", numFeatError},
    {"mixed_pt_poly", numFeatError},
    {"multipoint", numFeatError},
    {"multiline", numFeatError},
    {"point", numFeatError},
    {"polygon", numFeatError},
    {"polygon_hole", numFeatError},
    {"polygon_multi", numFeatError},
    {"polygon_tes", numFeatError},
    {"prop_bool", numFeatError},
    {"prop_bool_false", numFeatError},
    {"prop_f32", numFeatError},
    {"prop_f32_max", numFeatError},
    {"prop_f32_min", numFeatError},
    {"prop_f32_nan", numFeatError},
    {"prop_f32_neg_inf", numFeatError},
    {"prop_f32_pos_inf", numFeatError},
    {"prop_f32_zero", numFeatError},
    {"prop_f64", numFeatError},
    {"prop_f64_max", numFeatError},
    {"prop_f64_min", numFeatError},
    {"prop_f64_nan", numFeatError},
    {"prop_f64_neg_inf", numFeatError},
    {"prop_f64_pos_inf", numFeatError},
    {"prop_f64_zero", numFeatError},
    {"prop_f64_neg_zero", numFeatError},
    {"prop_i32", numFeatError},
    {"prop_i32_max", numFeatError},
    {"prop_i32_min", numFeatError},
    {"prop_i32_neg", numFeatError},
    {"prop_i64", numFeatError},
    {"prop_i64_max", numFeatError},
    {"prop_i64_min", numFeatError},
    {"prop_i64_neg", numFeatError},
    {"prop_str_ascii", numFeatError},
    {"prop_str_escape", numFeatError},
    {"prop_str_unicode", numFeatError},
    {"props_i32", numFeatError},
    {"props_i32_rle", numFeatError},
    {"props_i32_delta", numFeatError},
    {"props_i32_delta_rle", numFeatError},
    {"props_str", numFeatError},
    {"props_str_fsst", numFeatError},
    {"props_mixed", numFeatError},
    {"props_no_shared_dict", numFeatError},
    {"props_shared_dict", numFeatError},
    {"props_shared_dict_fsst", numFeatError},
};

class SyntheticTest : public ::testing::TestWithParam<std::pair<std::string, std::filesystem::path>> {
protected:
    void SetUp() override {
        const auto& [testName, _] = GetParam();
        if (const auto it = SKIPPED_TESTS.find(testName); it != SKIPPED_TESTS.end()) {
            GTEST_SKIP() << "Test '" << testName << "' skipped: " << it->second;
        }
    }
};

std::vector<std::pair<std::string, std::filesystem::path>> findSyntheticTests(const std::filesystem::path& dir) {
    std::set<std::string> testNames;

    if (!std::filesystem::exists(dir)) {
        throw std::runtime_error(
            std::format("Synthetic test directory {} does not exist", absolute(dir).generic_string()));
    }

    // Find all .mlt files and extract test names
    for (const auto& entry : std::filesystem::directory_iterator(dir)) {
        if (entry.is_regular_file() && entry.path().extension() == ".mlt") {
            testNames.insert(entry.path().stem().string());
        }
    }

    // Create pairs of test name and path
    std::vector<std::pair<std::string, std::filesystem::path>> tests;
    for (const auto& name : testNames) {
        const auto mltPath = dir / (name + ".mlt");
        const auto jsonPath = dir / (name + ".json");

        // Both files must exist
        if (std::filesystem::exists(mltPath) && std::filesystem::exists(jsonPath)) {
            tests.emplace_back(name, dir);
        }
    }

    std::ranges::sort(tests);
    return tests;
}

/// Preprocess JSON text to handle NaN and Infinity values
static std::string preprocessJson5ToJsonText(const std::string& text) {
    // \b is a word boundary anchor.
    // It matches the position between a word character and a non-word character.
    // This prevents matching "Infinity" inside "MyInfinityVariable".
    static const std::regex spec_values(R"(\b(Infinity|-Infinity|NaN)\b)");

    // We replace matches with "null" so that nlohmann::json can parse it.
    // We then handle the null-to-float conversion in assertJsonApproxEqual.
    return std::regex_replace(text, spec_values, "null");
}

// Instantiate tests for all synthetic test files found
INSTANTIATE_TEST_SUITE_P(Synthetic0x01,
                         SyntheticTest,
                         ::testing::ValuesIn(findSyntheticTests("../test/synthetic/0x01")),
                         [](const testing::TestParamInfo<SyntheticTest::ParamType>& info) {
                             // Use test name as the test case name
                             return info.param.first;
                         });

TEST_P(SyntheticTest, Decode) {
    const auto& [testName, dir] = GetParam();

    const auto mltPath = dir / (testName + ".mlt");
    const auto jsonPath = dir / (testName + ".json");

    const auto expectedJson5Text = mlt::util::loadTextFile(jsonPath);
    const mlt::util::GeoJsonFeatureCollection expected = nlohmann::json::parse(expectedJson5Text);

    const auto mltData = mlt::util::loadFile(mltPath);
    const auto tile = mlt::Decoder().decode({mltData.data(), mltData.size()});
    const auto actual = mlt::util::GeoJsonFeatureCollection(tile);

    try {
        mlt::util::JsonComparator comparator;
        comparator.assertApproxEqual(expected, actual);
    } catch (const std::runtime_error& e) {
        FAIL() << "\n"
               << "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
               << "SYNTHETIC TEST FAILED: " << testName << "\n"
               << "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
               << "Error:    " << e.what() << "\n"
               << "\n"
               << "Expected:\n"
               << expected.dump(2) << "\n"
               << "\n"
               << "Actual:\n"
               << actual.dump(2) << "\n"
               << "Files:\n"
               << "  MLT:      " << mltPath << "\n"
               << "  Expected: " << jsonPath << "\n"
               << "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n";
    }
}
#else  // MLT_WITH_JSON
TEST(SyntheticTest, JsonNotEnabled) {
    GTEST_SKIP() << "Synthetic tests require MLT_WITH_JSON to be enabled";
}
#endif // MLT_WITH_JSON
