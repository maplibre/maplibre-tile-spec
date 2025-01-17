#include <decoder.hpp>

#include <metadata/stream.hpp>
#include <util/varint.hpp>
#include <util/rle.hpp>
#include "metadata/tileset.hpp"

namespace mlt::decoder {

using namespace util::decoding;

namespace {
static constexpr std::string_view ID_COLUMN_NAME = "id";
}

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
        const auto [featureTableId, tileExtent, maxTileExtent, numFeatures] = decodeVarints<4>(tileData);

        if (featureTableId < 0 || tileMetadata.featureTables.size() <= featureTableId) {
            throw std::runtime_error("invalid table id");
        }
        const auto& tableMetadata = tileMetadata.featureTables[featureTableId];

        for (const auto& columnMetadata : tableMetadata.columns) {
            const auto& columnName = columnMetadata.name;
            const auto numStreams = decodeVarint(tileData);

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
#if 0
                if (idDataType == ScalarType::UINT_32) {
                    ids = rle::decodeInt<int>(tileData, offset, idDataStreamMetadata, false);
                } else {
                    ids = rle::decodeInt<long>(tileData, offset, idDataStreamMetadata, false);
                }
#endif
#if 0
          if (idDataType.equals(MltTilesetMetadata.ScalarType.UINT_32)) {
            ids =
                IntegerDecoder.decodeIntStream(tile, offset, idDataStreamMetadata, false).stream()
                    .mapToLong(i -> i)
                    .boxed()
                    .toList();
          } else {
            ids = IntegerDecoder.decodeLongStream(tile, offset, idDataStreamMetadata, false);
          }
#endif
            }
#if 0
        } else if (columnName.equals(GEOMETRY_COLUMN_NAME)) {
          var geometryColumn = GeometryDecoder.decodeGeometryColumn(tile, numStreams, offset);
          geometries = GeometryDecoder.decodeGeometry(geometryColumn);
        } else {
          var propertyColumn =
              PropertyDecoder.decodePropertyColumn(tile, offset, columnMetadata, numStreams);
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
