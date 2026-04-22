#include <mlt/encoder.hpp>

#include <mlt/encode/boolean.hpp>
#include <mlt/encode/geometry.hpp>
#include <mlt/encode/int.hpp>
#include <mlt/encode/property.hpp>
#include <mlt/encode/string.hpp>
#include <mlt/metadata/tileset.hpp>
#include <mlt/metadata/type_map.hpp>
#include <mlt/util/encoding/varint.hpp>
#include <mlt/util/hilbert_curve.hpp>
#include <mlt/util/string.hpp>
#include <mlt/util/stl.hpp>

#include <mapbox/earcut.hpp>

#include <algorithm>
#include <array>
#include <numeric>
#include <stdexcept>
#include <unordered_map>

namespace mlt {

using namespace encoder;
using namespace metadata::tileset;
using namespace metadata::stream;

using GeometryType = metadata::tileset::GeometryType;

namespace {
// `std::visit` requires exhaustive overloads, but the type is determined
// by column metadata, so the catch-all arms are dead by construction.
// This is used in cases that should be impossible to reach.
// GCOVR_EXCL_START
void throwInvalidType() {
    throw std::runtime_error("Invalid type");
};
// GCOVR_EXCL_STOP
} // namespace

struct Encoder::Impl {
    struct IdStats {
        bool hasLongId = false;
        bool hasMissingId = false;
    };

    IntegerEncoder intEncoder;

    static bool structValueHasChild(const Encoder::StructValue& sv,
                                    const std::string& rootName,
                                    const std::string& childName);
    static const std::string* resolveStructSourceKey(const std::vector<Feature>& features,
                                                     const metadata::tileset::Column& column);

    std::vector<std::uint8_t> encodeLayer(const Layer& layer, const EncoderConfig& config);

    static IdStats collectIdStats(const std::vector<Feature>& features);
    FeatureTable buildMetadata(const Layer& layer, const EncoderConfig& config, std::optional<IdStats> idStats);

    static void collectGeometry(const std::vector<Feature>& features,
                                std::vector<GeometryType>& geometryTypes,
                                std::vector<std::uint32_t>& numGeometries,
                                std::vector<std::uint32_t>& numParts,
                                std::vector<std::uint32_t>& numRings,
                                std::vector<GeometryEncoder::Vertex>& vertexBuffer);

    static std::vector<Encoder::Feature> sortFeatures(const std::vector<Encoder::Feature>& features);

