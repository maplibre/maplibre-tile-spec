#include <mlt/decoder.hpp>

#include <mlt/decode/geometry.hpp>
#include <mlt/decode/int.hpp>
#include <mlt/decode/int_template.hpp>
#include <mlt/decode/property.hpp>
#include <mlt/decode/string.hpp>
#include <mlt/geometry.hpp>
#include <mlt/geometry_vector.hpp>
#include <mlt/layer.hpp>
#include <mlt/metadata/type_map.hpp>
#include <mlt/metadata/stream.hpp>
#include <mlt/metadata/tileset.hpp>
#include <mlt/properties.hpp>
#include <mlt/tile.hpp>
#include <mlt/util/buffer_stream.hpp>
#include <mlt/util/packed_bitset.hpp>
#include <mlt/util/rle.hpp>
#include <mlt/util/stl.hpp>
#include <mlt/util/varint.hpp>

#include <cstddef>
#include <stdexcept>

namespace mlt {

using namespace decoder;
using namespace util::decoding;

struct Decoder::Impl {
    Impl(std::unique_ptr<GeometryFactory>&& geometryFactory_)
        : geometryFactory(std::move(geometryFactory_)) {}

    Layer parseBasicMVTEquivalent(BufferStream&);
    std::vector<Feature> makeFeatures(const std::vector<Feature::id_t>&, std::vector<std::unique_ptr<Geometry>>&&);

    IntegerDecoder integerDecoder;
    StringDecoder stringDecoder{integerDecoder};
    PropertyDecoder propertyDecoder{integerDecoder, stringDecoder};
    GeometryDecoder geometryDecoder;
    std::unique_ptr<GeometryFactory> geometryFactory;
};

Layer Decoder::Impl::parseBasicMVTEquivalent(BufferStream& tileData) {
    using metadata::stream::StreamMetadata;
    using metadata::type_map::Tag0x01;

    const auto layerMetadata = mlt::metadata::tileset::decodeFeatureTable(tileData);
    const auto layerExtent = decodeVarint<std::uint32_t>(tileData);

    std::vector<Feature::id_t> ids;
    std::unique_ptr<geometry::GeometryVector> geometryVector;
    PropertyVecMap properties;

    for (const auto& columnMetadata : layerMetadata.columns) {
        const auto numStreams = Tag0x01::hasStreamCount(columnMetadata) ? decodeVarint<std::uint32_t>(tileData) : 1;
        if (columnMetadata.isID()) {
            if (columnMetadata.nullable) {
                const auto presentStreamMetadata = StreamMetadata::decode(tileData);
                // data ignored, don't decode it
                tileData.consume(presentStreamMetadata->getByteLength());
            }

            const auto idDataStreamMetadata = StreamMetadata::decode(tileData);

            ids.resize(idDataStreamMetadata->getNumValues());
            if (columnMetadata.getScalarType().hasLongID) {
                integerDecoder.decodeIntStream<std::uint64_t>(tileData, ids, *idDataStreamMetadata);
            } else {
                integerDecoder.decodeIntStream<std::uint32_t, std::uint64_t>(tileData, ids, *idDataStreamMetadata);
            }
        } else if (columnMetadata.isGeometry()) {
            geometryVector = geometryDecoder.decodeGeometryColumn(tileData, columnMetadata, numStreams);
        } else {
            auto property = propertyDecoder.decodePropertyColumn(tileData, columnMetadata, numStreams);
            if (property) {
                properties.emplace(columnMetadata.name, std::move(*property));
            }
        }
    }

    return {layerMetadata.name,
            layerExtent,
            std::move(geometryVector),
            makeFeatures(ids, geometryVector->getGeometries(*geometryFactory)),
            std::move(properties)};
}

std::vector<Feature> Decoder::Impl::makeFeatures(const std::vector<Feature::id_t>& ids,
                                                 std::vector<std::unique_ptr<Geometry>>&& geometries) {
    const auto featureCount = ids.size();
    if (geometries.size() < featureCount) {
        throw std::runtime_error("Invalid geometry count");
    }

    return util::generateVector<Feature>(featureCount, [&](const auto i) {
        return Feature{ids[i], std::move(geometries[i]), static_cast<std::uint32_t>(i)};
    });
}

Decoder::Decoder()
    : Decoder(std::make_unique<GeometryFactory>()) {}

Decoder::Decoder(std::unique_ptr<GeometryFactory>&& geometryFactory)
    : impl{std::make_unique<Impl>(std::move(geometryFactory))} {}

Decoder::~Decoder() noexcept = default;

MapLibreTile Decoder::decode(BufferStream tileData) {
    std::vector<Layer> layers;
    while (tileData.available()) {
        const auto layerLength = decodeVarint<std::uint32_t>(tileData);
        const auto layerTag = decodeVarint<std::uint32_t>(tileData);
        if (layerTag == 1) {
            layers.push_back(impl->parseBasicMVTEquivalent(tileData));
        } else {
            // Not handled, skip the remaining bytes
            tileData.consume(layerLength - getVarintSize(layerTag));
            continue;
        }
    }
    return {std::move(layers)};
}

MapLibreTile Decoder::decode(DataView tileData) {
    return decode(BufferStream{tileData});
}

} // namespace mlt
