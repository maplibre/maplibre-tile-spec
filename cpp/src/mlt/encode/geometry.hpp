#pragma once

#include <mlt/encode/int.hpp>
#include <mlt/metadata/stream.hpp>
#include <mlt/metadata/tileset.hpp>
#include <mlt/util/encoding/varint.hpp>
#include <mlt/util/encoding/zigzag.hpp>
#include <mlt/util/hilbert_curve.hpp>
#include <mlt/util/morton_curve.hpp>

#include <algorithm>
#include <cstdint>
#include <limits>
#include <map>
#include <optional>
#include <set>
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
        std::int32_t maxVertexValue;
    };

    struct Vertex {
        std::int32_t x;
        std::int32_t y;
        bool operator==(const Vertex&) const = default;
    };

    static EncodedGeometryColumn encodeGeometryColumn(
        std::span<const GeometryType> geometryTypes,
        std::span<const std::uint32_t> numGeometries,
        std::span<const std::uint32_t> numParts,
        std::span<const std::uint32_t> numRings,
        std::span<const Vertex> vertexBuffer,
        PhysicalLevelTechnique physicalTechnique,
        IntegerEncoder& intEncoder,
        bool useMortonEncoding = true) {
        using namespace metadata::stream;

        std::vector<std::int32_t> geomTypeValues(geometryTypes.size());
        std::transform(geometryTypes.begin(), geometryTypes.end(), geomTypeValues.begin(),
                       [](auto t) { return static_cast<std::int32_t>(t); });

        auto encodedGeomTypes = intEncoder.encodeIntStream(
            geomTypeValues, physicalTechnique, false,
            PhysicalStreamType::LENGTH, std::nullopt);

        std::vector<std::uint8_t> topologyStreams;
        topologyStreams.insert(topologyStreams.end(), encodedGeomTypes.begin(), encodedGeomTypes.end());
        std::uint32_t numStreams = 1;

        if (!numGeometries.empty()) {
            auto data = encodeUint32AsInt32Stream(numGeometries, physicalTechnique, intEncoder,
                                                   PhysicalStreamType::LENGTH,
                                                   LogicalStreamType{LengthType::GEOMETRIES});
            topologyStreams.insert(topologyStreams.end(), data.begin(), data.end());
            ++numStreams;
        }

        if (!numParts.empty()) {
            auto data = encodeUint32AsInt32Stream(numParts, physicalTechnique, intEncoder,
                                                   PhysicalStreamType::LENGTH,
                                                   LogicalStreamType{LengthType::PARTS});
            topologyStreams.insert(topologyStreams.end(), data.begin(), data.end());
            ++numStreams;
        }

        if (!numRings.empty()) {
            auto data = encodeUint32AsInt32Stream(numRings, physicalTechnique, intEncoder,
                                                   PhysicalStreamType::LENGTH,
                                                   LogicalStreamType{LengthType::RINGS});
            topologyStreams.insert(topologyStreams.end(), data.begin(), data.end());
            ++numStreams;
        }

        auto minVal = std::numeric_limits<std::int32_t>::max();
        auto maxVal = std::numeric_limits<std::int32_t>::min();
        for (const auto& v : vertexBuffer) {
            minVal = std::min({minVal, v.x, v.y});
            maxVal = std::max({maxVal, v.x, v.y});
        }

        auto plainEncoded = encodeVertexBufferPlain(vertexBuffer, physicalTechnique);

        auto hilbertDict = buildHilbertDictionary(vertexBuffer, minVal, maxVal);
        auto hilbertEncoded = encodeHilbertDictionary(
            hilbertDict, vertexBuffer, minVal, maxVal, physicalTechnique, intEncoder);

        std::optional<std::vector<std::uint8_t>> mortonEncoded;
        if (useMortonEncoding) {
            auto mortonDict = buildMortonDictionary(vertexBuffer, minVal, maxVal);
            mortonEncoded = encodeMortonDictionary(
                mortonDict, vertexBuffer, minVal, maxVal, physicalTechnique, intEncoder);
        }

        auto plainSize = plainEncoded.size();
        auto hilbertSize = hilbertEncoded.size();
        auto mortonSize = mortonEncoded ? mortonEncoded->size() : std::numeric_limits<std::size_t>::max();

        std::vector<std::uint8_t> result = std::move(topologyStreams);

        if (plainSize <= hilbertSize && plainSize <= mortonSize) {
            result.insert(result.end(), plainEncoded.begin(), plainEncoded.end());
            return {numStreams + 1, std::move(result), maxVal};
        } else if (hilbertSize <= mortonSize) {
            result.insert(result.end(), hilbertEncoded.begin(), hilbertEncoded.end());
            return {numStreams + 2, std::move(result), maxVal};
        } else {
            result.insert(result.end(), mortonEncoded->begin(), mortonEncoded->end());
            return {numStreams + 2, std::move(result), maxVal};
        }
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

    static std::vector<std::int32_t> zigZagDeltaEncode(std::span<const Vertex> vertices) {
        std::vector<std::int32_t> result(vertices.size() * 2);
        Vertex prev{0, 0};
        for (std::size_t i = 0; i < vertices.size(); ++i) {
            result[i * 2] = static_cast<std::int32_t>(
                util::encoding::encodeZigZag(vertices[i].x - prev.x));
            result[i * 2 + 1] = static_cast<std::int32_t>(
                util::encoding::encodeZigZag(vertices[i].y - prev.y));
            prev = vertices[i];
        }
        return result;
    }

    static std::vector<std::uint8_t> encodeVertexBufferPlain(
        std::span<const Vertex> vertices,
        PhysicalLevelTechnique physicalTechnique) {
        using namespace metadata::stream;

        auto zigZagDelta = zigZagDeltaEncode(vertices);

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

    struct HilbertDictionary {
        std::vector<Vertex> vertices;
        std::vector<std::uint32_t> hilbertIds;
    };

    static HilbertDictionary buildHilbertDictionary(
        std::span<const Vertex> vertexBuffer,
        std::int32_t minVal, std::int32_t maxVal) {
        util::HilbertCurve curve(minVal, maxVal);
        std::map<std::uint32_t, Vertex> dict;
        for (const auto& v : vertexBuffer) {
            auto id = curve.encode({static_cast<float>(v.x), static_cast<float>(v.y)});
            dict.emplace(id, v);
        }
        HilbertDictionary result;
        result.vertices.reserve(dict.size());
        result.hilbertIds.reserve(dict.size());
        for (const auto& [id, v] : dict) {
            result.hilbertIds.push_back(id);
            result.vertices.push_back(v);
        }
        return result;
    }

    static std::vector<std::uint8_t> encodeHilbertDictionary(
        const HilbertDictionary& dict,
        std::span<const Vertex> vertexBuffer,
        std::int32_t minVal, std::int32_t maxVal,
        PhysicalLevelTechnique physicalTechnique,
        IntegerEncoder& intEncoder) {
        using namespace metadata::stream;

        util::HilbertCurve curve(minVal, maxVal);
        auto offsets = computeOffsets(vertexBuffer, dict.hilbertIds, curve);
        auto encodedOffsets = intEncoder.encodeIntStream(
            offsets, physicalTechnique, false,
            PhysicalStreamType::OFFSET, LogicalStreamType{OffsetType::VERTEX});

        auto zigZagDelta = zigZagDeltaEncode(dict.vertices);
        auto encodedDict = encodeVertexBufferRaw(zigZagDelta, physicalTechnique);

        std::vector<std::uint8_t> result;
        result.reserve(encodedOffsets.size() + encodedDict.size());
        result.insert(result.end(), encodedOffsets.begin(), encodedOffsets.end());
        result.insert(result.end(), encodedDict.begin(), encodedDict.end());
        return result;
    }

    struct MortonDictionary {
        std::vector<std::uint32_t> mortonCodes;
    };

    static MortonDictionary buildMortonDictionary(
        std::span<const Vertex> vertexBuffer,
        std::int32_t minVal, std::int32_t maxVal) {
        util::MortonCurve curve(minVal, maxVal);
        std::set<std::uint32_t> codes;
        for (const auto& v : vertexBuffer) {
            codes.insert(curve.encode({static_cast<float>(v.x), static_cast<float>(v.y)}));
        }
        return {{codes.begin(), codes.end()}};
    }

    static std::vector<std::uint8_t> encodeMortonDictionary(
        const MortonDictionary& dict,
        std::span<const Vertex> vertexBuffer,
        std::int32_t minVal, std::int32_t maxVal,
        PhysicalLevelTechnique physicalTechnique,
        IntegerEncoder& intEncoder) {
        using namespace metadata::stream;

        util::MortonCurve curve(minVal, maxVal);
        auto offsets = computeOffsets(vertexBuffer, dict.mortonCodes, curve);
        auto encodedOffsets = intEncoder.encodeIntStream(
            offsets, physicalTechnique, false,
            PhysicalStreamType::OFFSET, LogicalStreamType{OffsetType::VERTEX});

        auto encodedDict = encodeMortonCodes(
            dict.mortonCodes, curve.getNumBits(), curve.getCoordinateShift(),
            physicalTechnique, intEncoder);

        std::vector<std::uint8_t> result;
        result.reserve(encodedOffsets.size() + encodedDict.size());
        result.insert(result.end(), encodedOffsets.begin(), encodedOffsets.end());
        result.insert(result.end(), encodedDict.begin(), encodedDict.end());
        return result;
    }

    static std::vector<std::int32_t> computeOffsets(
        std::span<const Vertex> vertexBuffer,
        std::span<const std::uint32_t> sortedIds,
        const util::SpaceFillingCurve& curve) {
        std::vector<std::int32_t> offsets;
        offsets.reserve(vertexBuffer.size());
        for (const auto& v : vertexBuffer) {
            auto id = curve.encode({static_cast<float>(v.x), static_cast<float>(v.y)});
            auto it = std::lower_bound(sortedIds.begin(), sortedIds.end(), id);
            offsets.push_back(static_cast<std::int32_t>(it - sortedIds.begin()));
        }
        return offsets;
    }

    static std::vector<std::uint8_t> encodeVertexBufferRaw(
        std::span<const std::int32_t> zigZagDelta,
        PhysicalLevelTechnique physicalTechnique) {
        using namespace metadata::stream;

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

    static std::vector<std::uint8_t> encodeMortonCodes(
        std::span<const std::uint32_t> mortonCodes,
        std::uint32_t numBits, std::int32_t coordinateShift,
        PhysicalLevelTechnique physicalTechnique,
        IntegerEncoder& intEncoder) {
        using namespace metadata::stream;

        std::vector<std::int32_t> deltaValues(mortonCodes.size());
        std::int32_t prev = 0;
        for (std::size_t i = 0; i < mortonCodes.size(); ++i) {
            deltaValues[i] = static_cast<std::int32_t>(mortonCodes[i]) - prev;
            prev = static_cast<std::int32_t>(mortonCodes[i]);
        }

        std::vector<std::uint8_t> encodedData;
        encodedData.reserve(deltaValues.size() * 2);
        for (auto v : deltaValues) {
            util::encoding::encodeVarint(static_cast<std::uint32_t>(v), encodedData);
        }

        auto metadata = MortonEncodedStreamMetadata(
                            PhysicalStreamType::DATA,
                            LogicalStreamType{DictionaryType::MORTON},
                            LogicalLevelTechnique::MORTON,
                            LogicalLevelTechnique::DELTA,
                            physicalTechnique,
                            static_cast<std::uint32_t>(mortonCodes.size()),
                            static_cast<std::uint32_t>(encodedData.size()),
                            static_cast<int>(numBits),
                            static_cast<int>(coordinateShift))
                            .encode();

        std::vector<std::uint8_t> result;
        result.reserve(metadata.size() + encodedData.size());
        result.insert(result.end(), metadata.begin(), metadata.end());
        result.insert(result.end(), encodedData.begin(), encodedData.end());
        return result;
    }
};

} // namespace mlt::encoder
