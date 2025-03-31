#pragma once

#include <mlt/decode/int.hpp>
#include <mlt/feature.hpp>
#include <mlt/geometry.hpp>
#include <mlt/metadata/stream.hpp>
#include <mlt/metadata/tileset.hpp>
#include <mlt/util/buffer_stream.hpp>
#include <mlt/util/noncopyable.hpp>

#include <algorithm>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace mlt::geometry {

struct MortonSettings {
    int numBits;
    int coordinateShift;
};

enum class VertexBufferType : std::uint32_t {
    MORTON,
    VEC_2,
    VEC_3
};

struct TopologyVector : public util::noncopyable {
    TopologyVector(std::vector<std::uint32_t>&& geometryOffsets_,
                   std::vector<std::uint32_t>&& partOffsets_,
                   std::vector<std::uint32_t>&& ringOffsets_) noexcept
        : geometryOffsets(geometryOffsets_),
          partOffsets(partOffsets_),
          ringOffsets(ringOffsets_) {}

private:
    std::vector<std::uint32_t> geometryOffsets;
    std::vector<std::uint32_t> partOffsets;
    std::vector<std::uint32_t> ringOffsets;
};

struct GeometryVectorBase : public util::noncopyable {
    using GeometryType = metadata::tileset::GeometryType;

    virtual ~GeometryVectorBase() = default;

    // TODO: generator pattern?
    virtual std::vector<std::unique_ptr<Geometry>> getGeometry() const = 0;

protected:
    GeometryVectorBase(std::vector<std::uint32_t>&& indexBuffer_,
                       std::vector<std::int32_t>&& vertexBuffer_,
                       std::optional<TopologyVector>&& topologyVector_,
                       std::optional<MortonSettings> mortonSettings_ = {}) noexcept
        : indexBuffer(std::move(indexBuffer_)),
          vertexBuffer(std::move(vertexBuffer_)),
          topologyVector(std::move(topologyVector_)),
          mortonSettings(mortonSettings_) {}

private:
    std::vector<std::uint32_t> indexBuffer;
    std::vector<std::int32_t> vertexBuffer;
    std::optional<TopologyVector> topologyVector;
    std::optional<MortonSettings> mortonSettings;
};

struct GpuVector : public GeometryVectorBase {
    GpuVector(std::vector<std::uint32_t>&& triangleOffsets_,
              std::vector<std::uint32_t>&& indexBuffer_,
              std::vector<std::int32_t>&& vertexBuffer_,
              std::optional<TopologyVector>&& topologyVector_) noexcept
        : GeometryVectorBase(std::move(indexBuffer_), std::move(vertexBuffer_), std::move(topologyVector_)),
          triangleOffsets(std::move(triangleOffsets_)) {}

    std::vector<std::unique_ptr<Geometry>> getGeometry() const override { throw std::runtime_error("not implemented"); }

private:
    std::vector<std::uint32_t> triangleOffsets;
};

struct ConstGpuVector : public GpuVector {
    ConstGpuVector(std::uint32_t numGeometries_,
                   GeometryType geometryType_,
                   std::vector<std::uint32_t>&& triangleOffsets_,
                   std::vector<std::uint32_t>&& indexBuffer_,
                   std::vector<std::int32_t>&& vertexBuffer_,
                   std::optional<TopologyVector>&& topologyVector_) noexcept
        : GpuVector(std::move(triangleOffsets_),
                    std::move(indexBuffer_),
                    std::move(vertexBuffer_),
                    std::move(topologyVector_)),
          numGeometries(numGeometries_),
          geometryType(geometryType_) {}

    std::vector<std::unique_ptr<Geometry>> getGeometry() const override { throw std::runtime_error("not implemented"); }

private:
    std::uint32_t numGeometries;
    GeometryType geometryType;
};

struct FlatGpuVector : public GpuVector {
    FlatGpuVector(std::vector<GeometryType>&& geometryTypes_,
                  std::vector<std::uint32_t>&& triangleOffsets_,
                  std::vector<std::uint32_t>&& indexBuffer_,
                  std::vector<std::int32_t>&& vertexBuffer_,
                  std::optional<TopologyVector>&& topologyVector_ = {}) noexcept
        : GpuVector(std::move(triangleOffsets_),
                    std::move(indexBuffer_),
                    std::move(vertexBuffer_),
                    std::move(topologyVector_)),
          geometryTypes(std::move(geometryTypes_)) {}

    std::vector<std::unique_ptr<Geometry>> getGeometry() const override { throw std::runtime_error("not implemented"); }

private:
    std::vector<GeometryType> geometryTypes;
};

