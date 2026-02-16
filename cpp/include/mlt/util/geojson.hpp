#pragma once
#include <cmath>
#include <regex>
#include <nlohmann/json.hpp>

namespace mlt::util {
using json = nlohmann::json;

struct GeoJsonGeometry {
private:
    static json coordinateRingToJson(const CoordVec& ring) {
        json result = json::array();
        for (const auto& coord : ring) {
            result.push_back(json::array({coord.x, coord.y}));
        }
        return result;
    }

public:
    std::string type;
    // GeoJSON coordinates can be Point [x,y], Line [[x,y]...], or Polygon [[[x,y]...]]
    nlohmann::json coordinates;

    static GeoJsonGeometry from(const geometry::Geometry& geom) {
        switch (geom.type) {
            case metadata::tileset::GeometryType::POINT: {
                const auto& point = static_cast<const geometry::Point&>(geom);
                const auto coord = point.getCoordinate();
                return {.type = "Point", .coordinates = json::array({coord.x, coord.y})};
            }
            case metadata::tileset::GeometryType::LINESTRING: {
                const auto& line = static_cast<const geometry::LineString&>(geom);
                return {.type = "LineString", .coordinates = coordinateRingToJson(line.getCoordinates())};
            }
            case metadata::tileset::GeometryType::POLYGON: {
                const auto& poly = static_cast<const geometry::Polygon&>(geom);
                json rings = json::array();
                for (const auto& ring : poly.getRings()) {
                    rings.push_back(coordinateRingToJson(ring));
                }
                return {.type = "Polygon", .coordinates = rings};
            }
            case metadata::tileset::GeometryType::MULTIPOINT: {
                const auto& multipoint = static_cast<const geometry::MultiPoint&>(geom);
                json coords = json::array();
                for (const auto& coord : multipoint.getCoordinates()) {
                    coords.push_back(json::array({coord.x, coord.y}));
                }
                return {.type = "MultiPoint", .coordinates = coords};
            }
            case metadata::tileset::GeometryType::MULTILINESTRING: {
                const auto& multiline = static_cast<const geometry::MultiLineString&>(geom);
                json lines = json::array();
                for (const auto& line : multiline.getLineStrings()) {
                    lines.push_back(coordinateRingToJson(line));
                }
                return {.type = "MultiLineString", .coordinates = lines};
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
                return {.type = "MultiPolygon", .coordinates = polygons};
            }
            default:
                throw std::runtime_error("Unsupported geometry type");
        }
    }
};
NLOHMANN_DEFINE_TYPE_NON_INTRUSIVE(GeoJsonGeometry, type, coordinates)

struct GeoJsonFeature {
private:
    struct PropertyToJsonVisitor {
        json operator()(std::nullptr_t) const { return nullptr; }
        json operator()(bool v) const { return v; }
        json operator()(int32_t v) const { return v; }
        json operator()(uint32_t v) const { return v; }
        json operator()(int64_t v) const { return v; }
        json operator()(uint64_t v) const { return v; }
        json operator()(float v) const {
            if (std::isnan(v) || std::isinf(v)) return nullptr;
            return v;
        }
        json operator()(double v) const {
            if (std::isnan(v) || std::isinf(v)) return nullptr;
            return v;
        }
        json operator()(std::string_view v) const { return std::string(v); }
        json operator()(const std::string& v) const { return v; }

        template <typename T>
        json operator()(const T& opt) const {
            if constexpr (requires { opt.has_value(); }) {
                return opt.has_value() ? propertyToJson(*opt) : nullptr;
            }
            return nullptr;
        }
    };

    static json propertyToJson(const Property& prop) { return std::visit(PropertyToJsonVisitor{}, prop); }

public:
    std::string type = "Feature";
    nlohmann::json id;
    GeoJsonGeometry geometry;
    nlohmann::json properties;

    GeoJsonFeature(nlohmann::json id, GeoJsonGeometry geometry, nlohmann::json properties)
        : id(std::move(id)),
          geometry(std::move(geometry)),
          properties(std::move(properties)) {}

