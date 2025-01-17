#include <decoder.hpp>

#include <decoding_utils.hpp>
#include <stdexcept>

namespace mlt::decoder {

using namespace util::decoding;

namespace {
static constexpr std::string_view ID_COLUMN_NAME = "id";
}

MapLibreTile Decoder::decode(DataView tileData, const TileSetMetadata& tileMetadata) {
    offset_t offset = 0;
    std::vector<Layer> layers;
    // layers.reserve(...);

    while (offset < tileData.size()) {
        std::vector<Feature::id_t> ids;
        std::vector<Geometry> geometries;
        Feature::PropertyMap properties;

        const auto version = tileData[offset++];
        const auto [featureTableId, tileExtent, maxTileExtent, numFeatures] = decodeVarints<4>(tileData, offset);

        if (featureTableId < 0 || tileMetadata.featureTables.size() <= featureTableId) {
            throw std::runtime_error("invalid table id");
        }
        const auto& tableMetadata = tileMetadata.featureTables[featureTableId];

        for (const auto& columnMetadata : tableMetadata.columns) {
            const auto& columnName = columnMetadata.name;
            const auto numStreams = decodeVarint(tileData, offset);

            // TODO: add decoding of vector type to be compliant with the spec
            // TODO: compare based on ids

            if (columnName == ID_COLUMN_NAME) {
                if (numStreams == 2) {
                    // auto presentStreamMetadata = StreamMetadataDecoder.decode(tileData, offset);
                    // auto presentStream = DecodingUtils.decodeBooleanRle(tileData, presentStreamMetadata.numValues(),
                    // presentStreamMetadata.byteLength(), offset);
                }
            }
            break;
        }
        break;
    }
    return {std::move(layers)};
}

} // namespace mlt::decoder
