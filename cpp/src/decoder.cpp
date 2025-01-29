#include <decoder.hpp>

#include <decode/geometry.hpp>
#include <decode/int.hpp>
#include <decode/property.hpp>
#include <decode/string.hpp>
#include <layer.hpp>
#include <metadata/stream.hpp>
#include <metadata/tileset.hpp>
#include <util/varint.hpp>
#include <util/rle.hpp>

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
        Feature::PropertyMap properties;

        const auto version = tileData.read();
        const auto [featureTableId, tileExtent, maxTileExtent, numFeatures] = decodeVarints<std::uint32_t, 4>(tileData);

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

        auto features = makeFeatures(ids, std::move(geometries), std::move(properties), tileExtent, numFeatures);
        layers.emplace_back(tableMetadata.name, version, tileExtent, std::move(features));
    }
    return {std::move(layers)};
}

std::vector<Feature> Decoder::makeFeatures(const std::vector<Feature::id_t>& ids,
                                           std::vector<std::unique_ptr<Geometry>>&& geometries,
                                           const Feature::PropertyMap& /*properties*/,
                                           const Feature::extent_t extent,
                                           const count_t numFeatures) noexcept(false) {
    std::vector<Feature> features;
    features.reserve(numFeatures);

    assert(ids.size() == numFeatures);
    assert(geometries.size() == numFeatures);
    for (count_t j = 0; j < numFeatures; ++j) {
        features.push_back(Feature{ids[j], extent, std::move(geometries[j]), {}});
    }
    return features;
}

} // namespace mlt::decoder
