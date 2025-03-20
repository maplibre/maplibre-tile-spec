#include <mlt/decoder.hpp>

#include <mlt/decode/geometry.hpp>
#include <mlt/decode/int.hpp>
#include <mlt/decode/int_template.hpp>
#include <mlt/decode/property.hpp>
#include <mlt/decode/string.hpp>
#include <mlt/layer.hpp>
#include <mlt/metadata/stream.hpp>
#include <mlt/metadata/tileset.hpp>
#include <mlt/properties.hpp>
#include <mlt/util/packed_bitset.hpp>
#include <mlt/util/rle.hpp>
#include <mlt/util/stl.hpp>
#include <mlt/util/varint.hpp>

#include <cstddef>
#include <stdexcept>
#include "mlt/geometry.hpp"

namespace mlt {

using namespace decoder;
using namespace util::decoding;

namespace {
static constexpr std::string_view ID_COLUMN_NAME = "id";
static constexpr std::string_view GEOMETRY_COLUMN_NAME = "geometry";
} // namespace

struct Decoder::Impl {
    Impl(std::unique_ptr<GeometryFactory>&& geometryFactory)
        : geometryDecoder(std::move(geometryFactory)) {}

    IntegerDecoder integerDecoder;
    StringDecoder stringDecoder{integerDecoder};
    PropertyDecoder propertyDecoder{integerDecoder, stringDecoder};
    GeometryDecoder geometryDecoder;
};

Decoder::Decoder(bool legacy_, std::unique_ptr<GeometryFactory>&& geometryFactory)
    : impl{std::make_unique<Impl>(std::move(geometryFactory))},
      legacy(legacy_) {}

Decoder::~Decoder() noexcept = default;

MapLibreTile Decoder::decode(DataView tileData_, const TileSetMetadata& tileMetadata) {
    using namespace metadata;
    using namespace metadata::stream;
    using namespace metadata::tileset;
    using namespace util::decoding;

    auto tileData = BufferStream{tileData_};

    std::vector<Layer> layers;
    // layers.reserve(...);

    while (tileData.available()) {
        std::vector<Feature::id_t> ids;
        std::vector<std::unique_ptr<Geometry>> geometries;
        PropertyVecMap properties;

        const auto version = tileData.read();
        const auto featureTableId = decodeVarint<std::uint32_t>(tileData);
        const auto featureTableBodySize = legacy ? -1 : decodeVarint<std::uint32_t>(tileData);
        const auto [tileExtent, maxTileExtent, numFeatures] = decodeVarints<std::uint32_t, 3>(tileData);

        if (tileExtent == 0) {
            throw std::runtime_error("invalid tile extent");
        }
        if (featureTableId < 0 || tileMetadata.featureTables.size() <= featureTableId) {
            throw std::runtime_error("invalid table id");
        }
        const auto& tableMetadata = tileMetadata.featureTables[featureTableId];

        for (const auto& columnMetadata : tableMetadata.columns) {
            const auto& columnName = columnMetadata.name;
            const auto numStreams = decodeVarint<std::uint32_t>(tileData);

            // TODO: add decoding of vector type to be compliant with the spec
            // TODO: compare based on ids

            if (columnName == ID_COLUMN_NAME) {
                if (numStreams == 2) {
                    const auto presentStreamMetadata = StreamMetadata::decode(tileData);
                    // data ignored, don't decode it
                    tileData.consume(presentStreamMetadata->getByteLength());
                } else {
                    throw std::runtime_error("Unsupported number of streams for ID column: " +
                                             std::to_string(numStreams));
                }

                const auto idDataStreamMetadata = StreamMetadata::decode(tileData);
                if (!std::holds_alternative<ScalarColumn>(columnMetadata.type) ||
                    !std::holds_alternative<ScalarType>(std::get<ScalarColumn>(columnMetadata.type).type)) {
                    throw std::runtime_error("id column must be scalar and physical");
                }
                ids.resize(idDataStreamMetadata->getNumValues());
                const auto idDataType = std::get<ScalarType>(std::get<ScalarColumn>(columnMetadata.type).type);
                switch (idDataType) {
                    case metadata::tileset::schema::ScalarType::INT_32:
                    case metadata::tileset::schema::ScalarType::UINT_32:
                        impl->integerDecoder.decodeIntStream<std::uint32_t, std::uint64_t>(
                            tileData, ids, *idDataStreamMetadata);
                        break;
                    case metadata::tileset::schema::ScalarType::INT_64:
                    case metadata::tileset::schema::ScalarType::UINT_64:
                        impl->integerDecoder.decodeIntStream<std::uint64_t>(tileData, ids, *idDataStreamMetadata);
                        break;
                    default:
                        throw std::runtime_error("unsupported id data type");
                }
            } else if (columnName == GEOMETRY_COLUMN_NAME) {
                const auto geometryColumn = impl->geometryDecoder.decodeGeometryColumn(
                    tileData, columnMetadata, numStreams);
                geometries = impl->geometryDecoder.decodeGeometry(geometryColumn);
            } else {
                auto property = impl->propertyDecoder.decodePropertyColumn(tileData, columnMetadata, numStreams);
                properties.emplace(columnMetadata.name, std::move(property));
            }
        }

        if (numFeatures != ids.size() || numFeatures != geometries.size()) {
            throw std::runtime_error("ids and features mismatch");
        }
        auto features = makeFeatures(ids, std::move(geometries), properties);
        layers.emplace_back(tableMetadata.name, version, tileExtent, std::move(features), std::move(properties));
    }
    return {std::move(layers)};
}

namespace {
struct ExtractPropertyVisitor {
    const std::size_t i;

