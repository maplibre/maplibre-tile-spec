// Synthetic tests for MLT decoder
//
// Tests decode MLT files from test/synthetic/0x01/ and compare output against expected GeoJSON.
// Tests are discovered automatically from filesystem - no hardcoded test names.
// Known broken tests are listed in SKIPPED_TESTS map below.
//
// GeoJSON format matches Rust and TypeScript implementations:
// - Layer name/extent added as _layer/_extent properties
// - CRS section included (EPSG:0 for tile coordinates)
// - Floating point comparison uses relative/absolute tolerance

#include <gtest/gtest.h>

#include <mlt/decoder.hpp>
#include <mlt/feature.hpp>
#include <mlt/geometry.hpp>
#include <mlt/layer.hpp>
#include <mlt/tile.hpp>

#include <cmath>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <limits>
#include <map>
#include <sstream>
#include <string>
#include <vector>

#if MLT_WITH_JSON
#include <nlohmann/json.hpp>

namespace fs = std::filesystem;

// Tolerance for floating point comparisons
constexpr double RELATIVE_FLOAT_TOLERANCE = 0.0001 / 100.0;
constexpr double ABSOLUTE_FLOAT_TOLERANCE = std::numeric_limits<double>::epsilon();

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
    {"props_mixed", numFeatError},
    {"props_no_shared_dict", numFeatError},
    {"props_shared_dict_fsst", numFeatError},
};

class SyntheticTest : public ::testing::TestWithParam<std::pair<std::string, fs::path>> {
protected:
    void SetUp() override {
        const auto& [testName, _] = GetParam();
        if (const auto it = SKIPPED_TESTS.find(testName); it != SKIPPED_TESTS.end()) {
            GTEST_SKIP() << "Test '" << testName << "' skipped: " << it->second;
        }
    }
};

void testSyntheticPair(const std::string& testName, const fs::path& dir);

TEST_P(SyntheticTest, DecodeSynthetic) {
    const auto& [testName, dir] = GetParam();
    testSyntheticPair(testName, dir);
}

std::vector<std::pair<std::string, fs::path>> findSyntheticTests(const fs::path& dir);

// Instantiate tests for all synthetic test files found
INSTANTIATE_TEST_SUITE_P(Synthetic0x01,
                         SyntheticTest,
                         ::testing::ValuesIn(findSyntheticTests("../../../test/synthetic/0x01")),
                         [](const testing::TestParamInfo<SyntheticTest::ParamType>& info) {
                             // Use test name as the test case name
                             return info.param.first;
                         });

TEST(SyntheticTest, JsonNotEnabled) {
    GTEST_SKIP() << "Synthetic tests require MLT_WITH_JSON to be enabled";
}

namespace io {
/// Load binary file contents
std::vector<char> loadFile(const fs::path& path) {
    std::ifstream file(path, std::ios::binary | std::ios::ate);
    if (!file.is_open()) {
        throw std::runtime_error("Failed to open file: " + path.string());
    }

    const auto size = file.tellg();
    file.seekg(0);

    std::vector<char> buffer(size);
    if (!file.read(buffer.data(), size)) {
        throw std::runtime_error("Failed to read file: " + path.string());
    }

    return buffer;
}

/// Load text file contents
std::string loadTextFile(const fs::path& path) {
    std::ifstream file(path);
    if (!file) {
        throw std::runtime_error("Failed to open file: " + path.string());
    }

    std::stringstream buffer;
    buffer << file.rdbuf();
    return buffer.str();
}
} // namespace io

