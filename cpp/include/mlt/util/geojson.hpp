#pragma once
#include <cmath>
#include <nlohmann/json.hpp>

namespace mlt::util {
using json = nlohmann::json;
struct GeoJson : public util::noncopyable {
private:
    /// Check if two floating point values are approximately equal
    static bool floatsApproxEqual(const double actual, const double expected) {
        // Tolerance for floating point comparisons
        constexpr double RELATIVE_FLOAT_TOLERANCE = 0.0001 / 100.0;
        constexpr double ABSOLUTE_FLOAT_TOLERANCE = std::numeric_limits<double>::epsilon();

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

    static std::string joinWithDot(const std::vector<std::string>& parts) {
        std::string result;
        for (size_t i = 0; i < parts.size(); ++i) {
            if (!parts[i].starts_with('[')) result += '.';
            result += parts[i];
        }
        return result;
    }

    static json coordinateRingToJson(const CoordVec& ring) {
        json result = json::array();
        for (const auto& coord : ring) {
            result.push_back(json::array({coord.x, coord.y}));
        }
        return result;
    }

    static json geometryToGeoJson(const geometry::Geometry& geom) {
        json result;

        switch (geom.type) {
            case metadata::tileset::GeometryType::POINT: {
                const auto& point = static_cast<const geometry::Point&>(geom);
                const auto coord = point.getCoordinate();
                result = {{"type", "Point"}, {"coordinates", json::array({coord.x, coord.y})}};
                break;
            }
            case metadata::tileset::GeometryType::LINESTRING: {
                const auto& line = static_cast<const geometry::LineString&>(geom);
                result = {{"type", "LineString"}, {"coordinates", coordinateRingToJson(line.getCoordinates())}};
                break;
            }
            case metadata::tileset::GeometryType::POLYGON: {
                const auto& poly = static_cast<const geometry::Polygon&>(geom);
                json rings = json::array();
                for (const auto& ring : poly.getRings()) {
                    rings.push_back(coordinateRingToJson(ring));
                }
                result = {{"type", "Polygon"}, {"coordinates", rings}};
                break;
            }
            case metadata::tileset::GeometryType::MULTIPOINT: {
                const auto& multipoint = static_cast<const geometry::MultiPoint&>(geom);
                json coords = json::array();
                for (const auto& coord : multipoint.getCoordinates()) {
                    coords.push_back(json::array({coord.x, coord.y}));
                }
                result = {{"type", "MultiPoint"}, {"coordinates", coords}};
                break;
            }
            case metadata::tileset::GeometryType::MULTILINESTRING: {
                const auto& multiline = static_cast<const geometry::MultiLineString&>(geom);
                json lines = json::array();
                for (const auto& line : multiline.getLineStrings()) {
                    lines.push_back(coordinateRingToJson(line));
                }
                result = {{"type", "MultiLineString"}, {"coordinates", lines}};
                break;
            }
            case metadata::tileset::GeometryType::MULTIPOLYGON: {
                const auto& multipoly = static_cast<const geometry::MultiPolygon&>(geom);
                json polygons = json::array();
                for (const auto& poly : multipoly.getPolygons()) {
                    json rings = json::array();
                    for (const auto& ring : poly) {
                        rings.push_back(coordinateRingToJson(ring));
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
    template <class... Ts>
    struct overloaded : Ts... {
        using Ts::operator()...;
    };
    template <class... Ts>
    overloaded(Ts...) -> overloaded<Ts...>;

    static json propertyToJson(const Property& prop) {
        return std::visit(overloaded{[](std::nullptr_t) -> json { return nullptr; },
                                     [](bool v) -> json { return v; },
                                     [](int32_t v) -> json { return v; },
                                     [](uint32_t v) -> json { return v; },
                                     [](int64_t v) -> json { return v; },
                                     [](uint64_t v) -> json { return v; },
                                     [](float v) -> json {
                                         if (std::isnan(v) || std::isinf(v)) return nullptr; // Or handle as strings
                                         return v;
                                     },
                                     [](double v) -> json {
                                         if (std::isnan(v) || std::isinf(v)) return nullptr;
                                         return v;
                                     },
                                     [](std::string_view v) -> json { return std::string(v); },
                                     [](const std::string& v) -> json { return v; },
                                     [](auto&& opt) -> json {
                                         if constexpr (requires { opt.has_value(); }) {
                                             return opt.has_value() ? propertyToJson(*opt) : nullptr;
                                         } else {
                                             return nullptr;
                                         }
                                     }},
                          prop);
    }

    static json featureToGeoJson(const Layer& layer, const Feature& feature) {
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

    /// Recursively compare JSON values with float tolerance
    /// Throws std::runtime_error if comparison fails
    static void assertJsonApproxEqual(const json& actual, const json& expected, std::vector<std::string>& path) {
        // Handle numeric comparisons - treat all number types as equivalent
        if (actual.is_number() && expected.is_number()) {
            const double actualVal = actual.get<double>();
            const double expectedVal = expected.get<double>();

            // Handle NaN - both should be NaN
            if (std::isnan(actualVal) && std::isnan(expectedVal)) {
                return;
            }

            // Handle Infinity
            if (std::isinf(actualVal) && std::isinf(expectedVal)) {
                // Check same sign
                if ((actualVal > 0) == (expectedVal > 0)) {
                    return;
                }
            }

            if (!floatsApproxEqual(actualVal, expectedVal)) {
                std::ostringstream ss;
                ss << "Numeric mismatch at " << joinWithDot(path) << ": ";
                ss << "expected " << expectedVal << ", got " << actualVal;
                throw std::runtime_error(ss.str());
            }
            return;
        }

        // Handle null in expected (which represents NaN/Infinity from preprocessed JSON)
        // and number in actual (which is the decoded NaN/Infinity)
        if (expected.is_null() && actual.is_number()) {
            const double actualVal = actual.get<double>();
            // Accept NaN or Infinity as matching null from expected
            if (std::isnan(actualVal) || std::isinf(actualVal)) {
                return;
            }
        }

        // Handle null values (which may represent NaN or Infinity in expected JSON)
        // Accept null in actual if expected is also null
        if (actual.is_null() && expected.is_null()) {
            return;
        }

        // For non-numeric types, check type equality
        if (actual.type() != expected.type()) {
            std::ostringstream ss;
            ss << "Type mismatch at " << joinWithDot(path) << ": ";
            ss << "expected " << expected.type_name() << ", got " << actual.type_name();
            throw std::runtime_error(ss.str());
        }

        if (actual.is_array()) {
            if (actual.size() != expected.size()) {
                std::ostringstream ss;
                ss << "Array size mismatch at " << joinWithDot(path) << ": ";
                ss << "expected " << expected.size() << " elements, got " << actual.size();
                throw std::runtime_error(ss.str());
            }

            for (size_t i = 0; i < actual.size(); ++i) {
                std::string arrayKey = "[";
                arrayKey += std::to_string(i);
                arrayKey += "]";
                path.push_back(arrayKey);
                assertJsonApproxEqual(actual[i], expected[i], path);
                path.pop_back();
            }
            return;
        }

        if (actual.is_object()) {
            // Check for keys in expected that are missing in actual
            for (auto it = expected.begin(); it != expected.end(); ++it) {
                if (!actual.contains(it.key())) {
                    std::ostringstream ss;
                    ss << "Missing key at " << joinWithDot(path) << ": " << it.key();
                    throw std::runtime_error(ss.str());
                }
            }

            // Check for extra keys in actual
            for (auto it = actual.begin(); it != actual.end(); ++it) {
                if (!expected.contains(it.key())) {
                    std::ostringstream ss;
                    ss << "Extra key at " << joinWithDot(path) << ": " << it.key();
                    throw std::runtime_error(ss.str());
                }
            }

            // Compare values
            for (auto it = expected.begin(); it != expected.end(); ++it) {
                path.push_back(it.key());
                assertJsonApproxEqual(actual[it.key()], it.value(), path);
                path.pop_back();
            }
            return;
        }

        // For primitives, use direct comparison
        if (actual != expected) {
            std::ostringstream ss;
            ss << "Value mismatch at " << joinWithDot(path) << ": ";
            ss << "expected " << expected << ", got " << actual;
            throw std::runtime_error(ss.str());
        }
    }

    /// Preprocess JSON text to handle NaN and Infinity values
    static std::string preprocessJson5ToJsonText(const std::string& text) {
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

    static json tileToGeoJson(const mlt::MapLibreTile& tile) {
        json features = json::array();

        for (const auto& layer : tile.getLayers()) {
            for (const auto& feature : layer.getFeatures()) {
                features.push_back(featureToGeoJson(layer, feature));
            }
        }

        return {{"type", "FeatureCollection"}, {"features", features}};
    }

public:
    explicit GeoJson(const std::string& text)
        : value(json::parse(preprocessJson5ToJsonText(text))) {}
    explicit GeoJson(const mlt::MapLibreTile& tile)
        : value(tileToGeoJson(tile)) {}

    /// Asserts for equality except small floating point deviations
    void assertApproxEqual(const GeoJson& other) const {
        std::vector<std::string> path;
        assertJsonApproxEqual(this->value, other.value, path);
    }

    const json value;
};
} // namespace mlt::util