struct GeometryVector : public GeometryVectorBase {
    GeometryVector(VertexBufferType vertexBufferType_,
                   TopologyVector&& topologyVector_,
                   std::vector<std::uint32_t>&& vertexOffsets_,
                   std::vector<std::int32_t>&& vertexBuffer_,
                   std::optional<MortonSettings> mortonSettings_) noexcept
        : GeometryVectorBase(/*indexBuffer=*/{}, std::move(vertexBuffer_), std::move(topologyVector_), mortonSettings_),
          vertexBufferType(vertexBufferType_),
          vertexOffsets(std::move(vertexOffsets_)) {}

    std::vector<std::unique_ptr<Geometry>> getGeometry() const override { throw std::runtime_error("not implemented"); }

private:
    VertexBufferType vertexBufferType;
    std::vector<std::uint32_t> vertexOffsets;
};

struct ConstGeometryVector : public GeometryVector {
    ConstGeometryVector(std::uint32_t numGeometries_,
                        GeometryType geometryType_,
                        VertexBufferType vertexBufferType_,
                        TopologyVector&& topologyVector_,
                        std::vector<std::uint32_t>&& vertexOffsets_,
                        std::vector<std::int32_t>&& vertexBuffer_,
                        std::optional<MortonSettings> mortonSettings_) noexcept
        : GeometryVector(vertexBufferType_,
                         std::move(topologyVector_),
                         std::move(vertexOffsets_),
                         std::move(vertexBuffer_),
                         mortonSettings_),
          numGeometries(numGeometries_),
          geometryType(geometryType_) {}

    std::vector<std::unique_ptr<Geometry>> getGeometry() const override { throw std::runtime_error("not implemented"); }

private:
    std::uint32_t numGeometries;
    GeometryType geometryType;
};

struct FlatGeometryVector : public GeometryVector {
    FlatGeometryVector(VertexBufferType vertexBufferType_,
                       std::vector<GeometryType>&& geometryTypes_,
                       TopologyVector&& topologyVector_,
                       std::vector<std::uint32_t>&& vertexOffsets_,
                       std::vector<std::int32_t>&& vertexBuffer_,
                       std::optional<MortonSettings> mortonSettings_) noexcept
        : GeometryVector(vertexBufferType_,
                         std::move(topologyVector_),
                         std::move(vertexOffsets_),
                         std::move(vertexBuffer_),
                         mortonSettings_),
          geometryTypes(geometryTypes_) {}

    std::vector<std::unique_ptr<Geometry>> getGeometry() const override { throw std::runtime_error("not implemented"); }

private:
    std::vector<GeometryType> geometryTypes;
};

struct GeometryColumn : public util::noncopyable {
    GeometryColumn() = default;
    ~GeometryColumn() noexcept = default;

    GeometryColumn(GeometryColumn&&) noexcept = default;
    GeometryColumn& operator=(GeometryColumn&&) = delete;

    std::vector<metadata::tileset::GeometryType> geometryTypes;
    std::vector<std::uint32_t> geometryOffsets;
    std::vector<std::uint32_t> partOffsets;
    std::vector<std::uint32_t> ringOffsets;
    std::vector<std::uint32_t> vertexOffsets;
    std::vector<std::uint32_t> indexBuffer;
    std::vector<std::uint32_t> triangles;
    std::vector<std::int32_t> vertices;

    std::unique_ptr<GeometryVectorBase> geometryVector;
};
} // namespace mlt::geometry

namespace mlt::decoder {

class GeometryDecoder {
public:
    using GeometryColumn = geometry::GeometryColumn;

    GeometryDecoder(std::unique_ptr<GeometryFactory>&& geometryFactory_)
        : geometryFactory(std::move(geometryFactory_)) {
        if (!geometryFactory) {
            throw std::runtime_error("missing geometry factory");
        }
    }

private:
    enum class VectorType : std::uint32_t {
        FLAT,
        CONST,
        SEQUENCE,
        DICTIONARY,
        FSST_DICTIONARY,
    };

