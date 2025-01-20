#include <decoder.hpp>

#include <decode/ints.hpp>
#include <metadata/stream.hpp>
#include <metadata/tileset.hpp>
#include <util/varint.hpp>
#include <util/rle.hpp>

namespace mlt::decoder {

using namespace util::decoding;

namespace {
static constexpr std::string_view ID_COLUMN_NAME = "id";
}

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
        std::vector<Geometry> geometries;
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
                    const auto bitCount = presentStreamMetadata->getNumValues();
                    buffer.resize((bitCount + 7) / 8);
                    rle::decodeBoolean(tileData, buffer.data(), bitCount, presentStreamMetadata->getByteLength());
                    // TODO: result ignored?  We don't need to decode it then...
                    buffer.clear();
                }

                const auto idDataStreamMetadata = stream::decode(tileData);
                if (!std::holds_alternative<ScalarColumn>(columnMetadata.type) ||
                    !std::holds_alternative<ScalarType>(std::get<ScalarColumn>(columnMetadata.type).type)) {
                    throw std::runtime_error("id column must be scalar and physical");
                }
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
#if 0
          if (idDataType.equals(MltTilesetMetadata.ScalarType.UINT_32)) {
            ids =
                IntegerDecoder.decodeIntStream(tileData, idDataStreamMetadata, false).stream()
                    .mapToLong(i -> i)
                    .boxed()
                    .toList();
          } else {
            ids = IntegerDecoder.decodeLongStream(tileData, idDataStreamMetadata, false);
          }
#endif
            }
#if 0
        } else if (columnName.equals(GEOMETRY_COLUMN_NAME)) {
          var geometryColumn = GeometryDecoder.decodeGeometryColumn(tileData, numStreams);
          geometries = GeometryDecoder.decodeGeometry(geometryColumn);
        } else {
          var propertyColumn =
              PropertyDecoder.decodePropertyColumn(tileData, columnMetadata, numStreams);
          if (propertyColumn instanceof Map<?, ?> map) {
            for (var a : map.entrySet()) {
              properties.put(
                  a.getKey().toString(),
                  a.getValue() instanceof List<?> list ? list : List.of(a.getValue()));
            }
          } else if (propertyColumn instanceof List<?> list) {
            properties.put(columnName, list);
          }
        }
#endif

            break;
        }
        break;
    }
    return {std::move(layers)};
}

} // namespace mlt::decoder
