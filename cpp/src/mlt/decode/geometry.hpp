#pragma once

#include <mlt/decode/int.hpp>
#include <mlt/decode/int_template.hpp>
#include <mlt/feature.hpp>
#include <mlt/geometry.hpp>
#include <mlt/geometry_vector.hpp>
#include <mlt/metadata/stream.hpp>
#include <mlt/metadata/tileset.hpp>
#include <mlt/util/buffer_stream.hpp>
#include <mlt/util/memory_budget.hpp>
#include <mlt/util/noncopyable.hpp>
#include <mlt/util/vectorized.hpp>

#include <cassert>
#include <limits>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace mlt::decoder {

class GeometryDecoder {
public:
    using GeometryVector = geometry::GeometryVector;

    GeometryDecoder(IntegerDecoder& intDecoder)
        : intDecoder(intDecoder) {}

private:
    enum class VectorType : std::uint8_t {
        FLAT,
        CONST,
        SEQUENCE,
        DICTIONARY,
        FSST_DICTIONARY,
    };

    static VectorType getVectorTypeIntStream(const metadata::stream::StreamMetadata& streamMetadata) {
        using namespace metadata::stream;
        const auto logicalLevelTechnique1 = streamMetadata.getLogicalLevelTechnique1();
        const auto logicalLevelTechnique2 = streamMetadata.getLogicalLevelTechnique2();
        const auto metadataType = streamMetadata.getMetadataType();
        const auto rleRuns = (metadataType == LogicalLevelTechnique::RLE)
                                 ? static_cast<const RleEncodedStreamMetadata&>(streamMetadata).getRuns()
                                 : 0;

        if (logicalLevelTechnique1 == LogicalLevelTechnique::RLE) {
            return (rleRuns == 1) ? VectorType::CONST : VectorType::FLAT;
        }

        if (logicalLevelTechnique1 == LogicalLevelTechnique::DELTA &&
            logicalLevelTechnique2 == LogicalLevelTechnique::RLE) {
            // If base value equals delta value then one run else two runs
            if (rleRuns == 1 || rleRuns == 2) {
                return VectorType::SEQUENCE;
            }
        }

        return (streamMetadata.getNumValues() == 1) ? VectorType::CONST : VectorType::FLAT;
    }

public:
    std::unique_ptr<GeometryVector> decodeGeometryColumn(BufferStream& tileData,
                                                         const metadata::tileset::Column& column,
                                                         std::uint32_t numStreams) {
        using namespace util::decoding;
        using namespace metadata::stream;
        using namespace metadata::tileset;

        std::vector<GeometryType> geometryTypes;
        std::vector<std::uint32_t> geometryOffsets;
        std::vector<std::uint32_t> partOffsets;
        std::vector<std::uint32_t> ringOffsets;
        std::vector<std::uint32_t> vertexOffsets;
        std::vector<std::uint32_t> indexBuffer;
        std::vector<std::uint32_t> triangles;
        std::vector<std::int32_t> vertices;

        const auto geomTypeMetadata = StreamMetadata::decode(tileData);
        if (!geomTypeMetadata) {
            throw std::runtime_error("geometry column missing metadata stream: " + column.name);
        }

        [[maybe_unused]] const auto geometryTypesVectorType = getVectorTypeIntStream(*geomTypeMetadata);

        // If all geometries in the column have the same geometry type, we could decode them
        // somewhat more efficiently, and return the geometry in a more GPU-friendly form.
        //
        // FIXME: Before enabling this, fix decodeConstIntStream: it does not check logicalLevelTechnique1,
        // so for a DELTA-encoded single-value stream it returns the raw ZigZag value instead of
        // decoding it (e.g. geometry type 4 encodes as ZigZag delta 8, not 4). The TS decoder
        // received the equivalent fix in decodeUnsignedConstInt32Stream.
        // if (geometryTypesVectorType == VectorType::CONST) {
        //     const auto geomType = intDecoder.decodeConstIntStream<std::uint32_t, std::uint32_t, GeometryType>(
        //         tileData, *geomTypeMetadata);
        //     ...
        // }

        // Different geometry types are mixed in the geometry column
        intDecoder.decodeIntStream<std::uint32_t, std::uint32_t, GeometryType>(
            tileData, geometryTypes, *geomTypeMetadata);

        const std::optional<geometry::MortonSettings> mortonSettings;

        for (std::uint32_t i = 1; i < numStreams; ++i) {
            const auto geomStreamMetadata = StreamMetadata::decode(tileData);

            switch (geomStreamMetadata->getPhysicalStreamType()) {
                case PhysicalStreamType::LENGTH: {
                    if (!geomStreamMetadata->getLogicalStreamType() ||
                        !geomStreamMetadata->getLogicalStreamType()->getLengthType()) {
                        throw std::runtime_error("Length stream missing logical type: " + column.name);
                    }
                    const auto type = *geomStreamMetadata->getLogicalStreamType()->getLengthType();
                    std::optional<std::reference_wrapper<std::vector<std::uint32_t>>> target;
                    switch (type) {
                        case LengthType::GEOMETRIES:
                            target = geometryOffsets;
                            break;
                        case LengthType::PARTS:
                            target = partOffsets;
                            break;
                        case LengthType::RINGS:
                            target = ringOffsets;
                            break;
                        case LengthType::TRIANGLES:
                            target = triangles;
                            break;
                        default:
                            throw std::runtime_error("Length stream type '" + std::to_string(std::to_underlying(type)) +
                                                     " not implemented: " + column.name);
                    }
                    intDecoder.decodeIntStream<std::uint32_t, std::uint32_t, std::uint32_t>(
                        tileData, target->get(), *geomStreamMetadata);
                    break;
                }
                case PhysicalStreamType::OFFSET: {
                    if (!geomStreamMetadata->getLogicalStreamType() ||
                        !geomStreamMetadata->getLogicalStreamType()->getOffsetType()) {
                        throw std::runtime_error("Offset stream missing type: " + column.name);
                    }
                    const auto type = *geomStreamMetadata->getLogicalStreamType()->getOffsetType();
                    std::optional<std::reference_wrapper<std::vector<std::uint32_t>>> target;
                    switch (type) {
                        case OffsetType::VERTEX:
                            target = vertexOffsets;
                            break;
                        case OffsetType::INDEX:
                            target = indexBuffer;
                            break;
                        default:
                            throw std::runtime_error("Offset stream type '" + std::to_string(std::to_underlying(type)) +
                                                     " not implemented: " + column.name);
                    }
                    intDecoder.decodeIntStream<std::uint32_t>(tileData, target->get(), *geomStreamMetadata);
                    break;
                }
                case PhysicalStreamType::DATA: {
                    if (!geomStreamMetadata->getLogicalStreamType() ||
                        !geomStreamMetadata->getLogicalStreamType()->getDictionaryType()) {
                        throw std::runtime_error("Data stream missing dictionary type: " + column.name);
                    }
                    if (mortonSettings) {
                        throw std::runtime_error("multiple data streams");
                    }
                    const auto type = *geomStreamMetadata->getLogicalStreamType()->getDictionaryType();
                    switch (type) {
                        case DictionaryType::VERTEX:
                            switch (geomStreamMetadata->getPhysicalLevelTechnique()) {
                                case PhysicalLevelTechnique::NONE:
                                case PhysicalLevelTechnique::VARINT:
                                case PhysicalLevelTechnique::FAST_PFOR:
                                    if (geomStreamMetadata->getLogicalLevelTechnique1() ==
                                        LogicalLevelTechnique::COMPONENTWISE_DELTA) {
                                        intDecoder.decodeIntStream<std::uint32_t, std::uint32_t, std::int32_t>(
                                            tileData, vertices, *geomStreamMetadata, /*isSigned=*/true);
                                    } else {
                                        intDecoder.decodeIntStream<std::uint32_t, std::uint32_t, std::int32_t>(
                                            tileData, vertices, *geomStreamMetadata, /*isSigned=*/false);
                                        if (vertices.size() % 2 != 0) {
                                            throw std::runtime_error("Vertex buffer has an odd number of values: " +
                                                                     column.name);
                                        }
                                        util::decoding::vectorized::decodeComponentwiseDeltaVec2(vertices.data(),
                                                                                                 vertices.size());
                                    }
                                    break;
                                default:
                                    throw std::runtime_error("Unsupported encoding " +
                                                             std::to_string(std::to_underlying(
                                                                 geomStreamMetadata->getPhysicalLevelTechnique())) +
                                                             " for geometries: " + column.name);
                            };
                            break;
                        case DictionaryType::MORTON: {
                            if (geomStreamMetadata->getMetadataType() != LogicalLevelTechnique::MORTON) {
                                throw std::runtime_error("Morton vertex stream without Morton metadata: " +
                                                         column.name);
                            }
                            const auto& mortonStreamMetadata = static_cast<const MortonEncodedStreamMetadata&>(
                                *geomStreamMetadata);
                            // TODO: This results in double-morton-decoding, need to align with TypeScript
                            // implementation. mortonSettings = geometry::MortonSettings{
                            //    .numBits = mortonStreamMetadata.getNumBits(),
                            //    .coordinateShift = mortonStreamMetadata.getCoordinateShift()};
                            intDecoder.decodeMortonStream<std::uint32_t, std::int32_t>(
                                tileData, vertices, mortonStreamMetadata);
                            break;
                        }
                        default:
                            throw std::runtime_error("Dictionary type '" + std::to_string(std::to_underlying(type)) +
                                                     "' not implemented: " + column.name);
                    }
                    break;
                }
                case PhysicalStreamType::PRESENT:
                    break;
                default:
                    throw std::runtime_error("Unsupported logical stream type: " + column.name);
            }
        }

        if (!indexBuffer.empty() && partOffsets.empty()) {
            /* Case when the indices of a Polygon outline are not encoded in the data so no
             *  topology data are present in the tile */
            return std::make_unique<geometry::FlatGpuVector>(
                std::move(geometryTypes), std::move(triangles), std::move(indexBuffer), std::move(vertices));
        }

        if (!geometryOffsets.empty()) {
            auto geometryOffsetsCopy = geometryOffsets; // TODO: avoid copies
            auto& budget = intDecoder.getBudget();
            decodeRootLengthStream(geometryTypes,
                                   geometryOffsetsCopy,
                                   /*bufferId=*/GeometryType::POLYGON,
                                   geometryOffsets);
            if (!partOffsets.empty()) {
                if (!ringOffsets.empty()) {
                    auto partOffsetsCopy = partOffsets;
                    decodeLevel1LengthStream(geometryTypes,
                                             geometryOffsets,
                                             partOffsetsCopy,
                                             /*isLineStringPresent=*/false,
                                             partOffsets,
                                             budget);
                    auto ringOffsetsCopy = ringOffsets;
                    decodeLevel2LengthStream(
                        geometryTypes, geometryOffsets, partOffsets, ringOffsetsCopy, ringOffsets, budget);
                } else {
                    auto partOffsetsCopy = partOffsets;
                    decodeLevel1WithoutRingBufferLengthStream(
                        geometryTypes, geometryOffsets, partOffsetsCopy, partOffsets, budget);
                }
            }
        } else if (!partOffsets.empty()) {
            auto partOffsetsCopy = partOffsets;
            if (!ringOffsets.empty()) {
                auto ringOffsetsCopy = ringOffsets;
                decodeRootLengthStream(geometryTypes, partOffsetsCopy, GeometryType::LINESTRING, partOffsets);
                decodeLevel1LengthStream(geometryTypes,
                                         partOffsets,
                                         ringOffsetsCopy,
                                         /*isLineStringPresent=*/true,
                                         ringOffsets,
                                         intDecoder.getBudget());
            } else {
                decodeRootLengthStream(geometryTypes, partOffsetsCopy, GeometryType::POINT, partOffsets);
            }
        }

        if (!indexBuffer.empty()) {
            /* Case when the indices of a Polygon outline are encoded in the tile */
            return std::make_unique<geometry::FlatGpuVector>(
                std::move(geometryTypes),
                std::move(triangles),
                std::move(indexBuffer),
                std::move(vertices),
                geometry::TopologyVector(std::move(geometryOffsets), std::move(partOffsets), std::move(ringOffsets)));
        }

        return std::make_unique<geometry::FlatGeometryVector>(
            std::move(geometryTypes),
            geometry::TopologyVector(std::move(geometryOffsets), std::move(partOffsets), std::move(ringOffsets)),
            std::move(vertexOffsets),
            std::move(vertices),
            mortonSettings ? geometry::VertexBufferType::MORTON : geometry::VertexBufferType::VEC_2,
            mortonSettings);
    }