    VectorType getVectorTypeIntStream(const metadata::stream::StreamMetadata& streamMetadata) {
        using namespace metadata::stream;
        const auto logicalLevelTechnique1 = streamMetadata.getLogicalLevelTechnique1();
        const auto logicalLevelTechnique2 = streamMetadata.getLogicalLevelTechnique2();
        const auto metadataType = streamMetadata.getMetadataType();
        const auto& rleMetadata = static_cast<const RleEncodedStreamMetadata&>(streamMetadata);
        const auto rleRuns = (metadataType == LogicalLevelTechnique::RLE) ? rleMetadata.getRuns() : 0;

        if (logicalLevelTechnique1 == LogicalLevelTechnique::RLE) {
            assert(metadataType == LogicalLevelTechnique::RLE);
            return (rleRuns == 1) ? VectorType::CONST : VectorType::FLAT;
        } else if (logicalLevelTechnique1 == LogicalLevelTechnique::DELTA &&
                   logicalLevelTechnique2 == LogicalLevelTechnique::RLE) {
            assert(metadataType == LogicalLevelTechnique::RLE);
            // If base value equals delta value then one run else two runs
            if (rleRuns == 1 || rleRuns == 2) {
                return VectorType::SEQUENCE;
            }
        }

        return (streamMetadata.getNumValues() == 1) ? VectorType::CONST : VectorType::FLAT;
    }

public:
    GeometryColumn decodeGeometryColumn(BufferStream& tileData,
                                        const metadata::tileset::Column& column,
                                        std::uint32_t numStreams) {
        using namespace util::decoding;
        using namespace metadata::stream;
        using namespace metadata::tileset;

        GeometryColumn geomColumn;

        const auto geomTypeMetadata = StreamMetadata::decode(tileData);
        if (!geomTypeMetadata) {
            throw std::runtime_error("geometry column missing metadata stream: " + column.name);
        }

        const auto geometryTypesVectorType = getVectorTypeIntStream(*geomTypeMetadata);

        // If all geometries in the colum have the same geometry type, we could decode them
        // somewhat more efficiently, and return the geometry in a more GPU-friendly form.
        // if (geometryTypesVectorType == VectorType::CONST) {
        //     const auto geomType = intDecoder.decodeConstIntStream<std::uint32_t, std::uint32_t, GeometryType>(
        //         tileData, *geomTypeMetadata);
        //     ...
        // }

        // Different geometry types are mixed in the geometry column
        intDecoder.decodeIntStream<std::uint32_t, std::uint32_t, GeometryType>(
            tileData, geomColumn.geometryTypes, *geomTypeMetadata);

        std::optional<geometry::MortonSettings> mortonSettings;

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
                            target = geomColumn.geometryOffsets;
                            break;
                        case LengthType::PARTS:
                            target = geomColumn.partOffsets;
                            break;
                        case LengthType::RINGS:
                            target = geomColumn.ringOffsets;
                            break;
                        case LengthType::TRIANGLES:
                            target = geomColumn.triangles;
                            break;
                        default:
                            throw std::runtime_error("Length stream type '" + std::to_string(std::to_underlying(type)) +
                                                     " not implemented: " + column.name);
                    }
                    intDecoder.decodeIntStream<std::uint32_t, std::uint32_t, std::uint32_t>(
                        tileData, target->get(), *geomStreamMetadata);

                    // TODO: why? (integerStreamDecoder.ts:decodeLengthToOffsetBuffer)
                    if (type == LengthType::TRIANGLES &&
                        geomStreamMetadata->getLogicalLevelTechnique1() == LogicalLevelTechnique::NONE &&
                        geomStreamMetadata->getLogicalLevelTechnique2() == LogicalLevelTechnique::NONE) {
                        target->get().insert(target->get().begin(), 0);
                    }
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
                            target = geomColumn.vertexOffsets;
                            break;
                        case OffsetType::INDEX:
                            target = geomColumn.indexBuffer;
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
                                case PhysicalLevelTechnique::FAST_PFOR:
                                    throw std::runtime_error("FastPfor encoding for geometries is not yet supported.");
                                case PhysicalLevelTechnique::NONE:
                                case PhysicalLevelTechnique::ALP:
                                    // TODO: other implementations are not clear on whether these are valid
                                case PhysicalLevelTechnique::VARINT:
                                    intDecoder
                                        .decodeIntStream<std::uint32_t, std::uint32_t, std::int32_t, /*isSigned=*/true>(
                                            tileData, geomColumn.vertices, *geomStreamMetadata);
                                    break;
                                default:
                                    throw std::runtime_error("Unsupported encoding for geometries: " + column.name);
                            };
                            break;
                        case DictionaryType::MORTON: {
                            assert(geomStreamMetadata->getMetadataType() == LogicalLevelTechnique::MORTON);
                            const auto& mortonStreamMetadata = static_cast<MortonEncodedStreamMetadata&>(
                                *geomStreamMetadata);
                            mortonSettings = geometry::MortonSettings{
                                .numBits = mortonStreamMetadata.getNumBits(),
                                .coordinateShift = mortonStreamMetadata.getCoordinateShift()};
                            intDecoder.decodeMortonStream<std::uint32_t, std::int32_t>(
                                tileData, geomColumn.vertices, mortonStreamMetadata);
                            break;
                        }
                        default:
                        case DictionaryType::NONE:
                        case DictionaryType::SINGLE:
                        case DictionaryType::SHARED:
                        case DictionaryType::FSST:
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

        if (!geomColumn.indexBuffer.empty() && geomColumn.partOffsets.empty()) {
            /* Case when the indices of a Polygon outline are not encoded in the data so no
             *  topology data are present in the tile */
            geomColumn.geometryVector = std::make_unique<geometry::FlatGpuVector>(std::move(geomColumn.geometryTypes),
                                                                                  std::move(geomColumn.triangles),
                                                                                  std::move(geomColumn.indexBuffer),
                                                                                  std::move(geomColumn.vertices));
            return geomColumn;
        }

