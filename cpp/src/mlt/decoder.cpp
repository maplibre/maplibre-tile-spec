#include <mlt/decoder.hpp>

#include <mlt/decode/geometry.hpp>
#include <mlt/decode/int.hpp>
#include <mlt/decode/property.hpp>
#include <mlt/decode/string.hpp>
#include <mlt/layer.hpp>
#include <mlt/metadata/stream.hpp>
#include <mlt/metadata/tileset.hpp>
#include <mlt/properties.hpp>
#include <mlt/util/varint.hpp>
#include <mlt/util/rle.hpp>

#include <cstddef>
#include <stdexcept>

namespace mlt::decoder {

using namespace util::decoding;

namespace {
static constexpr std::string_view ID_COLUMN_NAME = "id";
static constexpr std::string_view GEOMETRY_COLUMN_NAME = "geometry";
} // namespace

struct Decoder::Impl {
    IntegerDecoder integerDecoder;
    StringDecoder stringDecoder{integerDecoder};
    PropertyDecoder propertyDecoder{integerDecoder, stringDecoder};
};

Decoder::Decoder() noexcept(false)
    : impl{std::make_unique<Impl>()} {}

Decoder::~Decoder() = default;

MapLibreTile Decoder::decode(DataView tileData_, const TileSetMetadata& tileMetadata) noexcept(false) {
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
        const auto [featureTableId, tileExtent, maxTileExtent, numFeatures] = decodeVarints<std::uint32_t, 4>(tileData);

        // TODO: what is `maxTileExtent`?

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
                GeometryDecoder decoder;
                const auto geometryColumn = decoder.decodeGeometryColumn(tileData, columnMetadata, numStreams);
                geometries = decoder.decodeGeometry(geometryColumn);
            } else {
                const auto property = impl->propertyDecoder.decodePropertyColumn(tileData, columnMetadata, numStreams);
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
    Property operator()(const T& vec) const;

    template <typename T>
    Property operator()(const std::vector<T>& vec) const {
        if (i < vec.size()) {
            return vec[i];
        }
        assert(false);
        return nullptr;
    }
    template <>
    Property operator()(const StringDictViews& vec) const {
        if (i < vec.second.size()) {
            return vec.second[i];
        }
        assert(false);
        return nullptr;
    }
    template <>
    Property operator()(const std::vector<std::uint8_t>& vec) const {
        return (i / 8 < vec.size()) && (vec[i / 8] & (1 << (i & 8)));
    }
};
}

std::vector<Feature> Decoder::makeFeatures(const std::vector<Feature::id_t>& ids,
                                           std::vector<std::unique_ptr<Geometry>>&& geometries,
                                           const PropertyVecMap& propertyVecs) noexcept(false) {
    std::vector<Feature> features;
    features.reserve(ids.size());

    std::unique_ptr<Geometry> noGeometry;

    for (count_t i = 0; i < ids.size(); ++i) {
        // TODO: Consider having features reference their parent layers and
        //       get properties on-demand instead of splaying them out here.
        PropertyMap properties;
        properties.reserve(propertyVecs.size());
        for (const auto& [key, vec] : propertyVecs) {
            const auto value = std::visit(ExtractPropertyVisitor{i}, vec);
            if (!std::holds_alternative<nullptr_t>(value)) {
                properties.emplace(key, std::move(value));
            }
        }

        auto& geom = (i < geometries.size()) ? geometries[i] : noGeometry;
        features.push_back(Feature{ids[i], std::move(geom), std::move(properties)});
    }
    return features;
}

} // namespace mlt::decoder