    /*
     * Handle the parsing of the different topology length buffers separate not generic to reduce the
     * branching and improve the performance
     *
     * The lengths and the geometry types come from the tile, so they may not be consistent with each other.
     */
private:
    static std::uint32_t addOffset(const std::uint32_t offset, const std::uint32_t length) {
        if (length > std::numeric_limits<std::uint32_t>::max() - offset) {
            throw std::runtime_error("Geometry offsets overflow");
        }
        return offset + length;
    }

    /// The number of items between two offsets, which must be there and not decrease
    static std::uint32_t delta(const std::vector<std::uint32_t>& offsets, const std::size_t index) {
        if (index == 0 || index >= offsets.size() || offsets[index] < offsets[index - 1]) {
            throw std::runtime_error("Invalid geometry offsets");
        }
        return offsets[index] - offsets[index - 1];
    }

    /// The next length from a stream
    static std::uint32_t nextLength(const std::vector<std::uint32_t>& lengths, std::size_t& counter) {
        if (counter >= lengths.size()) {
            throw std::runtime_error("Geometry length stream underflow");
        }
        return lengths[counter++];
    }

    /// Store the next offset
    static void putOffset(std::vector<std::uint32_t>& offsets, std::size_t& counter, const std::uint32_t value) {
        if (counter >= offsets.size()) {
            throw std::runtime_error("Geometry offsets overflow");
        }
        offsets[counter++] = value;
    }

