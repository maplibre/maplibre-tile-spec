#pragma once

#include <mlt/encode/int.hpp>
#include <mlt/metadata/stream.hpp>
#include <mlt/metadata/tileset.hpp>
#include <mlt/util/encoding/zigzag.hpp>

#include <algorithm>
#include <cstdint>
#include <span>
#include <vector>

namespace mlt::encoder {

class GeometryEncoder {
public:
    using GeometryType = metadata::tileset::GeometryType;
    using PhysicalLevelTechnique = metadata::stream::PhysicalLevelTechnique;
    using PhysicalStreamType = metadata::stream::PhysicalStreamType;
    using LogicalStreamType = metadata::stream::LogicalStreamType;

    struct EncodedGeometryColumn {
        std::uint32_t numStreams;
        std::vector<std::uint8_t> encodedValues;
    };

    struct Vertex {
        std::int32_t x;
        std::int32_t y;
    };

    static EncodedGeometryColumn encodeGeometryColumn(
        std::span<const GeometryType> geometryTypes,
        std::span<const std::uint32_t> numGeometries,
        std::span<const std::uint32_t> numParts,
        std::span<const std::uint32_t> numRings,
        std::span<const Vertex> vertexBuffer,
        PhysicalLevelTechnique physicalTechnique,
        IntegerEncoder& intEncoder) {
        using namespace metadata::stream;

        std::vector<std::int32_t> geomTypeValues(geometryTypes.size());
        std::transform(geometryTypes.begin(), geometryTypes.end(), geomTypeValues.begin(),
                       [](auto t) { return static_cast<std::int32_t>(t); });

        auto encodedGeomTypes = intEncoder.encodeIntStream(
            geomTypeValues, physicalTechnique, false,
            PhysicalStreamType::LENGTH, std::nullopt);

        std::vector<std::uint8_t> result;
        result.insert(result.end(), encodedGeomTypes.begin(), encodedGeomTypes.end());
        std::uint32_t numStreams = 1;

        if (!numGeometries.empty()) {
            auto data = encodeUint32AsInt32Stream(numGeometries, physicalTechnique, intEncoder,
                                                   PhysicalStreamType::LENGTH,
                                                   LogicalStreamType{LengthType::GEOMETRIES});
            result.insert(result.end(), data.begin(), data.end());
            ++numStreams;
        }

        if (!numParts.empty()) {
            auto data = encodeUint32AsInt32Stream(numParts, physicalTechnique, intEncoder,
                                                   PhysicalStreamType::LENGTH,
                                                   LogicalStreamType{LengthType::PARTS});
            result.insert(result.end(), data.begin(), data.end());
            ++numStreams;
        }

        if (!numRings.empty()) {
            auto data = encodeUint32AsInt32Stream(numRings, physicalTechnique, intEncoder,
                                                   PhysicalStreamType::LENGTH,
                                                   LogicalStreamType{LengthType::RINGS});
            result.insert(result.end(), data.begin(), data.end());
            ++numStreams;
        }

        auto encodedVertices = encodeVertexBuffer(vertexBuffer, physicalTechnique, intEncoder);
        result.insert(result.end(), encodedVertices.begin(), encodedVertices.end());
        ++numStreams;

        return {numStreams, std::move(result)};
    }

private:
    static std::vector<std::uint8_t> encodeUint32AsInt32Stream(
        std::span<const std::uint32_t> values,
        PhysicalLevelTechnique physicalTechnique,
        IntegerEncoder& intEncoder,
        PhysicalStreamType streamType,
        std::optional<LogicalStreamType> logicalType) {
        std::vector<std::int32_t> signedValues(values.size());
        std::transform(values.begin(), values.end(), signedValues.begin(),
                       [](auto v) { return static_cast<std::int32_t>(v); });
        return intEncoder.encodeIntStream(signedValues, physicalTechnique, false,
                                          streamType, std::move(logicalType));
    }

    /// Componentwise delta + zigzag is applied here, then varint-encoded directly.
    /// No additional logical encoding (delta/RLE) is layered on top.
    static std::vector<std::uint8_t> encodeVertexBuffer(
        std::span<const Vertex> vertices,
        PhysicalLevelTechnique physicalTechnique,
        [[maybe_unused]] IntegerEncoder& intEncoder) {
        using namespace metadata::stream;

        std::vector<std::int32_t> zigZagDelta(vertices.size() * 2);
        Vertex prev{0, 0};
        for (std::size_t i = 0; i < vertices.size(); ++i) {
            zigZagDelta[i * 2] = static_cast<std::int32_t>(
                util::encoding::encodeZigZag(vertices[i].x - prev.x));
            zigZagDelta[i * 2 + 1] = static_cast<std::int32_t>(
                util::encoding::encodeZigZag(vertices[i].y - prev.y));
            prev = vertices[i];
        }

        std::vector<std::uint8_t> encodedData;
        encodedData.reserve(zigZagDelta.size() * 2);
        for (auto v : zigZagDelta) {
            util::encoding::encodeVarint(static_cast<std::uint32_t>(v), encodedData);
        }

        auto metadata = StreamMetadata(
                            PhysicalStreamType::DATA,
                            LogicalStreamType{DictionaryType::VERTEX},
                            LogicalLevelTechnique::COMPONENTWISE_DELTA,
                            LogicalLevelTechnique::NONE,
                            physicalTechnique,
                            static_cast<std::uint32_t>(zigZagDelta.size()),
                            static_cast<std::uint32_t>(encodedData.size()))
                            .encode();

        std::vector<std::uint8_t> result;
        result.reserve(metadata.size() + encodedData.size());
        result.insert(result.end(), metadata.begin(), metadata.end());
        result.insert(result.end(), encodedData.begin(), encodedData.end());
        return result;
    }
};

} // namespace mlt::encoder