    template <typename T>
    std::optional<Property> operator()(const T& vec) const;

    template <typename T>
    std::optional<Property> operator()(const std::vector<T>& vec) const {
        assert(i < vec.size());
        return (i < vec.size()) ? std::optional<Property>{vec[i]} : std::nullopt;
    }
};

template <>
std::optional<Property> ExtractPropertyVisitor::operator()(const StringDictViews& views) const {
    const auto& strings = views.getStrings();
    assert(i < strings.size());
    return (i < strings.size()) ? std::optional<Property>{strings[i]} : std::nullopt;
}
template <>
std::optional<Property> ExtractPropertyVisitor::operator()(const PackedBitset& vec) const {
    return testBit(vec, i);
}

} // namespace

std::vector<Feature> Decoder::makeFeatures(const std::vector<Feature::id_t>& ids,
                                           std::vector<std::unique_ptr<Geometry>>&& geometries,
                                           const PropertyVecMap& propertyVecs) {
    const auto featureCount = ids.size();
    if (geometries.size() < featureCount) {
        throw std::runtime_error("Invalid geometry count");
    }

    // TODO: Consider having features reference their parent layers and
    //       get properties on-demand instead of splaying them out here.
    std::vector<PropertyMap> properties(featureCount);
    for (const auto& [key, column] : propertyVecs) {
        const auto& layerProperties = column.getProperties();
        const auto& presentBits = column.getPresentBits();

        const auto applyProperty = [&](std::size_t sourceIndex, std::size_t targetIndex) {
            if (const auto value = std::visit(ExtractPropertyVisitor{sourceIndex}, layerProperties); value) {
                properties[targetIndex].emplace(key, *value);
            }
        };

        // If there's a "present" bitstream, the index is within present values only
        if (!presentBits.empty()) {
            if (presentBits.size() < (featureCount + 7) / 8) {
                throw std::runtime_error("Invalid present stream");
            }

            // This should have been checked during property construction
            assert(propertyCount(layerProperties) == countSetBits(presentBits));

            // Place property N in the properties map for the feature corresponding to the Nth set bit
            std::size_t sourceIndex = 0;
            for (auto i = nextSetBit(presentBits, 0); i; i = nextSetBit(presentBits, *i + 1)) {
                applyProperty(sourceIndex++, *i);
            }
        } else {
            // Place property N in the properties map for feature N
            for (std::size_t i = 0; i < featureCount; ++i) {
                applyProperty(i, i);
            }
        }
    }

    return util::generateVector<Feature>(featureCount, [&](const auto i) {
        return Feature{ids[i], std::move(geometries[i]), std::move(properties[i])};
    });
}

} // namespace mlt