    /// Prepare the offsets buffer for the offsets of each of the `rootOffsets`, which also come from the tile
    static void prepareOffsets(std::vector<std::uint32_t>& offsets,
                               const std::vector<std::uint32_t>& rootOffsets,
                               const std::size_t numGeometries,
                               util::MemoryBudget& budget) {
        if (rootOffsets.size() != numGeometries + 1) {
            throw std::runtime_error("Invalid geometry offsets");
        }
        const auto size = std::size_t{rootOffsets.back()} + 1;
        budget.consumeItems<std::uint32_t>(size);
        offsets.assign(size, 0);
    }

public:
    static void decodeRootLengthStream(const std::vector<metadata::tileset::GeometryType>& geometryTypes,
                                       const std::vector<std::uint32_t>& rootLengthStream,
                                       const metadata::tileset::GeometryType bufferId,
                                       std::vector<std::uint32_t>& rootBufferOffsets) {
        assert(&rootLengthStream != &rootBufferOffsets);
        rootBufferOffsets.assign(geometryTypes.size() + 1, 0);
        std::uint32_t previousOffset = 0;
        std::size_t rootLengthCounter = 0;
        for (std::size_t i = 0; i < geometryTypes.size(); ++i) {
            /* Test if the geometry has and entry in the root buffer
             * BufferId: 2 GeometryOffsets -> MultiPolygon, MultiLineString, MultiPoint
             * BufferId: 1 PartOffsets -> Polygon
             * BufferId: 0 PartOffsets, RingOffsets -> LineString
             * */
            const auto length = (geometryTypes[i] > bufferId) ? nextLength(rootLengthStream, rootLengthCounter) : 1;
            previousOffset = rootBufferOffsets[i + 1] = addOffset(previousOffset, length);
        }
    }