    static bool allPolygons(const std::vector<Encoder::Feature>& features);
    static void tessellateFeatures(const std::vector<Feature>& features,
                                   std::vector<std::uint32_t>& numTriangles,
                                   std::vector<std::uint32_t>& indexBuffer);
};

bool Encoder::Impl::structValueHasChild(const Encoder::StructValue& sv,
                                        const std::string& rootName,
                                        const std::string& childName) {
    if (sv.contains(childName)) {
        return true;
    }
    return sv.contains(rootName + childName);
}

const std::string* Encoder::Impl::resolveStructSourceKey(const std::vector<Feature>& features,
                                                         const metadata::tileset::Column& column) {
    if (!column.isStruct()) {
        return nullptr;
    }

    const auto& complex = column.getComplexType();
    const auto& rootName = column.name;

    const std::string* bestKey = nullptr;
    std::size_t bestScore = 0;

    for (const auto& f : features) {
        for (const auto& [propertyKey, propertyValue] : f.properties) {
            const auto* sv = std::get_if<Encoder::StructValue>(&propertyValue);
            if (!sv) {
                continue;
            }

            std::size_t score = 0;
            for (const auto& child : complex.children) {
                if (structValueHasChild(*sv, rootName, child.name)) {
                    ++score;
                }
            }

            if (score > bestScore) {
                bestScore = score;
                bestKey = &propertyKey;
                if (score == complex.children.size()) {
                    return bestKey;
                }
            }
        }
    }

    return bestKey;
}

Encoder::Impl::IdStats Encoder::Impl::collectIdStats(const std::vector<Feature>& features) {
    IdStats idStats;
    for (const auto& feature : features) {
        if (!feature.id.has_value()) {
            idStats.hasMissingId = true;
            continue;
        }
        // Use 64-bit when any ID exceeds INT32_MAX: delta encoding accumulates in
        // int32_t, so uint32 values with bit 31 set would sign-extend on widening.
        if (*feature.id > std::numeric_limits<std::int32_t>::max()) {
            idStats.hasLongId = true;
        }
        if (idStats.hasLongId && idStats.hasMissingId) {
            break;
        }
    }
    return idStats;
}

FeatureTable Encoder::Impl::buildMetadata(const Layer& layer,
                                          const EncoderConfig& config,
                                          std::optional<IdStats> idStats) {
    FeatureTable table;
    table.name = layer.name;
    table.extent = layer.extent;

    if (config.includeIds) {
        const auto stats = idStats.value_or(collectIdStats(layer.features));

        table.columns.push_back(Column{
            .nullable = stats.hasMissingId,
            .columnScope = ColumnScope::FEATURE,
            .type = ScalarColumn{.type = LogicalScalarType::ID, .hasLongID = stats.hasLongId},
        });
    }

    Column geomColumn;
    geomColumn.nullable = false;
    geomColumn.columnScope = ColumnScope::FEATURE;
    geomColumn.type = ComplexColumn{.type = ComplexType::GEOMETRY};
    table.columns.push_back(std::move(geomColumn));

    struct ColumnInfo {
        ScalarType type;
        std::size_t presentCount;
    };
    std::map<std::string, ColumnInfo> scalarColumns;
    std::map<std::string, std::set<std::string>> structColumnsBySourceKey;

    for (const auto& feature : layer.features) {
        for (const auto& [key, value] : feature.properties) {
            if (std::holds_alternative<Encoder::StructValue>(value)) {
                const auto& sv = std::get<Encoder::StructValue>(value);
                auto& children = structColumnsBySourceKey[key];
                for (const auto& [childName, _] : sv) {
                    children.insert(childName);
                }
                continue;
            }

            auto scalarType = std::visit(
                util::overloaded{
                    [](bool) { return ScalarType::BOOLEAN; },
                    [](std::int32_t) { return ScalarType::INT_32; },
                    [](std::int64_t) { return ScalarType::INT_64; },
                    [](std::uint32_t) { return ScalarType::UINT_32; },
                    [](std::uint64_t) { return ScalarType::UINT_64; },
                    [](float) { return ScalarType::FLOAT; },
                    [](double) { return ScalarType::DOUBLE; },
                    [](const std::string&) { return ScalarType::STRING; },
                    [](const Encoder::StructValue&) -> ScalarType { throwInvalidType(); }, // GCOVR_EXCL_LINE
                },
                value);

            auto [it, inserted] = scalarColumns.try_emplace(key, ColumnInfo{.type = scalarType, .presentCount = 0});
            ++it->second.presentCount;
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

    for (const auto& [name, info] : scalarColumns) {
        Column col;
        col.name = name;
        col.nullable = config.forceNullableColumns || info.presentCount != layer.features.size();
        col.columnScope = ColumnScope::FEATURE;
        col.type = ScalarColumn{.type = info.type};
        table.columns.push_back(std::move(col));
    }

    for (const auto& [sourceKey, childNames] : structColumnsBySourceKey) {
        const auto derivedRoot = util::longestCommonPrefix(childNames);
        const bool hasDerivedRoot = childNames.size() > 1 && !derivedRoot.empty();

        Column col;
        col.name = hasDerivedRoot ? derivedRoot : sourceKey;
        col.nullable = false;
        col.columnScope = ColumnScope::FEATURE;

        ComplexColumn complex;
        complex.type = ComplexType::STRUCT;
        for (const auto& childName : childNames) {
            Column child;
            child.name = hasDerivedRoot ? childName.substr(derivedRoot.size()) : childName;
            child.nullable = true;
            child.columnScope = ColumnScope::FEATURE;
            child.type = ScalarColumn{.type = ScalarType::STRING};
            complex.children.push_back(std::move(child));
        }
        col.type = std::move(complex);
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
    const bool containsPolygon = std::ranges::any_of(features, [](const auto& f) {
        return f.geometry.type == GeometryType::POLYGON || f.geometry.type == GeometryType::MULTIPOLYGON;
    });

    const auto pushVertices = [&](const std::vector<Vertex>& coords) {
        for (const auto& v : coords) vertexBuffer.push_back({v.x, v.y});
    };

    for (const auto& feature : features) {
        const auto& geom = feature.geometry;
        geometryTypes.push_back(geom.type);

        switch (geom.type) {
            case GeometryType::POINT:
                pushVertices(geom.coordinates);
                break;

            case GeometryType::LINESTRING:
                (containsPolygon ? numRings : numParts).push_back(static_cast<std::uint32_t>(geom.coordinates.size()));
                pushVertices(geom.coordinates);
                break;

            case GeometryType::POLYGON:
                numParts.push_back(static_cast<std::uint32_t>(geom.ringSizes.size()));
                for (auto ringSize : geom.ringSizes) {
                    numRings.push_back(ringSize);
                }
                pushVertices(geom.coordinates);
                break;

            case GeometryType::MULTIPOINT:
                numGeometries.push_back(static_cast<std::uint32_t>(geom.coordinates.size()));
                pushVertices(geom.coordinates);
                break;

            case GeometryType::MULTILINESTRING:
                numGeometries.push_back(static_cast<std::uint32_t>(geom.parts.size()));
                for (const auto& part : geom.parts) {
                    (containsPolygon ? numRings : numParts).push_back(static_cast<std::uint32_t>(part.size()));
                    pushVertices(part);
                }
                break;

            case GeometryType::MULTIPOLYGON:
                numGeometries.push_back(static_cast<std::uint32_t>(geom.parts.size()));
                for (std::size_t p = 0; p < geom.parts.size(); ++p) {
                    const auto& rings = geom.partRingSizes[p];
                    numParts.push_back(static_cast<std::uint32_t>(rings.size()));
                    for (auto ringSize : rings) {
                        numRings.push_back(ringSize);
                    }
                    pushVertices(geom.parts[p]);
                }
                break;
        }
    }
}

std::vector<Encoder::Feature> Encoder::Impl::sortFeatures(const std::vector<Encoder::Feature>& features) {
    if (features.empty()) {
        return {};
    }

    const auto firstType = features.front().geometry.type;
    if (firstType != GeometryType::POINT && firstType != GeometryType::LINESTRING) {
        return {};
    }
    const bool allSame = std::ranges::all_of(features,
                                             [firstType](const auto& f) { return f.geometry.type == firstType; });
    if (!allSame) {
        return {};
    }

    auto minVal = std::numeric_limits<std::int32_t>::max();
    auto maxVal = std::numeric_limits<std::int32_t>::min();
    for (const auto& f : features) {
        for (const auto& v : f.geometry.coordinates) {
            minVal = std::min({minVal, v.x, v.y});
            maxVal = std::max({maxVal, v.x, v.y});
        }
    }

    util::HilbertCurve curve(minVal, maxVal);

    std::vector<std::uint32_t> hilbertIds(features.size());
    for (std::size_t i = 0; i < features.size(); ++i) {
        const auto& g = features[i].geometry;
        if (!g.coordinates.empty()) {
            const auto& v = features[i].geometry.coordinates[0];
            hilbertIds[i] = curve.encode({static_cast<float>(v.x), static_cast<float>(v.y)});
        }
    }

    std::vector<std::size_t> order(features.size());
    // NOLINTNEXTLINE(boost-use-ranges)
    std::iota(order.begin(), order.end(), 0);
    std::ranges::sort(order, [&](std::size_t a, std::size_t b) { return hilbertIds[a] < hilbertIds[b]; });

    std::vector<Feature> sorted(features.size());
    std::ranges::transform(order, sorted.begin(), [&](auto idx) { return features[idx]; });
    return sorted;
}

bool Encoder::Impl::allPolygons(const std::vector<Encoder::Feature>& features) {
    return !features.empty() && std::ranges::all_of(features, [](const auto& f) {
        return f.geometry.type == GeometryType::POLYGON || f.geometry.type == GeometryType::MULTIPOLYGON;
    });
}

void Encoder::Impl::tessellateFeatures(const std::vector<Feature>& features,
                                       std::vector<std::uint32_t>& numTriangles,
                                       std::vector<std::uint32_t>& indexBuffer) {
    using EarcutPoint = std::array<double, 2>;

    auto tessellateOnePolygon = [](const std::vector<Vertex>& coords,
                                   const std::vector<std::uint32_t>& ringSizes,
                                   std::uint32_t indexOffset) -> std::pair<std::uint32_t, std::vector<std::uint32_t>> {
        std::vector<std::vector<EarcutPoint>> polygon;
        std::size_t vertIdx = 0;
        for (auto ringSize : ringSizes) {
            std::vector<EarcutPoint> ring;
            ring.reserve(ringSize);
            for (std::uint32_t i = 0; i < ringSize; ++i) {
                ring.push_back({static_cast<double>(coords[vertIdx].x), static_cast<double>(coords[vertIdx].y)});
                ++vertIdx;
            }
            polygon.push_back(std::move(ring));
        }

        auto indices = mapbox::earcut<std::uint32_t>(polygon);
        if (indexOffset > 0) {
            std::ranges::for_each(indices, [=](auto& idx) { idx += indexOffset; });
        }
        return {static_cast<std::uint32_t>(indices.size() / 3), std::move(indices)};
    };

    for (const auto& feature : features) {
        const auto& geom = feature.geometry;
        if (geom.type == GeometryType::POLYGON) {
            auto [nTri, indices] = tessellateOnePolygon(geom.coordinates, geom.ringSizes, 0);
            numTriangles.push_back(nTri);
            indexBuffer.insert(indexBuffer.end(), indices.begin(), indices.end());
        } else if (geom.type == GeometryType::MULTIPOLYGON) {
            std::uint32_t totalTri = 0;
            std::uint32_t vertexOffset = 0;
            std::vector<std::uint32_t> allIndices;
            for (std::size_t p = 0; p < geom.parts.size(); ++p) {
                auto [nTri, indices] = tessellateOnePolygon(geom.parts[p], geom.partRingSizes[p], vertexOffset);
                totalTri += nTri;
                allIndices.insert(allIndices.end(), indices.begin(), indices.end());
                vertexOffset += static_cast<std::uint32_t>(geom.parts[p].size());
            }
            numTriangles.push_back(totalTri);
            indexBuffer.insert(indexBuffer.end(), allIndices.begin(), allIndices.end());
        }
    }
}

std::vector<std::uint8_t> Encoder::Impl::encodeLayer(const Layer& layer, const EncoderConfig& config) {
    if (layer.features.empty()) {
        return {};
    }

    intEncoder.setDefaultEncodingOption(config.integerEncodingOption);

    const auto sortedStorage = config.sortFeatures ? sortFeatures(layer.features) : std::vector<Feature>{};
    const auto& features = sortedStorage.empty() ? layer.features : sortedStorage;

    const auto physicalTechnique = config.useFastPfor ? PhysicalLevelTechnique::FAST_PFOR
                                                      : PhysicalLevelTechnique::VARINT;

    const auto idStats = config.includeIds ? std::optional<IdStats>{collectIdStats(features)} : std::nullopt;
    const auto featureTable = buildMetadata(layer, config, idStats);
    const auto metadataBytes = encodeFeatureTable(featureTable);

    // Collect encoded chunks; concatenate once at the end to avoid repeated reallocation.
    std::vector<std::vector<std::uint8_t>> bodyChunks;
    const auto appendEncodedColumn = [&](std::vector<std::uint8_t> encoded) {
        bodyChunks.push_back(std::move(encoded));
    };
    const auto appendEncodedColumnChunks = [&](std::vector<std::vector<std::uint8_t>> encodedChunks) {
        for (auto& chunk : encodedChunks) {
            bodyChunks.push_back(std::move(chunk));
        }
    };
    const auto appendEncodedStreamSet = [&](std::uint32_t numStreams, std::vector<std::uint8_t> encodedValues) {
        std::vector<std::uint8_t> header;
        util::encoding::encodeVarint(numStreams, header);
        bodyChunks.push_back(std::move(header));
        bodyChunks.push_back(std::move(encodedValues));
    };
    const auto appendEncodedStreamSetChunks = [&](std::uint32_t numStreams,
                                                  std::vector<std::vector<std::uint8_t>> encodedChunks) {
        std::vector<std::uint8_t> header;
        util::encoding::encodeVarint(numStreams, header);
        bodyChunks.push_back(std::move(header));
        for (auto& chunk : encodedChunks) {
            bodyChunks.push_back(std::move(chunk));
        }
    };
    const auto extractIds = [&]<typename TId>(auto&& convertId) {
        std::vector<std::optional<TId>> ids;
        ids.reserve(features.size());
        for (const auto& feature : features) {
            ids.push_back(convertId(feature));
        }
        return ids;
    };

    if (config.includeIds) {
        const auto hasLongId = idStats->hasLongId;
        const auto hasMissingId = idStats->hasMissingId;

        if (hasLongId) {
            auto ids = extractIds.template operator()<std::uint64_t>(
                [](const auto& feature) -> std::optional<std::uint64_t> { return feature.id; });
            appendEncodedColumnChunks(PropertyEncoder::encodeUint64ColumnChunked(ids, intEncoder, hasMissingId).chunks);
        } else {
            auto ids = extractIds.template operator()<std::int32_t>(
                [](const auto& feature) -> std::optional<std::int32_t> {
                    return feature.id.has_value() ? std::optional<std::int32_t>{static_cast<std::int32_t>(*feature.id)}
                                                  : std::nullopt;
                });
            appendEncodedColumnChunks(
                PropertyEncoder::encodeInt32ColumnChunked(ids, physicalTechnique, false, intEncoder, hasMissingId)
                    .chunks);
        }
    }

    std::vector<metadata::tileset::GeometryType> geometryTypes;
    std::vector<std::uint32_t> numGeometries, numParts, numRings;
    std::vector<GeometryEncoder::Vertex> vertexBuffer;
    collectGeometry(features, geometryTypes, numGeometries, numParts, numRings, vertexBuffer);

    const bool usePretessellation = config.preTessellate && allPolygons(features);
    const auto geometryIntegerEncodingOption = config.geometryEncodingOption.value_or(config.integerEncodingOption);
    const auto geometryTopologyIntegerEncodingOption = config.geometryTopologyEncodingOption.value_or(
        geometryIntegerEncodingOption);

    GeometryEncoder::EncodedGeometryColumn encodedGeom = [&] {
        if (usePretessellation) {
            std::vector<std::uint32_t> numTriangles;
            std::vector<std::uint32_t> indexBuffer;
            tessellateFeatures(features, numTriangles, indexBuffer);
            return GeometryEncoder::encodePretessellatedGeometryColumn(geometryTypes,
                                                                       numGeometries,
                                                                       numParts,
                                                                       numRings,
                                                                       vertexBuffer,
                                                                       numTriangles,
                                                                       indexBuffer,
                                                                       physicalTechnique,
                                                                       intEncoder,
                                                                       geometryIntegerEncodingOption,
                                                                       geometryTopologyIntegerEncodingOption,
                                                                       config.includeOutlines);
        }
        return GeometryEncoder::encodeGeometryColumn(geometryTypes,
                                                     numGeometries,
                                                     numParts,
                                                     numRings,
                                                     vertexBuffer,
                                                     physicalTechnique,
                                                     intEncoder,
                                                     geometryIntegerEncodingOption,
                                                     geometryTopologyIntegerEncodingOption,
                                                     config.useMortonEncoding,
                                                     config.forceMortonGeometryLayout);
    }();

    appendEncodedStreamSet(encodedGeom.numStreams, std::move(encodedGeom.encodedValues));

    // Create a cache of properties for each scalar column to avoid repeated linear searches during encoding
    using PropertyValuePtr = const Encoder::PropertyValue*;
    std::unordered_map<std::string, std::vector<PropertyValuePtr>> scalarPropertyCache;
    scalarPropertyCache.reserve(featureTable.columns.size());
    for (const auto& column : featureTable.columns) {
        if (!column.isID() && !column.isGeometry() && !column.isStruct()) {
            scalarPropertyCache.emplace(column.name, std::vector<PropertyValuePtr>(features.size(), nullptr));
        }
    }

    for (std::size_t fi = 0; fi < features.size(); ++fi) {
        for (const auto& [key, value] : features[fi].properties) {
            auto it = scalarPropertyCache.find(key);
            if (it != scalarPropertyCache.end()) {
                it->second[fi] = &value;
            }
        }
    }

    for (const auto& column : featureTable.columns) {
        if (column.isID() || column.isGeometry()) {
            continue;
        }

        if (column.isStruct()) {
            const auto& complex = column.getComplexType();
            const auto& rootName = column.name;
            const auto sourceKey = resolveStructSourceKey(features, column);
            const auto numChildren = complex.children.size();
            const auto resolveStructChildValue = [&](const Encoder::StructValue& sv,
                                                     const std::string& childName) -> const std::string* {
                if (auto childIt = sv.find(childName); childIt != sv.end()) {
                    return &childIt->second;
                }
                if (auto prefixedChildIt = sv.find(rootName + childName); prefixedChildIt != sv.end()) {
                    return &prefixedChildIt->second;
                }
                return nullptr;
            };

            std::vector<const Encoder::StructValue*> structValues(features.size(), nullptr);
            for (std::size_t fi = 0; fi < features.size(); ++fi) {
                auto it = sourceKey ? features[fi].properties.find(*sourceKey) : features[fi].properties.find(rootName);
                if (it != features[fi].properties.end() && std::holds_alternative<Encoder::StructValue>(it->second)) {
                    structValues[fi] = &std::get<Encoder::StructValue>(it->second);
                }
            }

            // Build one optional<string_view> column per child, pointing directly into live feature data.
            // This eliminates the previous ownedStrings (string copies), viewStorage, and childPresent intermediates.
            std::vector<std::vector<std::optional<std::string_view>>> optCols(numChildren);
            for (std::size_t c = 0; c < numChildren; ++c) {
                optCols[c].reserve(features.size());
            }
            for (std::size_t fi = 0; fi < features.size(); ++fi) {
                const auto* sv = structValues[fi];
                for (std::size_t c = 0; c < numChildren; ++c) {
                    const auto* childValue = sv ? resolveStructChildValue(*sv, complex.children[c].name) : nullptr;
                    optCols[c].push_back(childValue ? std::optional<std::string_view>{*childValue} : std::nullopt);
                }
            }

            auto result = StringEncoder::encodeSharedDictionaryChunked(
                optCols, physicalTechnique, intEncoder, config.useFsst);

            appendEncodedStreamSetChunks(result.numStreams, std::move(result.chunks));
            continue;
        }

        const auto& scalarCol = column.getScalarType();
        const auto scalarType = scalarCol.getPhysicalType();
        const auto& colName = column.name;
        const auto cachedIt = scalarPropertyCache.find(colName);
        if (cachedIt == scalarPropertyCache.end()) {
            throwInvalidType(); // GCOVR_EXCL_LINE
        }
        const auto& cachedPropertyValues = cachedIt->second;

        const auto extractSeparatedColumn =
            [&]<typename T>(
                auto&& visitor, std::vector<bool>& presentValues, std::vector<T>& dataValues, bool& hasNull) {
                presentValues.clear();
                presentValues.reserve(features.size());
                dataValues.clear();
                dataValues.reserve(features.size());
                hasNull = false;
                for (const auto* propertyValue : cachedPropertyValues) {
                    if (propertyValue != nullptr) {
                        presentValues.push_back(true);
                        dataValues.push_back(std::visit(visitor, *propertyValue));
                    } else {
                        presentValues.push_back(false);
                        hasNull = true;
                    }
                }
            };

        const auto encodeSeparatedColumn =
            [&]<typename T>(auto&& visitor, auto&& encodeColumn) -> std::vector<std::vector<std::uint8_t>> {
            std::vector<bool> presentValues;
            std::vector<T> dataValues;
            bool hasNull = false;
            extractSeparatedColumn.template operator()<T>(visitor, presentValues, dataValues, hasNull);
            return encodeColumn(std::span<const T>{dataValues}, presentValues, hasNull);
        };

        std::vector<std::vector<std::uint8_t>> encodedChunks;
        switch (scalarType) {
            case ScalarType::BOOLEAN:
                encodedChunks = encodeSeparatedColumn.template operator()<std::uint8_t>(
                    util::overloaded{
                        [](bool v) -> std::uint8_t { return static_cast<std::uint8_t>(v); },
                        // the type is already determined by column metadata, so the catch-all arms are dead by
                        // construction
                        [](auto) -> std::uint8_t { throwInvalidType(); }, // GCOVR_EXCL_LINE
                    },
                    [&](auto dataValues, const auto& presentValues, bool hasNull) {
                        return PropertyEncoder::encodeSeparatedDataColumnChunked(
                                   dataValues,
                                   presentValues,
                                   hasNull,
                                   column.nullable,
                                   [](std::span<const std::uint8_t> input) {
                                       return BooleanEncoder::encodeBooleanStream(
                                           input, metadata::stream::PhysicalStreamType::DATA);
                                   })
                            .chunks;
                    });
                break;
            case ScalarType::INT_32:
                encodedChunks = encodeSeparatedColumn.template operator()<std::int32_t>(
                    util::overloaded{
                        [](std::int32_t v) -> std::int32_t { return v; },
                        [](std::int64_t v) -> std::int32_t { return static_cast<std::int32_t>(v); },
                        [](auto) -> std::int32_t { throwInvalidType(); }, // GCOVR_EXCL_LINE
                    },
                    [&](auto dataValues, const auto& presentValues, bool hasNull) {
                        return PropertyEncoder::encodeSeparatedDataColumnChunked(
                                   dataValues,
                                   presentValues,
                                   hasNull,
                                   column.nullable,
                                   [&](std::span<const std::int32_t> input) {
                                       return intEncoder.encodeIntStream(input,
                                                                         physicalTechnique,
                                                                         true,
                                                                         metadata::stream::PhysicalStreamType::DATA,
                                                                         std::nullopt);
                                   })
                            .chunks;
                    });
                break;
            case ScalarType::UINT_32:
                encodedChunks = encodeSeparatedColumn.template operator()<std::uint32_t>(
                    util::overloaded{
                        [](std::uint32_t v) -> std::uint32_t { return v; },
                        [](std::int32_t v) -> std::uint32_t { return static_cast<std::uint32_t>(v); },
                        [](auto) -> std::uint32_t { throwInvalidType(); }, // GCOVR_EXCL_LINE
                    },
                    [&](auto dataValues, const auto& presentValues, bool hasNull) {
                        return PropertyEncoder::encodeSeparatedDataColumnChunked(
                                   dataValues,
                                   presentValues,
                                   hasNull,
                                   column.nullable,
                                   [&](std::span<const std::uint32_t> input) {
                                       return intEncoder.encodeUint32Stream(input,
                                                                            physicalTechnique,
                                                                            metadata::stream::PhysicalStreamType::DATA,
                                                                            std::nullopt);
                                   })
                            .chunks;
                    });
                break;
            case ScalarType::INT_64:
                encodedChunks = encodeSeparatedColumn.template operator()<std::int64_t>(
                    util::overloaded{
                        [](std::int64_t v) -> std::int64_t { return v; },
                        [](std::int32_t v) -> std::int64_t { return v; },
                        [](auto) -> std::int64_t { throwInvalidType(); }, // GCOVR_EXCL_LINE
                    },
                    [&](auto dataValues, const auto& presentValues, bool hasNull) {
                        return PropertyEncoder::encodeSeparatedDataColumnChunked(
                                   dataValues,
                                   presentValues,
                                   hasNull,
                                   column.nullable,
                                   [&](std::span<const std::int64_t> input) {
                                       return intEncoder.encodeLongStream(
                                           input, true, metadata::stream::PhysicalStreamType::DATA, std::nullopt);
                                   })
                            .chunks;
                    });
                break;
            case ScalarType::UINT_64:
                encodedChunks = encodeSeparatedColumn.template operator()<std::uint64_t>(
                    util::overloaded{
                        [](std::uint64_t v) -> std::uint64_t { return v; },
                        [](std::int64_t v) -> std::uint64_t { return static_cast<std::uint64_t>(v); },
                        [](auto) -> std::uint64_t { throwInvalidType(); }, // GCOVR_EXCL_LINE
                    },
                    [&](auto dataValues, const auto& presentValues, bool hasNull) {
                        return PropertyEncoder::encodeSeparatedDataColumnChunked(
                                   dataValues,
                                   presentValues,
                                   hasNull,
                                   column.nullable,
                                   [&](std::span<const std::uint64_t> input) {
                                       return intEncoder.encodeUint64Stream(
                                           input, metadata::stream::PhysicalStreamType::DATA, std::nullopt);
                                   })
                            .chunks;
                    });
                break;
            case ScalarType::FLOAT:
                encodedChunks = encodeSeparatedColumn.template operator()<float>(
                    util::overloaded{
                        [](float v) -> float { return v; },
                        [](double v) -> float { return static_cast<float>(v); },
                        [](auto) -> float { throwInvalidType(); }, // GCOVR_EXCL_LINE
                    },
                    [&](auto dataValues, const auto& presentValues, bool hasNull) {
                        return PropertyEncoder::encodeSeparatedDataColumnChunked(
                                   dataValues,
                                   presentValues,
                                   hasNull,
                                   column.nullable,
                                   [](std::span<const float> input) { return FloatEncoder::encodeStream(input); })
                            .chunks;
                    });
                break;
            case ScalarType::DOUBLE:
                encodedChunks = encodeSeparatedColumn.template operator()<double>(
                    util::overloaded{
                        [](double v) -> double { return v; },
                        [](float v) -> double { return static_cast<double>(v); },
                        [](auto) -> double { throwInvalidType(); }, // GCOVR_EXCL_LINE
                    },
                    [&](auto dataValues, const auto& presentValues, bool hasNull) {
                        return PropertyEncoder::encodeSeparatedDataColumnChunked(
                                   dataValues,
                                   presentValues,
                                   hasNull,
                                   column.nullable,
                                   [](std::span<const double> input) { return FloatEncoder::encodeStream(input); })
                            .chunks;
                    });
                break;
            case ScalarType::STRING: {
                std::vector<std::string> coercedStrings;
                coercedStrings.reserve(features.size());
                std::vector<bool> presentValues;
                presentValues.reserve(features.size());
                std::vector<std::string_view> dataValues;
                dataValues.reserve(features.size());
                for (const auto* propertyValue : cachedPropertyValues) {
                    if (propertyValue != nullptr) {
                        presentValues.push_back(true);
                        dataValues.push_back(std::visit(
                            util::overloaded{
                                [](const std::string& v) -> std::string_view { return std::string_view{v}; },
                                [](const Encoder::StructValue&) -> std::string_view { return std::string_view{}; },
                                [&](auto v) -> std::string_view {
                                    auto& coerced = coercedStrings.emplace_back(std::to_string(v));
                                    return std::string_view{coerced};
                                },
                            },
                            *propertyValue));
                    } else {
                        presentValues.push_back(false);
                    }
                }
                encodedChunks =
                    PropertyEncoder::encodeStringColumnChunkedFromSeparated(
                        dataValues, presentValues, physicalTechnique, intEncoder, config.useFsst, column.nullable)
                        .chunks;
                break;
            }
            default:
                throwInvalidType(); // GCOVR_EXCL_LINE
        }
        appendEncodedColumnChunks(std::move(encodedChunks));
    }

    // Compute total body size, then assemble the layer in a single allocation.
    std::size_t bodySize = 0;
    for (const auto& chunk : bodyChunks) {
        bodySize += chunk.size();
    }

    std::vector<std::uint8_t> layerBytes;
    layerBytes.reserve(metadataBytes.size() + bodySize + 8 /* varint overhead */);
    util::encoding::encodeVarint(static_cast<std::uint32_t>(1), layerBytes);
    layerBytes.insert(layerBytes.end(), metadataBytes.begin(), metadataBytes.end());
    for (const auto& chunk : bodyChunks) {
        layerBytes.insert(layerBytes.end(), chunk.begin(), chunk.end());
    }
    return layerBytes;
}

Encoder::Encoder()
    : impl(std::make_unique<Impl>()) {}

Encoder::~Encoder() noexcept = default;

std::vector<std::uint8_t> Encoder::encode(const std::vector<Layer>& layers, const EncoderConfig& config) {
    // Accumulate (size-varint, layer-bytes) pairs, then concatenate once.
    struct LayerChunk {
        std::vector<std::uint8_t> sizeVarint;
        std::vector<std::uint8_t> data;
    };
    std::vector<LayerChunk> chunks;
    chunks.reserve(layers.size());
    std::size_t totalSize = 0;
    for (const auto& layer : layers) {
        auto layerBytes = impl->encodeLayer(layer, config);
        if (layerBytes.empty()) {
            continue;
        }
        std::vector<std::uint8_t> sizeVarint;
        util::encoding::encodeVarint(static_cast<std::uint32_t>(layerBytes.size()), sizeVarint);
        totalSize += sizeVarint.size() + layerBytes.size();
        chunks.push_back({std::move(sizeVarint), std::move(layerBytes)});
    }
    std::vector<std::uint8_t> result;
    result.reserve(totalSize);
    for (const auto& chunk : chunks) {
        result.insert(result.end(), chunk.sizeVarint.begin(), chunk.sizeVarint.end());
        result.insert(result.end(), chunk.data.begin(), chunk.data.end());
    }
    return result;
}

} // namespace mlt