namespace json_encoding {

using json = nlohmann::json;
/// Preprocess JSON text to handle NaN and Infinity values
std::string preprocessJsonText(const std::string& text) {
    std::string result = text;

    // Replace JavaScript-style special values with valid JSON
    // We need to preserve them as special markers that we can handle later

    // Handle -Infinity first (before Infinity)
    size_t pos = 0;
    while ((pos = result.find("-Infinity", pos)) != std::string::npos) {
        const bool isWord = (pos > 0 && std::isalnum(result[pos - 1])) ||
                            (pos + 9 < result.length() && std::isalnum(result[pos + 9]));
        if (!isWord) {
            result.replace(pos, 9, "null");
            pos += 4;
        } else {
            pos += 9;
        }
    }

    // Handle Infinity
    pos = 0;
    while ((pos = result.find("Infinity", pos)) != std::string::npos) {
        bool isWord = (pos > 0 && std::isalnum(result[pos - 1])) ||
                      (pos + 8 < result.length() && std::isalnum(result[pos + 8]));
        if (!isWord) {
            result.replace(pos, 8, "null");
            pos += 4;
        } else {
            pos += 8;
        }
    }

    // Handle NaN
    pos = 0;
    while ((pos = result.find("NaN", pos)) != std::string::npos) {
        // Check it's not part of a word (e.g., not "NaNcy")
        const bool isWord = (pos > 0 && std::isalnum(result[pos - 1])) ||
                            (pos + 3 < result.length() && std::isalnum(result[pos + 3]));
        if (!isWord) {
            result.replace(pos, 3, "null");
            pos += 4;
        } else {
            pos += 3;
        }
    }

    return result;
}

/// Check if two floating point values are approximately equal
bool floatsApproxEqual(const double actual, const double expected) {
    // Handle special cases: NaN, infinity
    if (std::isnan(expected)) {
        return std::isnan(actual);
    }
    if (std::isinf(expected)) {
        return std::isinf(actual) && std::signbit(actual) == std::signbit(expected);
    }

    // Check for values very close to zero
    if (std::abs(expected) < ABSOLUTE_FLOAT_TOLERANCE) {
        return std::abs(actual) <= ABSOLUTE_FLOAT_TOLERANCE;
    }

    // Relative error check
    const double relativeError = std::abs(actual - expected) / std::abs(expected);
    return relativeError <= RELATIVE_FLOAT_TOLERANCE;
}

/// Recursively compare JSON values with float tolerance
bool jsonApproxEqual(const json& actual, const json& expected, std::vector<std::string>& path, std::string& errorMsg) {
    // Handle numeric comparisons - treat all number types as equivalent
    if (actual.is_number() && expected.is_number()) {
        const double actualVal = actual.get<double>();
        const double expectedVal = expected.get<double>();

        // Handle NaN - both should be NaN
        if (std::isnan(actualVal) && std::isnan(expectedVal)) {
            return true;
        }

        // Handle Infinity
        if (std::isinf(actualVal) && std::isinf(expectedVal)) {
            // Check same sign
            if ((actualVal > 0) == (expectedVal > 0)) {
                return true;
            }
        }

        if (!floatsApproxEqual(actualVal, expectedVal)) {
            std::ostringstream ss;
            ss << "Numeric mismatch at " << nlohmann::json(path).dump() << ": ";
            ss << "expected " << expectedVal << ", got " << actualVal;
            errorMsg = ss.str();
            return false;
        }
        return true;
    }

    // Handle null in expected (which represents NaN/Infinity from preprocessed JSON)
    // and number in actual (which is the decoded NaN/Infinity)
    if (expected.is_null() && actual.is_number()) {
        const double actualVal = actual.get<double>();
        // Accept NaN or Infinity as matching null from expected
        if (std::isnan(actualVal) || std::isinf(actualVal)) {
            return true;
        }
    }

    // Handle null values (which may represent NaN or Infinity in expected JSON)
    // Accept null in actual if expected is also null
    if (actual.is_null() && expected.is_null()) {
        return true;
    }

    // For non-numeric types, check type equality
    if (actual.type() != expected.type()) {
        std::ostringstream ss;
        ss << "Type mismatch at " << nlohmann::json(path).dump() << ": ";
        ss << "expected " << expected.type_name() << ", got " << actual.type_name();
        errorMsg = ss.str();
        return false;
    }

    if (actual.is_array()) {
        if (actual.size() != expected.size()) {
            std::ostringstream ss;
            ss << "Array size mismatch at " << nlohmann::json(path).dump() << ": ";
            ss << "expected " << expected.size() << " elements, got " << actual.size();
            errorMsg = ss.str();
            return false;
        }

        for (size_t i = 0; i < actual.size(); ++i) {
            path.push_back(std::to_string(i));
            if (!jsonApproxEqual(actual[i], expected[i], path, errorMsg)) {
                return false;
            }
            path.pop_back();
        }
        return true;
    }

    if (actual.is_object()) {
        // Check for keys in expected that are missing in actual
        for (auto it = expected.begin(); it != expected.end(); ++it) {
            if (!actual.contains(it.key())) {
                std::ostringstream ss;
                ss << "Missing key at " << nlohmann::json(path).dump() << ": " << it.key();
                errorMsg = ss.str();
                return false;
            }
        }

        // Check for extra keys in actual
        for (auto it = actual.begin(); it != actual.end(); ++it) {
            if (!expected.contains(it.key())) {
                std::ostringstream ss;
                ss << "Extra key at " << nlohmann::json(path).dump() << ": " << it.key();
                errorMsg = ss.str();
                return false;
            }
        }

        // Compare values
        for (auto it = expected.begin(); it != expected.end(); ++it) {
            path.push_back(it.key());
            if (!jsonApproxEqual(actual[it.key()], it.value(), path, errorMsg)) {
                return false;
            }
            path.pop_back();
        }
        return true;
    }

    // For primitives, use direct comparison
    if (actual != expected) {
        std::ostringstream ss;
        ss << "Value mismatch at " << nlohmann::json(path).dump() << ": ";
        ss << "expected " << expected << ", got " << actual;
        errorMsg = ss.str();
        return false;
    }

    return true;
}

/// Convert a coordinate to JSON array [x, y]
json coordToJson(const mlt::Coordinate& coord) {
    return json::array({coord.x, coord.y});
}

/// Convert a ring of coordinates to JSON
json ringToJson(const mlt::CoordVec& ring) {
    json result = json::array();
    for (const auto& coord : ring) {
        result.push_back(coordToJson(coord));
    }
    return result;
}

/// Convert geometry to GeoJSON with CRS
json geometryToGeoJson(const mlt::geometry::Geometry& geom) {
    json result;

    switch (geom.type) {
        case mlt::metadata::tileset::GeometryType::POINT: {
            const auto& point = static_cast<const mlt::geometry::Point&>(geom);
            result = {{"type", "Point"}, {"coordinates", coordToJson(point.getCoordinate())}};
            break;
        }
        case mlt::metadata::tileset::GeometryType::LINESTRING: {
            const auto& line = static_cast<const mlt::geometry::LineString&>(geom);
            result = {{"type", "LineString"}, {"coordinates", ringToJson(line.getCoordinates())}};
            break;
        }
        case mlt::metadata::tileset::GeometryType::POLYGON: {
            const auto& poly = static_cast<const mlt::geometry::Polygon&>(geom);
            json rings = json::array();
            for (const auto& ring : poly.getRings()) {
                rings.push_back(ringToJson(ring));
            }
            result = {{"type", "Polygon"}, {"coordinates", rings}};
            break;
        }
        case mlt::metadata::tileset::GeometryType::MULTIPOINT: {
            const auto& multipoint = static_cast<const mlt::geometry::MultiPoint&>(geom);
            json coords = json::array();
            for (const auto& coord : multipoint.getCoordinates()) {
                coords.push_back(coordToJson(coord));
            }
            result = {{"type", "MultiPoint"}, {"coordinates", coords}};
            break;
        }
        case mlt::metadata::tileset::GeometryType::MULTILINESTRING: {
            const auto& multiline = static_cast<const mlt::geometry::MultiLineString&>(geom);
            json lines = json::array();
            for (const auto& line : multiline.getLineStrings()) {
                lines.push_back(ringToJson(line));
            }
            result = {{"type", "MultiLineString"}, {"coordinates", lines}};
            break;
        }
        case mlt::metadata::tileset::GeometryType::MULTIPOLYGON: {
            const auto& multipoly = static_cast<const mlt::geometry::MultiPolygon&>(geom);
            json polygons = json::array();
            for (const auto& poly : multipoly.getPolygons()) {
                json rings = json::array();
                for (const auto& ring : poly) {
                    rings.push_back(ringToJson(ring));
                }
                polygons.push_back(rings);
            }
            result = {{"type", "MultiPolygon"}, {"coordinates", polygons}};
            break;
        }
        default:
            throw std::runtime_error("Unsupported geometry type");
    }

    return result;
}

/// Convert property value to JSON, handling all Property variant types
json propertyToJson(const mlt::Property& prop) {
    return std::visit(
        [](const auto& value) -> json {
            using T = std::decay_t<decltype(value)>;

            // Handle nullptr_t
            if constexpr (std::is_same_v<T, std::nullptr_t>) {
                return nullptr;
            }
            // Handle optional types
            else if constexpr (std::is_same_v<T, std::optional<bool>> || std::is_same_v<T, std::optional<int32_t>> ||
                               std::is_same_v<T, std::optional<int64_t>> ||
                               std::is_same_v<T, std::optional<uint32_t>> ||
                               std::is_same_v<T, std::optional<uint64_t>> || std::is_same_v<T, std::optional<float>> ||
                               std::is_same_v<T, std::optional<double>>) {
                if (value.has_value()) {
                    // Convert large integers to double to match JSON number type
                    if constexpr (std::is_same_v<T, std::optional<int64_t>> ||
                                  std::is_same_v<T, std::optional<uint64_t>>) {
                        return static_cast<double>(*value);
                    } else {
                        return *value;
                    }
                }
                return nullptr;
            }
            // Handle int64_t and uint64_t - convert to double for JSON
            else if constexpr (std::is_same_v<T, int64_t> || std::is_same_v<T, uint64_t>) {
                return static_cast<double>(value);
            }
            // Handle string_view - convert to string
            else if constexpr (std::is_same_v<T, std::string_view>) {
                return std::string(value);
            }
            // All other types (bool, int32_t, uint32_t, float, double)
            else {
                return value;
            }
        },
        prop);
}

/// Convert feature to GeoJSON
json featureToGeoJson(const mlt::Layer& layer, const mlt::Feature& feature) {
    json properties = json::object();
    properties["_layer"] = layer.getName();
    properties["_extent"] = layer.getExtent();

    // Add feature properties
    for (const auto& key : layer.getProperties() | std::views::keys) {
        if (const auto prop = feature.getProperty(key, layer)) {
            properties[key] = propertyToJson(*prop);
        }
    }

    return {{"type", "Feature"},
            {"id", feature.getID()},
            {"geometry", geometryToGeoJson(feature.getGeometry())},
            {"properties", properties}};
}

/// Convert entire tile to FeatureCollection GeoJSON
json tileToGeoJson(const mlt::MapLibreTile& tile) {
    json features = json::array();

    for (const auto& layer : tile.getLayers()) {
        for (const auto& feature : layer.getFeatures()) {
            features.push_back(featureToGeoJson(layer, feature));
        }
    }

    return {{"type", "FeatureCollection"}, {"features", features}};
}
} // namespace json_encoding