    static void decodeLevel1LengthStream(const std::vector<metadata::tileset::GeometryType>& geometryTypes,
                                         const std::vector<std::uint32_t>& rootOffsetBuffer,
                                         const std::vector<std::uint32_t>& level1LengthBuffer,
                                         const bool isLineStringPresent,
                                         std::vector<std::uint32_t>& level1BufferOffsets,
                                         util::MemoryBudget& budget) {
        assert(&rootOffsetBuffer != &level1BufferOffsets);
        assert(&level1LengthBuffer != &level1BufferOffsets);
        using metadata::tileset::GeometryType;
        prepareOffsets(level1BufferOffsets, rootOffsetBuffer, geometryTypes.size(), budget);
        std::uint32_t previousOffset = 0;
        std::size_t level1BufferCounter = 1;
        std::size_t level1LengthBufferCounter = 0;
        for (std::size_t i = 0; i < geometryTypes.size(); ++i) {
            const auto geometryType = geometryTypes[i];
            const auto numGeometries = delta(rootOffsetBuffer, i + 1);

            if (geometryType == GeometryType::MULTIPOLYGON || geometryType == GeometryType::POLYGON ||
                (isLineStringPresent &&
                 (geometryType == GeometryType::MULTILINESTRING || geometryType == GeometryType::LINESTRING))) {
                /* For MultiPolygon, Polygon and in some cases for MultiLineString and LineString
                 * a value in the level1LengthBuffer exists */
                for (std::uint32_t j = 0; j < numGeometries; ++j) {
                    previousOffset = addOffset(previousOffset,
                                               nextLength(level1LengthBuffer, level1LengthBufferCounter));
                    putOffset(level1BufferOffsets, level1BufferCounter, previousOffset);
                }
            } else {
                /* For MultiPoint and Point and in some cases for MultiLineString and LineString no value in the
                 * level1LengthBuffer exists */
                for (std::uint32_t j = 0; j < numGeometries; j++) {
                    putOffset(level1BufferOffsets, level1BufferCounter, ++previousOffset);
                }
            }
        }
    }

