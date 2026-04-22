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
    IntegerEncoder intEncoder;

    static bool structValueHasChild(const Encoder::StructValue& sv,
                                    const std::string& rootName,
                                    const std::string& childName);
    static const std::string* resolveStructSourceKey(const std::vector<Feature>& features,
                                                     const metadata::tileset::Column& column);

    std::vector<std::uint8_t> encodeLayer(const Layer& layer, const EncoderConfig& config);

    FeatureTable buildMetadata(const Layer& layer, const EncoderConfig& config);

    static void collectGeometry(const std::vector<Feature>& features,
                                std::vector<GeometryType>& geometryTypes,
                                std::vector<std::uint32_t>& numGeometries,
                                std::vector<std::uint32_t>& numParts,
                                std::vector<std::uint32_t>& numRings,
                                std::vector<GeometryEncoder::Vertex>& vertexBuffer);

    static bool canSort(const std::vector<Encoder::Feature>& features);
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

FeatureTable Encoder::Impl::buildMetadata(const Layer& layer, const EncoderConfig& config) {
    FeatureTable table;
    table.name = layer.name;
    table.extent = layer.extent;

    if (config.includeIds) {
        // Use 64-bit when any ID exceeds INT32_MAX: delta encoding accumulates in
        // int32_t, so uint32 values with bit 31 set would sign-extend on widening.
        const bool hasLongId = std::ranges::any_of(layer.features, [](const auto& f) {
            return f.id.has_value() && (*f.id > std::numeric_limits<std::int32_t>::max());
        });
        const bool hasMissingId = std::ranges::any_of(layer.features, [](const auto& f) { return !f.id.has_value(); });

        table.columns.push_back(Column{
            .nullable = hasMissingId,
            .columnScope = ColumnScope::FEATURE,
            .type = ScalarColumn{.type = LogicalScalarType::ID, .hasLongID = hasLongId},
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

bool Encoder::Impl::canSort(const std::vector<Encoder::Feature>& features) {
    if (features.empty()) {
        return false;
    }

    const auto firstType = features[0].geometry.type;
    const bool allSame = std::ranges::all_of(features,
                                             [firstType](const auto& f) { return f.geometry.type == firstType; });
    return allSame && (firstType == GeometryType::POINT || firstType == GeometryType::LINESTRING);
}

std::vector<Encoder::Feature> Encoder::Impl::sortFeatures(const std::vector<Encoder::Feature>& features) {
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

    const bool shouldSort = config.sortFeatures && canSort(layer.features);
    const auto sortedStorage = shouldSort ? sortFeatures(layer.features) : std::vector<Feature>{};
    const auto& features = shouldSort ? sortedStorage : layer.features;

    const auto physicalTechnique = config.useFastPfor ? PhysicalLevelTechnique::FAST_PFOR
                                                      : PhysicalLevelTechnique::VARINT;

    const auto featureTable = buildMetadata(layer, config);
    const auto metadataBytes = encodeFeatureTable(featureTable);

    std::vector<std::uint8_t> bodyBytes;
    const auto appendEncodedColumn = [&](const auto& encoded) {
        bodyBytes.insert(bodyBytes.end(), encoded.begin(), encoded.end());
    };
    const auto appendEncodedStreamSet = [&](std::uint32_t numStreams, const auto& encodedValues) {
        util::encoding::encodeVarint(numStreams, bodyBytes);
        bodyBytes.insert(bodyBytes.end(), encodedValues.begin(), encodedValues.end());
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
        const bool hasLongId = std::ranges::any_of(features, [](const auto& f) {
            return f.id.has_value() && (*f.id > std::numeric_limits<std::int32_t>::max());
        });
        const bool hasMissingId = std::ranges::any_of(features, [](const auto& f) { return !f.id.has_value(); });

        if (hasLongId) {
            auto ids = extractIds.template operator()<std::uint64_t>(
                [](const auto& feature) -> std::optional<std::uint64_t> { return feature.id; });
            appendEncodedColumn(PropertyEncoder::encodeUint64Column(ids, intEncoder, hasMissingId));
        } else {
            auto ids = extractIds.template operator()<std::int32_t>(
                [](const auto& feature) -> std::optional<std::int32_t> {
                    return feature.id.has_value() ? std::optional<std::int32_t>{static_cast<std::int32_t>(*feature.id)}
                                                  : std::nullopt;
                });
            appendEncodedColumn(
                PropertyEncoder::encodeInt32Column(ids, physicalTechnique, false, intEncoder, hasMissingId));
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

    appendEncodedStreamSet(encodedGeom.numStreams, encodedGeom.encodedValues);

    for (const auto& column : featureTable.columns) {
        if (column.isID() || column.isGeometry()) {
            continue;
        }

        if (column.isStruct()) {
            const auto& complex = column.getComplexType();
            const auto& rootName = column.name;
            const auto sourceKey = resolveStructSourceKey(features, column);
            const auto numChildren = complex.children.size();
            const auto getStructValue = [&](const auto& feature) -> const Encoder::StructValue* {
                auto it = sourceKey ? feature.properties.find(*sourceKey) : feature.properties.find(rootName);
                if (it != feature.properties.end() && std::holds_alternative<Encoder::StructValue>(it->second)) {
                    return &std::get<Encoder::StructValue>(it->second);
                }
                return nullptr;
            };
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

            std::vector<std::vector<std::string>> ownedStrings(numChildren);
            std::vector<std::vector<std::string_view>> viewStorage(numChildren);

            for (std::size_t c = 0; c < numChildren; ++c) {
                ownedStrings[c].reserve(features.size());
                viewStorage[c].reserve(features.size());
            }

            for (const auto& f : features) {
                const auto* sv = getStructValue(f);
                for (std::size_t c = 0; c < numChildren; ++c) {
                    if (sv) {
                        if (const auto* childValue = resolveStructChildValue(*sv, complex.children[c].name);
                            childValue != nullptr) {
                            ownedStrings[c].push_back(*childValue);
                            continue;
                        }
                    }
                    ownedStrings[c].emplace_back();
                }
            }

            for (std::size_t c = 0; c < numChildren; ++c) {
                for (const auto& s : ownedStrings[c]) {
                    viewStorage[c].emplace_back(s);
                }
            }

            std::vector<std::vector<const std::string_view*>> sharedCols(numChildren);
            for (std::size_t c = 0; c < numChildren; ++c) {
                sharedCols[c].reserve(features.size());
                for (std::size_t fi = 0; fi < features.size(); ++fi) {
                    const auto* sv = getStructValue(features[fi]);
                    if (sv && structValueHasChild(*sv, rootName, complex.children[c].name)) {
                        sharedCols[c].push_back(&viewStorage[c][fi]);
                    } else {
                        sharedCols[c].push_back(nullptr);
                    }
                }
            }

            auto result = StringEncoder::encodeSharedDictionary(
                sharedCols, physicalTechnique, intEncoder, config.useFsst);

            appendEncodedStreamSet(result.numStreams, result.data);
            continue;
        }

        const auto& scalarCol = column.getScalarType();
        const auto scalarType = scalarCol.getPhysicalType();
        const auto& colName = column.name;

        const auto extractColumn = [&]<typename T>(auto&& visitor) {
            std::vector<std::optional<T>> values;
            values.reserve(features.size());
            for (const auto& f : features) {
                auto it = f.properties.find(colName);
                if (it != f.properties.end()) {
                    values.push_back(std::visit(visitor, it->second));
                } else {
                    values.push_back(std::nullopt);
                }
            }
            return values;
        };

        const auto encodeExtractedColumn = [&]<typename T>(auto&& visitor, auto&& encodeColumn) {
            auto values = extractColumn.template operator()<T>(visitor);
            return encodeColumn(values);
        };

        std::vector<std::uint8_t> encoded;
        switch (scalarType) {
            case ScalarType::BOOLEAN:
                encoded = encodeExtractedColumn.template operator()<bool>(
                    util::overloaded{
                        [](bool v) -> bool { return v; },
                        // the type is already determined by column metadata, so the catch-all arms are dead by
                        // construction
                        [](auto) -> bool { throwInvalidType(); }, // GCOVR_EXCL_LINE
                    },
                    [&](const auto& values) { return PropertyEncoder::encodeBooleanColumn(values, column.nullable); });
                break;
            case ScalarType::INT_32:
                encoded = encodeExtractedColumn.template operator()<std::int32_t>(
                    util::overloaded{
                        [](std::int32_t v) -> std::int32_t { return v; },
                        [](std::int64_t v) -> std::int32_t { return static_cast<std::int32_t>(v); },
                        [](auto) -> std::int32_t { throwInvalidType(); }, // GCOVR_EXCL_LINE
                    },
                    [&](const auto& values) {
                        return PropertyEncoder::encodeInt32Column(
                            values, physicalTechnique, true, intEncoder, column.nullable);
                    });
                break;
            case ScalarType::UINT_32:
                encoded = encodeExtractedColumn.template operator()<std::uint32_t>(
                    util::overloaded{
                        [](std::uint32_t v) -> std::uint32_t { return v; },
                        [](std::int32_t v) -> std::uint32_t { return static_cast<std::uint32_t>(v); },
                        [](auto) -> std::uint32_t { throwInvalidType(); }, // GCOVR_EXCL_LINE
                    },
                    [&](const auto& values) {
                        return PropertyEncoder::encodeUint32Column(
                            values, physicalTechnique, intEncoder, column.nullable);
                    });
                break;
            case ScalarType::INT_64:
                encoded = encodeExtractedColumn.template operator()<std::int64_t>(
                    util::overloaded{
                        [](std::int64_t v) -> std::int64_t { return v; },
                        [](std::int32_t v) -> std::int64_t { return v; },
                        [](auto) -> std::int64_t { throwInvalidType(); }, // GCOVR_EXCL_LINE
                    },
                    [&](const auto& values) {
                        return PropertyEncoder::encodeInt64Column(values, true, intEncoder, column.nullable);
                    });
                break;
            case ScalarType::UINT_64:
                encoded = encodeExtractedColumn.template operator()<std::uint64_t>(
                    util::overloaded{
                        [](std::uint64_t v) -> std::uint64_t { return v; },
                        [](std::int64_t v) -> std::uint64_t { return static_cast<std::uint64_t>(v); },
                        [](auto) -> std::uint64_t { throwInvalidType(); }, // GCOVR_EXCL_LINE
                    },
                    [&](const auto& values) {
                        return PropertyEncoder::encodeUint64Column(values, intEncoder, column.nullable);
                    });
                break;
            case ScalarType::FLOAT:
                encoded = encodeExtractedColumn.template operator()<float>(
                    util::overloaded{
                        [](float v) -> float { return v; },
                        [](double v) -> float { return static_cast<float>(v); },
                        [](auto) -> float { throwInvalidType(); }, // GCOVR_EXCL_LINE
                    },
                    [&](const auto& values) { return PropertyEncoder::encodeFloatColumn(values, column.nullable); });
                break;
            case ScalarType::DOUBLE:
                encoded = encodeExtractedColumn.template operator()<double>(
                    util::overloaded{
                        [](double v) -> double { return v; },
                        [](float v) -> double { return static_cast<double>(v); },
                        [](auto) -> double { throwInvalidType(); }, // GCOVR_EXCL_LINE
                    },
                    [&](const auto& values) { return PropertyEncoder::encodeDoubleColumn(values, column.nullable); });
                break;
            case ScalarType::STRING: {
                std::vector<std::string> ownedStrings;
                ownedStrings.reserve(features.size());
                std::vector<std::optional<std::string_view>> values;
                values.reserve(features.size());
                for (const auto& f : features) {
                    auto it = f.properties.find(colName);
                    if (it != f.properties.end()) {
                        auto& owned = ownedStrings.emplace_back(
                            std::visit(util::overloaded{
                                           [](const std::string& v) -> std::string { return v; },
                                           [](const Encoder::StructValue&) -> std::string { return {}; },
                                           [](auto v) -> std::string { return std::to_string(v); },
                                       },
                                       it->second));
                        values.push_back(std::string_view{owned});
                    } else {
                        values.push_back(std::nullopt);
                    }
                }
                encoded = PropertyEncoder::encodeStringColumn(
                    values, physicalTechnique, intEncoder, config.useFsst, column.nullable);
                break;
            }
            default:
                throwInvalidType(); // GCOVR_EXCL_LINE
        }
        appendEncodedColumn(encoded);
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