        if (!geomColumn.geometryOffsets.empty()) {
            auto geometryOffsets = geomColumn.geometryOffsets; // TODO: avoid copies
            decodeRootLengthStream(geomColumn.geometryTypes,
                                   geomColumn.geometryOffsets,
                                   /*bufferId=*/GeometryType::POLYGON,
                                   geometryOffsets);
            if (!geomColumn.partOffsets.empty()) {
                auto partOffsets = geomColumn.partOffsets;
                if (!geomColumn.ringOffsets.empty()) {
                    decodeLevel1LengthStream(geomColumn.geometryTypes,
                                             geomColumn.geometryOffsets,
                                             partOffsets,
                                             /*isLineStringPresent=*/false,
                                             geomColumn.partOffsets);
                    auto ringOffsets = geomColumn.ringOffsets;
                    decodeLevel2LengthStream(geomColumn.geometryTypes,
                                             geomColumn.geometryOffsets,
                                             geomColumn.partOffsets,
                                             ringOffsets,
                                             geomColumn.ringOffsets);
                } else {
                    decodeLevel1WithoutRingBufferLengthStream(
                        geomColumn.geometryTypes, geomColumn.geometryOffsets, partOffsets, geomColumn.partOffsets);
                }
            }
        } else if (!geomColumn.partOffsets.empty()) {
            if (!geomColumn.ringOffsets.empty()) {
                auto partOffsets = geomColumn.partOffsets;
                auto ringOffsets = geomColumn.ringOffsets;
                decodeRootLengthStream(
                    geomColumn.geometryTypes, partOffsets, GeometryType::LINESTRING, geomColumn.partOffsets);
                decodeLevel1LengthStream(geomColumn.geometryTypes,
                                         partOffsets,
                                         geomColumn.ringOffsets,
                                         /*isLineStringPresent=*/true,
                                         geomColumn.ringOffsets);
            } else {
                auto partOffsets = geomColumn.partOffsets;
                decodeRootLengthStream(
                    geomColumn.geometryTypes, partOffsets, GeometryType::POINT, geomColumn.partOffsets);
            }
        }

        if (!geomColumn.indexBuffer.empty()) {
            /* Case when the indices of a Polygon outline are encoded in the tile */
            geomColumn.geometryVector = std::make_unique<geometry::FlatGpuVector>(
                std::move(geomColumn.geometryTypes),
                std::move(geomColumn.triangles),
                std::move(geomColumn.indexBuffer),
                std::move(geomColumn.vertices),
                geometry::TopologyVector(std::move(geomColumn.geometryOffsets),
                                         std::move(geomColumn.partOffsets),
                                         std::move(geomColumn.ringOffsets)));
            return geomColumn;
        }

        geomColumn.geometryVector = std::make_unique<geometry::FlatGeometryVector>(
            mortonSettings ? geometry::VertexBufferType::MORTON : geometry::VertexBufferType::VEC_2,
            std::move(geomColumn.geometryTypes),
            geometry::TopologyVector(std::move(geomColumn.geometryOffsets),
                                     std::move(geomColumn.partOffsets),
                                     std::move(geomColumn.ringOffsets)),
            std::move(geomColumn.vertexOffsets),
            std::move(geomColumn.vertices),
            mortonSettings);