    /*
     * Case where no ring buffer exists so no MultiPolygon or Polygon geometry is part of the buffer
     */
    static void decodeLevel1WithoutRingBufferLengthStream(
        const std::vector<metadata::tileset::GeometryType>& geometryTypes,
        const std::vector<std::uint32_t>& rootOffsetBuffer,
        const std::vector<std::uint32_t>& level1LengthBuffer,
        std::vector<std::uint32_t>& level1BufferOffsets,
        util::MemoryBudget& budget) {
        assert(&rootOffsetBuffer != &level1BufferOffsets);
        assert(&level1LengthBuffer != &level1BufferOffsets);
        using metadata::tileset::GeometryType;
        prepareOffsets(level1BufferOffsets, rootOffsetBuffer, geometryTypes.size(), budget);
        std::uint32_t previousOffset = 0;
        std::size_t level1OffsetBufferCounter = 1;
        std::size_t level1LengthCounter = 0;
        for (std::size_t i = 0; i < geometryTypes.size(); ++i) {
            const auto geometryType = geometryTypes[i];
            const auto numGeometries = delta(rootOffsetBuffer, i + 1);
            if (geometryType == GeometryType::MULTILINESTRING || geometryType == GeometryType::LINESTRING) {
                /* For MultiLineString and LineString a value in the level1LengthBuffer exists */
                for (std::uint32_t j = 0; j < numGeometries; ++j) {
                    previousOffset = addOffset(previousOffset, nextLength(level1LengthBuffer, level1LengthCounter));
                    putOffset(level1BufferOffsets, level1OffsetBufferCounter, previousOffset);
                }
            } else {
                /* For MultiPoint and Point no value in level1LengthBuffer exists */
                for (std::uint32_t j = 0; j < numGeometries; ++j) {
                    putOffset(level1BufferOffsets, level1OffsetBufferCounter, ++previousOffset);
                }
            }
        }
    }

    static void decodeLevel2LengthStream(const std::vector<metadata::tileset::GeometryType>& geometryTypes,
                                         const std::vector<std::uint32_t>& rootOffsetBuffer,
                                         const std::vector<std::uint32_t>& level1OffsetBuffer,
                                         const std::vector<std::uint32_t>& level2LengthBuffer,
                                         std::vector<std::uint32_t>& level2BufferOffsets,
                                         util::MemoryBudget& budget) {
        assert(&rootOffsetBuffer != &level2BufferOffsets);
        assert(&level1OffsetBuffer != &level2BufferOffsets);
        assert(&level2LengthBuffer != &level2BufferOffsets);
        using metadata::tileset::GeometryType;
        if (level1OffsetBuffer.empty()) {
            throw std::runtime_error("Invalid geometry offsets");
        }
        const auto size = std::size_t{level1OffsetBuffer.back()} + 1;
        budget.consumeItems<std::uint32_t>(size);
        level2BufferOffsets.assign(size, 0);
        std::uint32_t previousOffset = 0;
        std::size_t level1OffsetBufferCounter = 1;
        std::size_t level2OffsetBufferCounter = 1;
        std::size_t level2LengthBufferCounter = 0;
        for (std::size_t i = 0; i < geometryTypes.size(); ++i) {
            const auto geometryType = geometryTypes[i];
            const auto numGeometries = delta(rootOffsetBuffer, i + 1);
            if (geometryType != GeometryType::POINT && geometryType != GeometryType::MULTIPOINT) {
                /* For MultiPolygon, MultiLineString, Polygon and LineString a value in level2LengthBuffer
                 * exists */
                for (std::uint32_t j = 0; j < numGeometries; ++j) {
                    const auto numParts = delta(level1OffsetBuffer, level1OffsetBufferCounter);
                    level1OffsetBufferCounter++;
                    for (std::uint32_t k = 0; k < numParts; ++k) {
                        previousOffset = addOffset(previousOffset,
                                                   nextLength(level2LengthBuffer, level2LengthBufferCounter));
                        putOffset(level2BufferOffsets, level2OffsetBufferCounter, previousOffset);
                    }
                }
            } else {
                /* For MultiPoint and Point no value in level2LengthBuffer exists */
                for (std::uint32_t j = 0; j < numGeometries; j++) {
                    putOffset(level2BufferOffsets, level2OffsetBufferCounter, ++previousOffset);
                    level1OffsetBufferCounter++;
                }
            }
        }
    }

private:
    IntegerDecoder& intDecoder;
};

} // namespace mlt::decoder
