#include <mlt/encoder.hpp>

#include <mlt/encode/boolean.hpp>
#include <mlt/encode/geometry.hpp>
#include <mlt/encode/int.hpp>
#include <mlt/encode/property.hpp>
#include <mlt/encode/string.hpp>
#include <mlt/metadata/tileset.hpp>
#include <mlt/metadata/type_map.hpp>
#include <mlt/util/encoding/varint.hpp>
#include <mlt/util/stl.hpp>

#include <algorithm>
#include <stdexcept>
#include <unordered_set>

namespace mlt {

using namespace encoder;
using namespace metadata::tileset;
using namespace metadata::stream;

struct Encoder::Impl {
    IntegerEncoder intEncoder;

    std::vector<std::uint8_t> encodeLayer(const Layer& layer, const EncoderConfig& config);

    FeatureTable buildMetadata(const Layer& layer, const EncoderConfig& config);

    void collectGeometry(const std::vector<Feature>& features,
                         std::vector<GeometryType>& geometryTypes,
                         std::vector<std::uint32_t>& numGeometries,
                         std::vector<std::uint32_t>& numParts,
                         std::vector<std::uint32_t>& numRings,
                         std::vector<GeometryEncoder::Vertex>& vertexBuffer);
};

FeatureTable Encoder::Impl::buildMetadata(const Layer& layer, const EncoderConfig& config) {
    FeatureTable table;
    table.name = layer.name;
    table.extent = layer.extent;

    if (config.includeIds) {
        bool hasLongId = std::any_of(layer.features.begin(), layer.features.end(),
                                     [](const auto& f) { return f.id > std::numeric_limits<std::uint32_t>::max(); });

        Column idColumn;
        idColumn.nullable = false;
        idColumn.columnScope = ColumnScope::FEATURE;
        idColumn.type = ScalarColumn{.type = LogicalScalarType::ID, .hasLongID = hasLongId};
        table.columns.push_back(std::move(idColumn));
    }

    Column geomColumn;
    geomColumn.nullable = false;
    geomColumn.columnScope = ColumnScope::FEATURE;
    geomColumn.type = ComplexColumn{.type = ComplexType::GEOMETRY};
    table.columns.push_back(std::move(geomColumn));

    struct ColumnInfo {
        ScalarType type;
        bool nullable;
    };
    std::map<std::string, ColumnInfo> propertyColumns;

    for (const auto& feature : layer.features) {
        for (const auto& [key, value] : feature.properties) {
            auto scalarType = std::visit(util::overloaded{
                [](bool) { return ScalarType::BOOLEAN; },
                [](std::int32_t) { return ScalarType::INT_32; },
                [](std::int64_t) { return ScalarType::INT_64; },
                [](std::uint32_t) { return ScalarType::UINT_32; },
                [](std::uint64_t) { return ScalarType::UINT_64; },
                [](float) { return ScalarType::FLOAT; },
                [](double) { return ScalarType::DOUBLE; },
                [](const std::string&) { return ScalarType::STRING; },
            }, value);

            auto [it, inserted] = propertyColumns.try_emplace(key, ColumnInfo{scalarType, false});
            if (!inserted) {
                auto& existing = it->second;
                if (existing.type != scalarType) {
                    if ((existing.type == ScalarType::INT_32 && scalarType == ScalarType::INT_64) ||
                        (existing.type == ScalarType::FLOAT && scalarType == ScalarType::DOUBLE)) {
                        existing.type = scalarType;
                    } else if ((existing.type == ScalarType::INT_64 && scalarType == ScalarType::INT_32) ||
                               (existing.type == ScalarType::DOUBLE && scalarType == ScalarType::FLOAT)) {
                    } else {
                        existing.type = ScalarType::STRING;
                    }
                }
            }
        }
    }

    for (auto& [key, info] : propertyColumns) {
        // String columns always require a present stream per the decoder contract
        if (info.type == ScalarType::STRING) {
            info.nullable = true;
            continue;
        }
        for (const auto& feature : layer.features) {
            if (!feature.properties.contains(key)) {
                info.nullable = true;
                break;
            }
        }
    }

    for (const auto& [name, info] : propertyColumns) {
        Column col;
        col.name = name;
        col.nullable = info.nullable;
        col.columnScope = ColumnScope::FEATURE;
        col.type = ScalarColumn{.type = info.type};
        table.columns.push_back(std::move(col));
    }

    return table;
}

void Encoder::Impl::collectGeometry(const std::vector<Feature>& features,
                                     std::vector<metadata::tileset::GeometryType>& geometryTypes,
                                     std::vector<std::uint32_t>& numGeometries,
                                     std::vector<std::uint32_t>& numParts,
                                     std::vector<std::uint32_t>& numRings,
                                     std::vector<GeometryEncoder::Vertex>& vertexBuffer) {
    using GT = metadata::tileset::GeometryType;

    const bool containsPolygon = std::any_of(features.begin(), features.end(), [](const auto& f) {
        return f.geometry.type == GT::POLYGON || f.geometry.type == GT::MULTIPOLYGON;
    });

    for (const auto& feature : features) {
        const auto& geom = feature.geometry;
        geometryTypes.push_back(geom.type);

        switch (geom.type) {
            case GT::POINT:
                for (const auto& v : geom.coordinates) {
                    vertexBuffer.push_back({v.x, v.y});
                }
                break;

            case GT::LINESTRING:
                if (containsPolygon) {
                    numRings.push_back(static_cast<std::uint32_t>(geom.coordinates.size()));
                } else {
                    numParts.push_back(static_cast<std::uint32_t>(geom.coordinates.size()));
                }
                for (const auto& v : geom.coordinates) {
                    vertexBuffer.push_back({v.x, v.y});
                }
                break;

            case GT::POLYGON:
                numParts.push_back(static_cast<std::uint32_t>(geom.ringSizes.size()));
                for (auto ringSize : geom.ringSizes) {
                    numRings.push_back(ringSize);
                }
                for (const auto& v : geom.coordinates) {
                    vertexBuffer.push_back({v.x, v.y});
                }
                break;

            case GT::MULTIPOINT:
                numGeometries.push_back(static_cast<std::uint32_t>(geom.coordinates.size()));
                for (const auto& v : geom.coordinates) {
                    vertexBuffer.push_back({v.x, v.y});
                }
                break;

            case GT::MULTILINESTRING:
                numGeometries.push_back(static_cast<std::uint32_t>(geom.parts.size()));
                for (const auto& part : geom.parts) {
                    if (containsPolygon) {
                        numRings.push_back(static_cast<std::uint32_t>(part.size()));
                    } else {
                        numParts.push_back(static_cast<std::uint32_t>(part.size()));
                    }
                    for (const auto& v : part) {
                        vertexBuffer.push_back({v.x, v.y});
                    }
                }
                break;

            case GT::MULTIPOLYGON:
                numGeometries.push_back(static_cast<std::uint32_t>(geom.parts.size()));
                for (std::size_t p = 0; p < geom.parts.size(); ++p) {
                    const auto& rings = geom.partRingSizes[p];
                    numParts.push_back(static_cast<std::uint32_t>(rings.size()));
                    for (auto ringSize : rings) {
                        numRings.push_back(ringSize);
                    }
                    for (const auto& v : geom.parts[p]) {
                        vertexBuffer.push_back({v.x, v.y});
                    }
                }
                break;
        }
    }
}

std::vector<std::uint8_t> Encoder::Impl::encodeLayer(const Layer& layer, const EncoderConfig& config) {
    if (layer.features.empty()) {
        return {};
    }

    auto physicalTechnique = config.useFastPfor ? PhysicalLevelTechnique::FAST_PFOR
                                                : PhysicalLevelTechnique::VARINT;

    auto featureTable = buildMetadata(layer, config);
    auto metadataBytes = encodeFeatureTable(featureTable);

    std::vector<std::uint8_t> bodyBytes;

    if (config.includeIds) {
        bool hasLongId = std::any_of(layer.features.begin(), layer.features.end(),
                                     [](const auto& f) { return f.id > std::numeric_limits<std::uint32_t>::max(); });

        if (hasLongId) {
            std::vector<std::uint64_t> ids;
            ids.reserve(layer.features.size());
            for (const auto& f : layer.features) {
                ids.push_back(f.id);
            }
            auto encoded = PropertyEncoder::encodeUint64Column(ids, intEncoder);
            bodyBytes.insert(bodyBytes.end(), encoded.begin(), encoded.end());
        } else {
            std::vector<std::uint32_t> ids;
            ids.reserve(layer.features.size());
            for (const auto& f : layer.features) {
                ids.push_back(static_cast<std::uint32_t>(f.id));
            }
            auto encoded = PropertyEncoder::encodeUint32Column(ids, physicalTechnique, intEncoder);
            bodyBytes.insert(bodyBytes.end(), encoded.begin(), encoded.end());
        }
    }

    std::vector<metadata::tileset::GeometryType> geometryTypes;
    std::vector<std::uint32_t> numGeometries, numParts, numRings;
    std::vector<GeometryEncoder::Vertex> vertexBuffer;
    collectGeometry(layer.features, geometryTypes, numGeometries, numParts, numRings, vertexBuffer);

    auto encodedGeom = GeometryEncoder::encodeGeometryColumn(
        geometryTypes, numGeometries, numParts, numRings, vertexBuffer,
        physicalTechnique, intEncoder);

    util::encoding::encodeVarint(encodedGeom.numStreams, bodyBytes);
    bodyBytes.insert(bodyBytes.end(), encodedGeom.encodedValues.begin(), encodedGeom.encodedValues.end());

    for (const auto& column : featureTable.columns) {
        if (column.isID() || column.isGeometry()) {
            continue;
        }

        const auto& scalarCol = column.getScalarType();
        const auto scalarType = scalarCol.getPhysicalType();
        const auto& colName = column.name;

        switch (scalarType) {
            case ScalarType::BOOLEAN: {
                std::vector<std::optional<bool>> values;
                values.reserve(layer.features.size());
                for (const auto& f : layer.features) {
                    auto it = f.properties.find(colName);
                    if (it != f.properties.end()) {
                        values.push_back(std::get<bool>(it->second));
                    } else {
                        values.push_back(std::nullopt);
                    }
                }
                auto encoded = PropertyEncoder::encodeBooleanColumn(values);
                bodyBytes.insert(bodyBytes.end(), encoded.begin(), encoded.end());
                break;
            }
            case ScalarType::INT_32: {
                std::vector<std::optional<std::int32_t>> values;
                values.reserve(layer.features.size());
                for (const auto& f : layer.features) {
                    auto it = f.properties.find(colName);
                    if (it != f.properties.end()) {
                        values.push_back(std::visit(util::overloaded{
                            [](std::int32_t v) -> std::int32_t { return v; },
                            [](std::int64_t v) -> std::int32_t { return static_cast<std::int32_t>(v); },
                            [](auto) -> std::int32_t { return 0; },
                        }, it->second));
                    } else {
                        values.push_back(std::nullopt);
                    }
                }
                auto encoded = PropertyEncoder::encodeInt32Column(values, physicalTechnique, true, intEncoder);
                bodyBytes.insert(bodyBytes.end(), encoded.begin(), encoded.end());
                break;
            }
            case ScalarType::UINT_32: {
                std::vector<std::optional<std::int32_t>> values;
                values.reserve(layer.features.size());
                for (const auto& f : layer.features) {
                    auto it = f.properties.find(colName);
                    if (it != f.properties.end()) {
                        values.push_back(static_cast<std::int32_t>(std::visit(util::overloaded{
                            [](std::uint32_t v) -> std::uint32_t { return v; },
                            [](std::int32_t v) -> std::uint32_t { return static_cast<std::uint32_t>(v); },
                            [](auto) -> std::uint32_t { return 0; },
                        }, it->second)));
                    } else {
                        values.push_back(std::nullopt);
                    }
                }
                auto encoded = PropertyEncoder::encodeInt32Column(values, physicalTechnique, false, intEncoder);
                bodyBytes.insert(bodyBytes.end(), encoded.begin(), encoded.end());
                break;
            }
            case ScalarType::INT_64: {
                std::vector<std::optional<std::int64_t>> values;
                values.reserve(layer.features.size());
                for (const auto& f : layer.features) {
                    auto it = f.properties.find(colName);
                    if (it != f.properties.end()) {
                        values.push_back(std::visit(util::overloaded{
                            [](std::int64_t v) -> std::int64_t { return v; },
                            [](std::int32_t v) -> std::int64_t { return v; },
                            [](auto) -> std::int64_t { return 0; },
                        }, it->second));
                    } else {
                        values.push_back(std::nullopt);
                    }
                }
                auto encoded = PropertyEncoder::encodeInt64Column(values, true, intEncoder);
                bodyBytes.insert(bodyBytes.end(), encoded.begin(), encoded.end());
                break;
            }
            case ScalarType::UINT_64: {
                std::vector<std::optional<std::int64_t>> values;
                values.reserve(layer.features.size());
                for (const auto& f : layer.features) {
                    auto it = f.properties.find(colName);
                    if (it != f.properties.end()) {
                        values.push_back(static_cast<std::int64_t>(std::visit(util::overloaded{
                            [](std::uint64_t v) -> std::uint64_t { return v; },
                            [](std::int64_t v) -> std::uint64_t { return static_cast<std::uint64_t>(v); },
                            [](auto) -> std::uint64_t { return 0; },
                        }, it->second)));
                    } else {
                        values.push_back(std::nullopt);
                    }
                }
                auto encoded = PropertyEncoder::encodeInt64Column(values, false, intEncoder);
                bodyBytes.insert(bodyBytes.end(), encoded.begin(), encoded.end());
                break;
            }
            case ScalarType::FLOAT:
            case ScalarType::DOUBLE: {
                std::vector<std::optional<float>> values;
                values.reserve(layer.features.size());
                for (const auto& f : layer.features) {
                    auto it = f.properties.find(colName);
                    if (it != f.properties.end()) {
                        values.push_back(std::visit(util::overloaded{
                            [](float v) -> float { return v; },
                            [](double v) -> float { return static_cast<float>(v); },
                            [](auto) -> float { return 0.0f; },
                        }, it->second));
                    } else {
                        values.push_back(std::nullopt);
                    }
                }
                auto encoded = PropertyEncoder::encodeFloatColumn(values);
                bodyBytes.insert(bodyBytes.end(), encoded.begin(), encoded.end());
                break;
            }
            case ScalarType::STRING: {
                std::vector<std::string> ownedStrings;
                ownedStrings.reserve(layer.features.size());
                std::vector<std::optional<std::string_view>> values;
                values.reserve(layer.features.size());
                for (const auto& f : layer.features) {
                    auto it = f.properties.find(colName);
                    if (it != f.properties.end()) {
                        auto& owned = ownedStrings.emplace_back(std::visit(util::overloaded{
                            [](const std::string& v) -> std::string { return v; },
                            [](auto v) -> std::string { return std::to_string(v); },
                        }, it->second));
                        values.push_back(std::string_view{owned});
                    } else {
                        values.push_back(std::nullopt);
                    }
                }
                auto encoded = PropertyEncoder::encodeStringColumn(values, physicalTechnique, intEncoder);
                bodyBytes.insert(bodyBytes.end(), encoded.begin(), encoded.end());
                break;
            }
            default:
                throw std::runtime_error("Unsupported property type for column: " + colName);
        }
    }

    std::vector<std::uint8_t> layerBytes;
    util::encoding::encodeVarint(static_cast<std::uint32_t>(1), layerBytes);
    layerBytes.insert(layerBytes.end(), metadataBytes.begin(), metadataBytes.end());
    layerBytes.insert(layerBytes.end(), bodyBytes.begin(), bodyBytes.end());
    return layerBytes;
}

Encoder::Encoder()
    : impl(std::make_unique<Impl>()) {}

Encoder::~Encoder() noexcept = default;

std::vector<std::uint8_t> Encoder::encode(const std::vector<Layer>& layers, const EncoderConfig& config) {
    std::vector<std::uint8_t> result;
    for (const auto& layer : layers) {
        auto layerBytes = impl->encodeLayer(layer, config);
        if (layerBytes.empty()) {
            continue;
        }
        util::encoding::encodeVarint(static_cast<std::uint32_t>(layerBytes.size()), result);
        result.insert(result.end(), layerBytes.begin(), layerBytes.end());
    }
    return result;
}

} // namespace mlt