        return geomColumn;
    }

    /*
     * Handle the parsing of the different topology length buffers separate not generic to reduce the
     * branching and improve the performance
     */
    void decodeRootLengthStream(const std::vector<metadata::tileset::GeometryType>& geometryTypes,
                                const std::vector<std::uint32_t>& rootLengthStream,
                                const metadata::tileset::GeometryType bufferId,
                                std::vector<std::uint32_t>& rootBufferOffsets) {
        rootBufferOffsets.resize(geometryTypes.size() + 1);
        std::uint32_t previousOffset = rootBufferOffsets[0] = 0;
        std::uint32_t rootLengthCounter = 0;
        for (std::size_t i = 0; i < geometryTypes.size(); ++i) {
            /* Test if the geometry has and entry in the root buffer
             * BufferId: 2 GeometryOffsets -> MultiPolygon, MultiLineString, MultiPoint
             * BufferId: 1 PartOffsets -> Polygon
             * BufferId: 0 PartOffsets, RingOffsets -> LineString
             * */
            previousOffset = rootBufferOffsets[i + 1] = previousOffset + ((geometryTypes[i] > bufferId)
                                                                              ? rootLengthStream[rootLengthCounter++]
                                                                              : 1);
        }
    }

    void decodeLevel1LengthStream(const std::vector<metadata::tileset::GeometryType>& geometryTypes,
                                  const std::vector<std::uint32_t>& rootOffsetBuffer,
                                  const std::vector<std::uint32_t>& level1LengthBuffer,
                                  const bool isLineStringPresent,
                                  std::vector<std::uint32_t>& level1BufferOffsets) {
        using metadata::tileset::GeometryType;
        level1BufferOffsets.resize(rootOffsetBuffer[rootOffsetBuffer.size() - 1] + 1);
        std::uint32_t previousOffset = level1BufferOffsets[0] = 0;
        std::uint32_t level1BufferCounter = 1;
        std::uint32_t level1LengthBufferCounter = 0;
        for (std::size_t i = 0; i < geometryTypes.size(); ++i) {
            const auto geometryType = geometryTypes[i];
            const auto numGeometries = rootOffsetBuffer[i + 1] - rootOffsetBuffer[i];

            if (geometryType == GeometryType::MULTIPOLYGON || geometryType == GeometryType::POLYGON ||
                (isLineStringPresent &&
                 (geometryType == GeometryType::MULTILINESTRING || geometryType == GeometryType::LINESTRING))) {
                /* For MultiPolygon, Polygon and in some cases for MultiLineString and LineString
                 * a value in the level1LengthBuffer exists */
                for (std::uint32_t j = 0; j < numGeometries; ++j) {
                    previousOffset = level1BufferOffsets[level1BufferCounter++] =
                        previousOffset + level1LengthBuffer[level1LengthBufferCounter++];
                }
            } else {
                /* For MultiPoint and Point and in some cases for MultiLineString and LineString no value in the
                 * level1LengthBuffer exists */
                for (std::uint32_t j = 0; j < numGeometries; j++) {
                    level1BufferOffsets[level1BufferCounter++] = ++previousOffset;
                }
            }
        }
    }

    /*
     * Case where no ring buffer exists so no MultiPolygon or Polygon geometry is part of the buffer
     */
    void decodeLevel1WithoutRingBufferLengthStream(const std::vector<metadata::tileset::GeometryType>& geometryTypes,
                                                   const std::vector<std::uint32_t>& rootOffsetBuffer,
                                                   const std::vector<std::uint32_t>& level1LengthBuffer,
                                                   std::vector<std::uint32_t>& level1BufferOffsets) {
        using metadata::tileset::GeometryType;
        level1BufferOffsets.resize(rootOffsetBuffer[rootOffsetBuffer.size() - 1] + 1);
        std::uint32_t previousOffset = level1BufferOffsets[0] = 0;
        std::uint32_t level1OffsetBufferCounter = 1;
        std::uint32_t level1LengthCounter = 0;
        for (std::size_t i = 0; i < geometryTypes.size(); ++i) {
            const auto geometryType = geometryTypes[i];
            const auto numGeometries = rootOffsetBuffer[i + 1] - rootOffsetBuffer[i];
            if (geometryType == GeometryType::MULTILINESTRING || geometryType == GeometryType::LINESTRING) {
                /* For MultiLineString and LineString a value in the level1LengthBuffer exists */
                for (std::uint32_t j = 0; j < numGeometries; ++j) {
                    previousOffset = level1BufferOffsets[level1OffsetBufferCounter++] =
                        previousOffset + level1LengthBuffer[level1LengthCounter++];
                }
            } else {
                /* For MultiPoint and Point no value in level1LengthBuffer exists */
                for (std::uint32_t j = 0; j < numGeometries; ++j) {
                    level1BufferOffsets[level1OffsetBufferCounter++] = ++previousOffset;
                }
            }
        }
    }

    void decodeLevel2LengthStream(const std::vector<metadata::tileset::GeometryType>& geometryTypes,
                                  const std::vector<std::uint32_t>& rootOffsetBuffer,
                                  const std::vector<std::uint32_t>& level1OffsetBuffer,
                                  const std::vector<std::uint32_t>& level2LengthBuffer,
                                  std::vector<std::uint32_t>& level2BufferOffsets) {
        using metadata::tileset::GeometryType;
        level2BufferOffsets.resize(level1OffsetBuffer[level1OffsetBuffer.size() - 1] + 1);
        std::uint32_t previousOffset = level2BufferOffsets[0] = 0;
        std::uint32_t level1OffsetBufferCounter = 1;
        std::uint32_t level2OffsetBufferCounter = 1;
        std::uint32_t level2LengthBufferCounter = 0;
        for (std::size_t i = 0; i < geometryTypes.size(); ++i) {
            const auto geometryType = geometryTypes[i];
            const auto numGeometries = rootOffsetBuffer[i + 1] - rootOffsetBuffer[i];
            if (geometryType != GeometryType::POINT && geometryType != GeometryType::MULTIPOINT) {
                /* For MultiPolygon, MultiLineString, Polygon and LineString a value in level2LengthBuffer
                 * exists */
                for (std::uint32_t j = 0; j < numGeometries; ++j) {
                    const auto numParts = level1OffsetBuffer[level1OffsetBufferCounter] -
                                          level1OffsetBuffer[level1OffsetBufferCounter - 1];
                    level1OffsetBufferCounter++;
                    for (std::uint32_t k = 0; k < numParts; ++k) {
                        previousOffset = level2BufferOffsets[level2OffsetBufferCounter++] =
                            previousOffset + level2LengthBuffer[level2LengthBufferCounter++];
                    }
                }
            } else {
                /* For MultiPoint and Point no value in level2LengthBuffer exists */
                for (std::uint32_t j = 0; j < numGeometries; j++) {
                    level2BufferOffsets[level2OffsetBufferCounter++] = ++previousOffset;
                    level1OffsetBufferCounter++;
                }
            }
        }
    }

    std::vector<std::unique_ptr<Geometry>> decodeGeometry(const GeometryColumn& geometryColumn) {
        using namespace geometry;
        using metadata::tileset::GeometryType;

        const auto& vertices = geometryColumn.vertices;
        const auto& vertexOffsets = geometryColumn.vertexOffsets;
        const auto& geometryOffsets = geometryColumn.geometryOffsets;
        const auto& partOffsets = geometryColumn.partOffsets;
        const auto& ringOffsets = geometryColumn.ringOffsets;

        if (vertices.empty()) {
            return {};
        }

        count_t vertexBufferOffset = 0;
        count_t vertexOffsetsOffset = 0;
        count_t geometryOffsetsCounter = 0;
        count_t partOffsetCounter = 0;
        count_t ringOffsetsCounter = 0;

        std::vector<std::unique_ptr<Geometry>> geometries;
        geometries.reserve(geometryColumn.geometryTypes.size());
        std::vector<Coordinate> coordBuffer;

        for (const auto geomType : geometryColumn.geometryTypes) {
            switch (geomType) {
                case GeometryType::POINT: {
                    auto coord = nextPoint(geometryColumn, vertexBufferOffset, vertexOffsetsOffset);
                    geometries.push_back(geometryFactory->createPoint(coord));
                    break;
                }
                case GeometryType::MULTIPOINT: {
                    if (geometryOffsetsCounter >= geometryOffsets.size()) {
                        throw std::runtime_error("geometry error");
                    }
                    const auto numPoints = geometryOffsets[geometryOffsetsCounter++];
                    coordBuffer.clear();
                    coordBuffer.reserve(numPoints);
                    std::generate_n(std::back_inserter(coordBuffer), numPoints, [&] {
                        return nextPoint(geometryColumn, vertexBufferOffset, vertexOffsetsOffset);
                    });
                    geometries.push_back(geometryFactory->createMultiPoint(std::move(coordBuffer)));
                    coordBuffer.clear();
                    break;
                }
                case GeometryType::LINESTRING: {
                    const auto hasPolygon = containsPolygon(geometryColumn);
                    if ((hasPolygon && ringOffsetsCounter >= ringOffsets.size()) ||
                        (!hasPolygon && partOffsetCounter >= partOffsets.size())) {
                        throw std::runtime_error("geometry error");
                    }

                    const auto numVertices = hasPolygon ? ringOffsets[ringOffsetsCounter++]
                                                        : partOffsets[partOffsetCounter++];

                    std::vector<Coordinate> coords;
                    if (vertexOffsets.empty()) {
                        coords = getLineStringCoords(vertices, vertexBufferOffset, numVertices, false);
                        vertexBufferOffset += numVertices * 2;
                    } else {
                        coords = getDictionaryEncodedLineStringCoords(
                            vertices, vertexOffsets, vertexOffsetsOffset, numVertices, false);
                        vertexOffsetsOffset += numVertices;
                    }
                    geometries.push_back(geometryFactory->createLineString(std::move(coords)));
                    break;
                }
                case GeometryType::POLYGON: {
                    if (partOffsetCounter >= partOffsets.size()) {
                        throw std::runtime_error("geometry error");
                    }
                    const auto numRings = partOffsets[partOffsetCounter++];
                    Polygon::Shell shell;
                    Polygon::RingVec rings;
                    rings.reserve(numRings - 1);

                    if (ringOffsetsCounter >= ringOffsets.size()) {
                        throw std::runtime_error("geometry error");
                    }
                    const auto shellVertices = ringOffsets[ringOffsetsCounter++];
                    if (vertexOffsets.empty()) {
                        shell = getLineStringCoords(vertices, vertexBufferOffset, shellVertices, true);
                        vertexBufferOffset += shellVertices * 2;
                    } else {
                        shell = getDictionaryEncodedLineStringCoords(
                            vertices, vertexOffsets, vertexOffsetsOffset, shellVertices, true);
                        vertexOffsetsOffset += shellVertices;
                    }

                    for (count_t i = 1; i < numRings; ++i) {
                        if (ringOffsetsCounter >= ringOffsets.size()) {
                            throw std::runtime_error("geometry error");
                        }
                        const auto numVertices = ringOffsets[ringOffsetsCounter++];
                        if (vertexOffsets.empty()) {
                            rings.push_back(getLineStringCoords(vertices, vertexBufferOffset, numVertices, true));
                            vertexBufferOffset += numVertices * 2;
                        } else {
                            rings.push_back(getDictionaryEncodedLineStringCoords(
                                vertices, vertexOffsets, vertexOffsetsOffset, numVertices, true));
                            vertexOffsetsOffset += numVertices;
                        }
                    }

                    geometries.push_back(geometryFactory->createPolygon(std::move(shell), std::move(rings)));
                    break;
                }
                case GeometryType::MULTILINESTRING: {
                    const auto hasPolygon = containsPolygon(geometryColumn);

                    if (geometryOffsetsCounter >= geometryOffsets.size()) {
                        throw std::runtime_error("geometry error");
                    }
                    const auto numLineStrings = geometryOffsets[geometryOffsetsCounter++];

                    if ((hasPolygon && ringOffsetsCounter + numLineStrings > ringOffsets.size()) ||
                        (!hasPolygon && partOffsetCounter + numLineStrings > partOffsets.size())) {
                        throw std::runtime_error("geometry error");
                    }

                    std::vector<std::vector<Coordinate>> lineStrings;
                    for (count_t i = 0; i < numLineStrings; ++i) {
                        const auto numVertices = hasPolygon ? ringOffsets[ringOffsetsCounter++]
                                                            : partOffsets[partOffsetCounter++];
                        std::vector<Coordinate> lineString;
                        if (vertexOffsets.empty()) {
                            lineString = getLineStringCoords(vertices, vertexBufferOffset, numVertices, false);
                            vertexBufferOffset += numVertices * 2;
                        } else {
                            lineString = getDictionaryEncodedLineStringCoords(
                                vertices, vertexOffsets, vertexOffsetsOffset, numVertices, false);
                            vertexOffsetsOffset += numVertices;
                        }
                        lineStrings.push_back(std::move(lineString));
                        lineString.clear();
                    }
                    geometries.push_back(geometryFactory->createMultiLineString(std::move(lineStrings)));
                    break;
                }
                case GeometryType::MULTIPOLYGON: {
                    if (geometryOffsetsCounter >= geometryOffsets.size()) {
                        throw std::runtime_error("geometry error");
                    }
                    const auto numPolygons = geometryOffsets[geometryOffsetsCounter++];

                    std::vector<MultiPolygon::ShellRingsPair> polygons;
                    std::vector<std::vector<Coordinate>> rings;
                    if (vertexOffsets.empty()) {
                        if (partOffsetCounter + numPolygons > partOffsets.size() ||
                            ringOffsetsCounter + numPolygons > ringOffsets.size()) {
                            throw std::runtime_error("geometry error");
                        }
                        for (count_t i = 0; i < numPolygons; ++i) {
                            const auto numRings = partOffsets[partOffsetCounter++];
                            const auto numVertices = ringOffsets[ringOffsetsCounter++];
                            rings.reserve(numRings - 1);

                            auto shell = getLineStringCoords(vertices, vertexBufferOffset, numVertices, true);
                            vertexBufferOffset += 2 * numVertices;

                            if (ringOffsetsCounter + numRings - 1 > ringOffsets.size()) {
                                throw std::runtime_error("geometry error");
                            }
                            for (count_t j = 1; j < numRings; ++j) {
                                const auto numRingVertices = ringOffsets[ringOffsetsCounter++];
                                rings.push_back(
                                    getLineStringCoords(vertices, vertexBufferOffset, numRingVertices, true));
                                vertexBufferOffset += 2 * numRingVertices;
                            }
                            polygons.emplace_back(std::move(shell), std::move(rings));
                            rings.clear();
                        }
                    } else {
                        if (partOffsetCounter + numPolygons > partOffsets.size() ||
                            ringOffsetsCounter + numPolygons > ringOffsets.size()) {
                            throw std::runtime_error("geometry error");
                        }
                        for (std::size_t i = 0; i < numPolygons; ++i) {
                            const auto numRings = partOffsets[partOffsetCounter++];
                            const auto numVertices = ringOffsets[ringOffsetsCounter++];
                            rings.reserve(numRings - 1);

                            auto shell = getDictionaryEncodedLineStringCoords(
                                vertices, vertexOffsets, vertexOffsetsOffset, numVertices, true);
                            vertexOffsetsOffset += numVertices;

                            if (ringOffsetsCounter + numRings - 1 > ringOffsets.size()) {
                                throw std::runtime_error("geometry error");
                            }
                            for (count_t j = 1; j < numRings; ++j) {
                                const auto numRingVertices = ringOffsets[ringOffsetsCounter++];
                                rings.push_back(getDictionaryEncodedLineStringCoords(
                                    vertices, vertexOffsets, vertexOffsetsOffset, numRingVertices, true));
                                vertexOffsetsOffset += numRingVertices;
                            }
                            polygons.emplace_back(std::move(shell), std::move(rings));
                            rings.clear();
                        }
                    }
                    geometries.push_back(geometryFactory->createMultiPolygon(std::move(polygons)));
                    break;
                }
                default:
                    throw std::runtime_error("Unsupported geometry type: " +
                                             std::to_string(std::to_underlying(geomType)));
            }
        }

        // We should use up all the input
        assert(geometryColumn.geometryTypes.size() == geometries.size());
        assert(!vertexOffsets.empty() || vertexBufferOffset == vertices.size());
        assert(vertexOffsets.empty() || vertexOffsetsOffset == vertexOffsets.size());
        assert(geometryOffsetsCounter == geometryOffsets.size());
        assert(partOffsetCounter == partOffsets.size());
        assert(ringOffsetsCounter == ringOffsets.size());

        return geometries;
    }

    inline static Coordinate nextPoint(const GeometryColumn& geometryColumn,
                                       count_t& vertexBufferOffset,
                                       count_t& vertexOffsetsOffset) {
        if (geometryColumn.vertexOffsets.empty()) {
            if (geometryColumn.vertices.size() < vertexBufferOffset + 2) {
                throw std::runtime_error("Vertex buffer underflow");
            }
            return {static_cast<double>(geometryColumn.vertices[vertexBufferOffset++]),
                    static_cast<double>(geometryColumn.vertices[vertexBufferOffset++])};
        }
        if (geometryColumn.vertexOffsets.size() < vertexOffsetsOffset + 1) {
            throw std::runtime_error("Vertex offset buffer underflow");
        }
        const auto offset = 2 * geometryColumn.vertexOffsets[vertexOffsetsOffset++];
        if (geometryColumn.vertices.size() < offset + 2) {
            throw std::runtime_error("Vertex buffer underflow");
        }
        return {static_cast<double>(geometryColumn.vertices[offset]),
                static_cast<double>(geometryColumn.vertices[offset + 1])};
    }

    inline static bool containsPolygon(const GeometryColumn& geometryColumn) noexcept {
        return geometryColumn.geometryTypes.end() !=
               std::ranges::find_if(geometryColumn.geometryTypes, [](const auto type) {
                   return type == metadata::tileset::GeometryType::POLYGON ||
                          type == metadata::tileset::GeometryType::MULTIPOLYGON;
               });
    }

    static std::vector<Coordinate> getLineStringCoords(const std::vector<std::int32_t>& vertexBuffer,
                                                       count_t startIndex,
                                                       count_t numVertices,
                                                       bool closeLineString) {
        std::vector<Coordinate> coords;
        coords.reserve(closeLineString ? numVertices + 1 : numVertices);
        if (startIndex + numVertices > vertexBuffer.size()) {
            throw std::runtime_error("geometry error");
        }
        std::generate_n(std::back_inserter(coords), numVertices, [&]() noexcept -> Coordinate {
            return {static_cast<double>(vertexBuffer[startIndex++]), static_cast<double>(vertexBuffer[startIndex++])};
        });
        if (closeLineString && !coords.empty()) {
            coords.emplace_back(coords.front());
        }
        return coords;
    }

    static std::vector<Coordinate> getDictionaryEncodedLineStringCoords(const std::vector<std::int32_t>& vertexBuffer,
                                                                        const std::vector<std::uint32_t>& vertexOffsets,
                                                                        const count_t vertexOffset,
                                                                        const count_t numVertices,
                                                                        const bool closeLineString) {
        std::vector<Coordinate> coords;
        coords.reserve(closeLineString ? numVertices + 1 : numVertices);
        if (vertexOffset + numVertices > vertexOffsets.size()) {
            throw std::runtime_error("geometry error");
        }
        for (count_t i = 0; i < numVertices; ++i) {
            const auto offset = 2 * vertexOffsets[vertexOffset + i];
            if (offset + 1 >= vertexBuffer.size()) {
                throw std::runtime_error("geometry error");
            }
            coords.emplace_back(static_cast<double>(vertexBuffer[offset]),
                                static_cast<double>(vertexBuffer[offset + 1]));
        }
        if (closeLineString && !coords.empty()) {
            coords.emplace_back(coords.front());
        }
        return coords;
    }

private:
    std::unique_ptr<GeometryFactory> geometryFactory;
    IntegerDecoder intDecoder;
};

} // namespace mlt::decoder
