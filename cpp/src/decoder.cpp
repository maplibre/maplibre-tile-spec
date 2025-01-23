#include <decoder.hpp>

#include <decode/geometry.hpp>
#include <decode/int.hpp>
#include <decode/property.hpp>
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
};

Decoder::Decoder()
    : impl{std::make_unique<Impl>()} {}

Decoder::~Decoder() = default;

MapLibreTile Decoder::decode(DataView tileData_, const TileSetMetadata& tileMetadata) {
    using namespace metadata;
    using namespace metadata::tileset;
    using namespace util::decoding;

    auto tileData = BufferStream{tileData_};

    std::vector<Layer> layers;
    // layers.reserve(...);

    std::vector<std::uint8_t> buffer;

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
                    const auto presentStreamMetadata = stream::decode(tileData);
                    // data ignored, don't decode it
                    tileData.consume(presentStreamMetadata->getByteLength());
                }

                const auto idDataStreamMetadata = stream::decode(tileData);
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
                            tileData, ids, *idDataStreamMetadata, false);
                        break;
                    case metadata::tileset::schema::ScalarType::INT_64:
                    case metadata::tileset::schema::ScalarType::UINT_64:
                        impl->integerDecoder.decodeIntStream<std::uint64_t>(
                            tileData, ids, *idDataStreamMetadata, false);
                        break;
                    default:
                        throw std::runtime_error("unsupported id data type");
                }
            } else if (columnName == GEOMETRY_COLUMN_NAME) {
                GeometryDecoder decoder;
                const auto geometryColumn = decoder.decodeGeometryColumn(tileData, columnMetadata, numStreams);
                geometries = decoder.decodeGeometry(geometryColumn);
                break;
            } else {
                const auto propertyColumn = PropertyDecoder::decodePropertyColumn(tileData, columnMetadata, numStreams);
                // if (propertyColumn instanceof Map<?, ?> map) {
                //     for (const auto& [key, value] : std::get<std::map<std::string,
                //     std::variant<std::vector<std::uint8_t>, std::string>>>(propertyColumn)) {
                //         properties[key] = std::holds_alternative<std::vector<std::uint8_t>>(value) ?
                //             std::get<std::vector<std::uint8_t>>(value) :
                //             std::vector<std::uint8_t>{std::get<std::string>(value).begin(),
                //             std::get<std::string>(value).end()};
                //     }
                // } else if (std::holds_alternative<std::vector<std::uint8_t>>(propertyColumn)) {
                //     properties[columnName] = std::get<std::vector<std::uint8_t>>(propertyColumn);
                // }
                break;
            }
        }
        break;
    }
    return {std::move(layers)};
}

} // namespace mlt::decoder