/// Find all test pairs (mlt + json files) in the synthetic test directory
std::vector<std::pair<std::string, fs::path>> findSyntheticTests(const fs::path& dir) {
    std::set<std::string> testNames;

    if (!fs::exists(dir)) {
        return {};
    }

    // Find all .mlt files and extract test names
    for (const auto& entry : fs::directory_iterator(dir)) {
        if (entry.is_regular_file() && entry.path().extension() == ".mlt") {
            testNames.insert(entry.path().stem().string());
        }
    }

    // Create pairs of test name and path
    std::vector<std::pair<std::string, fs::path>> tests;
    for (const auto& name : testNames) {
        const auto mltPath = dir / (name + ".mlt");
        const auto jsonPath = dir / (name + ".json");

        // Both files must exist
        if (fs::exists(mltPath) && fs::exists(jsonPath)) {
            tests.emplace_back(name, dir);
        }
    }

    std::ranges::sort(tests);
    return tests;
}

/// Perform a single synthetic test
void testSyntheticPair(const std::string& testName, const fs::path& dir) {
    const auto mltPath = dir / (testName + ".mlt");
    const auto jsonPath = dir / (testName + ".json");

    // Load and decode MLT file
    const auto mltData = io::loadFile(mltPath);
    auto tile = mlt::Decoder().decode({mltData.data(), mltData.size()});

    // Convert to GeoJSON
    const auto actualJson = json_encoding::tileToGeoJson(tile);

    // Load expected GeoJSON
    const auto expectedText = io::loadTextFile(jsonPath);
    const auto processedText = json_encoding::preprocessJsonText(expectedText);
    const auto expectedJson = json_encoding::json::parse(processedText);

    // Compare with tolerance for floats
    std::vector<std::string> path;
    std::string errorMsg;

    if (!json_encoding::jsonApproxEqual(actualJson, expectedJson, path, errorMsg)) {
        FAIL() << "\n"
               << "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
               << "SYNTHETIC TEST FAILED: " << testName << "\n"
               << "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
               << "Error:    " << errorMsg << "\n"
               << "\n"
               << "Expected:\n"
               << expectedJson.dump(2) << "\n"
               << "\n"
               << "Actual:\n"
               << actualJson.dump(2) << "\n"
               << "Files:\n"
               << "  MLT:      " << mltPath << "\n"
               << "  Expected: " << jsonPath << "\n"
               << "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n";
    }
}
#endif // MLT_WITH_JSON
