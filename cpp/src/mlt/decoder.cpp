#include <mlt/decoder.hpp>

#include <mlt/decode/geometry.hpp>
#include <mlt/decode/int.hpp>
#include <mlt/decode/int_template.hpp>
#include <mlt/decode/property.hpp>
#include <mlt/decode/string.hpp>
#include <mlt/geometry.hpp>
#include <mlt/geometry_vector.hpp>
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

namespace mlt {

using namespace decoder;
using namespace util::decoding;

namespace {
constexpr std::string_view ID_COLUMN_NAME = "id";
constexpr std::string_view GEOMETRY_COLUMN_NAME = "geometry";
} // namespace

struct Decoder::Impl {
    Impl(std::unique_ptr<GeometryFactory>&& geometryFactory_)
        : geometryFactory(std::move(geometryFactory_)) {}

    IntegerDecoder integerDecoder;
    StringDecoder stringDecoder{integerDecoder};
    PropertyDecoder propertyDecoder{integerDecoder, stringDecoder};
    GeometryDecoder geometryDecoder;
    std::unique_ptr<GeometryFactory> geometryFactory;
};

Decoder::Decoder(std::unique_ptr<GeometryFactory>&& geometryFactory)
    : impl{std::make_unique<Impl>(std::move(geometryFactory))} {}

Decoder::~Decoder() noexcept = default;

MapLibreTile Decoder::decode(DataView tileData_, const TileMetadata& tileMetadata) {
    auto tileData = BufferStream{tileData_};
    return decode(tileData, tileMetadata);
}

MapLibreTile Decoder::decode(BufferStream& tileData, const TileMetadata& tileMetadata) {
    using namespace metadata;
    using namespace metadata::stream;
    using namespace metadata::tileset;
    using namespace util::decoding;

    std::vector<Layer> layers;
    // layers.reserve(...);

    while (tileData.available()) {
        std::vector<Feature::id_t> ids;
        std::unique_ptr<geometry::GeometryVector> geometryVector;
        PropertyVecMap properties;

        const auto version = tileData.read();
        const auto featureTableId = decodeVarint<std::uint32_t>(tileData);
        const auto featureTableBodySize = decodeVarint<std::uint32_t>(tileData);
        const auto [tileExtent, maxTileExtent, numFeatures] = decodeVarints<std::uint32_t, 3>(tileData);

        if (tileExtent == 0) {
            throw std::runtime_error("invalid tile extent");
        }
        if (tileMetadata.featureTables.size() <= featureTableId) {
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
                geometryVector = impl->geometryDecoder.decodeGeometryColumn(tileData, columnMetadata, numStreams);
            } else {
                auto property = impl->propertyDecoder.decodePropertyColumn(tileData, columnMetadata, numStreams);
                if (property) {
                    properties.emplace(columnMetadata.name, std::move(*property));
                }
            }
        }

        layers.emplace_back(tableMetadata.name,
                            version,
                            tileExtent,
                            std::move(geometryVector),
                            makeFeatures(ids, geometryVector->getGeometries(*impl->geometryFactory)),
                            std::move(properties));
    }
    return {std::move(layers)};
}

std::vector<Feature> Decoder::makeFeatures(const std::vector<Feature::id_t>& ids,
                                           std::vector<std::unique_ptr<Geometry>>&& geometries) {
    const auto featureCount = ids.size();
    if (geometries.size() < featureCount) {
        throw std::runtime_error("Invalid geometry count");
    }

    return util::generateVector<Feature>(featureCount, [&](const auto i) {
        return Feature{ids[i], std::move(geometries[i]), static_cast<std::uint32_t>(i)};
    });
}

} // namespace mlt