    static GeoJsonFeature from(const Layer& layer, const Feature& feature) {
        json properties = json::object();
        properties["_layer"] = layer.getName();
        properties["_extent"] = layer.getExtent();
        for (const auto& key : layer.getProperties() | std::views::keys) {
            if (const auto prop = feature.getProperty(key, layer)) {
                properties[key] = propertyToJson(*prop);
            }
        }

        const auto id = feature.getID();
        const auto geom = GeoJsonGeometry::from(feature.getGeometry());
        return GeoJsonFeature{id, std::move(geom), std::move(properties)};
    }
};
NLOHMANN_DEFINE_TYPE_NON_INTRUSIVE(GeoJsonFeature, type, id, geometry, properties)

struct GeoJsonFeatureCollection {
    std::string type = "FeatureCollection";
    json::array_t features{};

    explicit GeoJsonFeatureCollection(const mlt::MapLibreTile& tile) {
        for (const auto& layer : tile.getLayers()) {
            for (const auto& feature : layer.getFeatures()) {
                json featObj{};
                const auto feat = GeoJsonFeature::from(layer, feature);
                to_json(featObj, feat);
                features.push_back(featObj);
            }
        }
    }
    GeoJsonFeatureCollection(const GeoJsonFeatureCollection& other)
        : features(other.features) {}
    GeoJsonFeatureCollection(GeoJsonFeatureCollection&& other) noexcept
        : features(std::move(other.features)) {}
    GeoJsonFeatureCollection& operator=(GeoJsonFeatureCollection other) {
        std::swap(this->features, other.features);
        return *this;
    }
};
NLOHMANN_DEFINE_TYPE_NON_INTRUSIVE(GeoJsonFeatureCollection, type, features)

class JsonComparator {
    std::vector<std::string> path_;
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

    std::string currentPath() const {
        std::string result;
        for (size_t i = 0; i < path_.size(); ++i) {
            if (!path_[i].starts_with('[')) result += '.';
            result += path_[i];
        }
        return result;
    }

    struct PathGuard {
        JsonComparator& comp;
        explicit PathGuard(JsonComparator& c, std::string part)
            : comp(c) {
            comp.path_.push_back(std::move(part));
        }
        ~PathGuard() { comp.path_.pop_back(); }
    };
    void assertNumbersApproxEqual(const json& expected, const json& actual) const {
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
            ss << "Numeric mismatch at " << currentPath() << ": expected " << expectedVal << ", got " << actualVal;
            throw std::runtime_error(ss.str());
        }
    }
    void assertEqualArray(const json& expected, const json& actual) {
        if (actual.size() != expected.size()) {
            std::ostringstream ss;
            ss << "Array size mismatch at " << currentPath() << ": expected " << expected.size() << " elements, got "
               << actual.size();
            throw std::runtime_error(ss.str());
        }

        for (size_t i = 0; i < actual.size(); ++i) {
            std::ostringstream ss;
            ss << "[" << i << "]";
            PathGuard guard(*this, ss.str());
            assertApproxEqual(actual[i], expected[i]);
        }
    }
    void assertEqualObject(const json& expected, const json& actual) {
        // Check for keys in expected that are missing in actual
        for (auto it = expected.begin(); it != expected.end(); ++it) {
            if (!actual.contains(it.key())) {
                std::ostringstream ss;
                ss << "Missing key at " << currentPath() << ": " << it.key();
                throw std::runtime_error(ss.str());
            }
        }

        // Check for extra keys in actual
        for (auto it = actual.begin(); it != actual.end(); ++it) {
            if (!expected.contains(it.key())) {
                std::ostringstream ss;
                ss << "Extra key at " << currentPath() << ": " << it.key();
                throw std::runtime_error(ss.str());
            }
        }

        // Compare values
        for (auto it = expected.begin(); it != expected.end(); ++it) {
            PathGuard guard(*this, it.key());
            assertApproxEqual(actual[it.key()], it.value());
        }
    }

public:
    void assertApproxEqual(const json& expected, const json& actual) {
        // Handle numeric comparisons - treat all number types as equivalent
        if (actual.is_number() && expected.is_number()) {
            assertNumbersApproxEqual(expected, actual);
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
            ss << "Type mismatch at " << currentPath() << ": expected " << expected.type_name() << ", got "
               << actual.type_name();
            throw std::runtime_error(ss.str());
        }

        if (actual.is_array()) {
            assertEqualArray(expected, actual);
            return;
        }

        if (actual.is_object()) {
            assertEqualObject(expected, actual);
            return;
        }

        // For primitives, use direct comparison
        if (actual != expected) {
            std::ostringstream ss;
            ss << "Value mismatch at " << currentPath() << ": expected " << expected << ", got " << actual;
            throw std::runtime_error(ss.str());
        }
    }
};
} // namespace mlt::util
