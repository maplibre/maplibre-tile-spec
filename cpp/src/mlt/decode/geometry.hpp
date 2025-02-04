#pragma once

#include <mlt/decode/int.hpp>
#include <mlt/feature.hpp>
#include <mlt/geometry.hpp>
#include <mlt/metadata/stream.hpp>
#include <mlt/metadata/tileset.hpp>
#include <mlt/util/buffer_stream.hpp>

#include <algorithm>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace mlt::geometry {
struct GeometryColumn {
    GeometryColumn() = default;
    GeometryColumn(const GeometryColumn&) = delete;
    GeometryColumn(GeometryColumn&&) noexcept = default;
    ~GeometryColumn() = default;

    GeometryColumn& operator=(const GeometryColumn&) = delete;
    GeometryColumn& operator=(GeometryColumn&&) noexcept = default;

    std::vector<metadata::tileset::GeometryType> geometryTypes;
    std::vector<std::uint32_t> geometryOffsets;
    std::vector<std::uint32_t> partOffsets;
    std::vector<std::uint32_t> ringOffsets;
    std::vector<std::uint32_t> vertexOffsets;
    std::vector<std::int32_t> vertices;
};
} // namespace mlt::geometry

namespace mlt::decoder {

class GeometryDecoder {
    GeometryFactory geometryFactory;

public:
    using GeometryColumn = geometry::GeometryColumn;

    GeometryColumn decodeGeometryColumn(BufferStream& tileData,
                                        const metadata::tileset::Column& column,
                                        std::uint32_t numStreams) noexcept(false) {
        using namespace util::decoding;
        using namespace metadata::stream;
        using namespace metadata::tileset;

        GeometryColumn geomColumn;

        const auto geomTypeMetadata = StreamMetadata::decode(tileData);
        intDecoder.decodeIntStream<std::uint32_t, std::uint32_t, GeometryType>(
            tileData, geomColumn.geometryTypes, *geomTypeMetadata);

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
                        default:
                        case LengthType::TRIANGLES:
                            throw std::runtime_error("Length stream type '" + std::to_string(std::to_underlying(type)) +
                                                     " not implemented: " + column.name);
                    }
                    intDecoder.decodeIntStream<std::uint32_t, std::uint32_t, std::uint32_t>(
                        tileData, target->get(), *geomStreamMetadata);
                    break;
                }
                case PhysicalStreamType::OFFSET:
                    intDecoder.decodeIntStream<std::uint32_t>(tileData, geomColumn.vertexOffsets, *geomStreamMetadata);
                    break;
                case PhysicalStreamType::DATA: {
                    if (!geomStreamMetadata->getLogicalStreamType() ||
                        !geomStreamMetadata->getLogicalStreamType()->getDictionaryType()) {
                        throw std::runtime_error("Data stream missing dictionary type: " + column.name);
                    }
                    const auto type = *geomStreamMetadata->getLogicalStreamType()->getDictionaryType();
                    switch (type) {
                        case DictionaryType::VERTEX:
                            switch (geomStreamMetadata->getPhysicalLevelTechnique()) {
                                case PhysicalLevelTechnique::FAST_PFOR:
                                    throw std::runtime_error("FastPfor encoding for geometries is not yet supported.");
                                    break;
                                case PhysicalLevelTechnique::NONE:
                                case PhysicalLevelTechnique::ALP:
                                    // TODO: other implementations are not clear on whether these are valid
                                case PhysicalLevelTechnique::VARINT:
                                    intDecoder
                                        .decodeIntStream<std::uint32_t, std::uint32_t, std::int32_t, /*isSigned=*/true>(
                                            tileData, geomColumn.vertices, *geomStreamMetadata);
                                    break;
                            };
                            break;
                        case DictionaryType::MORTON: {
                            assert(geomStreamMetadata->getMetadataType() == LogicalLevelTechnique::MORTON);
                            const auto& mortonStreamMetadata = static_cast<MortonEncodedStreamMetadata&>(
                                *geomStreamMetadata);
                            intDecoder.decodeMortonStream<std::uint32_t, std::int32_t>(
                                tileData, geomColumn.vertices, mortonStreamMetadata);
                            break;
                        }
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
            }
        }

        return geomColumn;
    }

    std::vector<std::unique_ptr<Geometry>> decodeGeometry(const GeometryColumn& geometryColumn) noexcept(false) {
        using namespace geometry;
        using metadata::tileset::GeometryType;

        const auto& vertices = geometryColumn.vertices;
        const auto& vertexOffsets = geometryColumn.vertexOffsets;
        const auto& geometryOffsets = geometryColumn.geometryOffsets;
        const auto& partOffsets = geometryColumn.partOffsets;
        const auto& ringOffsets = geometryColumn.ringOffsets;

        if (vertices.empty()) {
            MLT_LOG_WARN("Warning: Vertex list is empty, skipping geometry decoding");
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
                    geometries.push_back(geometryFactory.createPoint(coord));
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
                    geometries.push_back(geometryFactory.createMultiPoint(std::move(coordBuffer)));
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
                    geometries.push_back(geometryFactory.createLineString(std::move(coords)));
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

                    geometries.push_back(geometryFactory.createPolygon(std::move(shell), std::move(rings)));
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
                    geometries.push_back(geometryFactory.createMultiLineString(std::move(lineStrings)));
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
                        if (partOffsetCounter + vertexOffsets.size() >= partOffsets.size() ||
                            ringOffsetsCounter + vertexOffsets.size() >= ringOffsets.size()) {
                            throw std::runtime_error("geometry error");
                        }
                        for (std::size_t i = 0; i < vertexOffsets.size(); ++i) {
                            const auto numRings = partOffsets[partOffsetCounter++];
                            const auto numVertices = ringOffsets[ringOffsetsCounter++];
                            rings.reserve(numRings - 1);

                            auto shell = getDictionaryEncodedLineStringCoords(
                                vertices, vertexOffsets, vertexOffsetsOffset, numVertices, true);
                            vertexOffsetsOffset += numVertices;

                            if (ringOffsetsCounter + numRings > ringOffsets.size()) {
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
                    geometries.push_back(geometryFactory.createMultiPolygon(std::move(polygons)));
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
                                       count_t& vertexOffsetsOffset) noexcept(false) {
        if (geometryColumn.vertexOffsets.empty()) {
            if (geometryColumn.vertices.size() < vertexBufferOffset + 2) {
                throw std::runtime_error("Vertex buffer underflow");
            }
            return {.x = static_cast<double>(geometryColumn.vertices[vertexBufferOffset++]),
                    .y = static_cast<double>(geometryColumn.vertices[vertexBufferOffset++])};
        }
        if (geometryColumn.vertexOffsets.size() < vertexOffsetsOffset + 1) {
            throw std::runtime_error("Vertex offset buffer underflow");
        }
        const auto offset = 2 * geometryColumn.vertexOffsets[vertexOffsetsOffset++];
        if (geometryColumn.vertices.size() < offset + 2) {
            throw std::runtime_error("Vertex buffer underflow");
        }
        return {.x = static_cast<double>(geometryColumn.vertices[offset]),
                .y = static_cast<double>(geometryColumn.vertices[offset + 1])};
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
                                                       bool closeLineString) noexcept(false) {
        std::vector<Coordinate> coords;
        coords.reserve(closeLineString ? numVertices + 1 : numVertices);
        if (startIndex + numVertices > vertexBuffer.size()) {
            throw std::runtime_error("geometry error");
        }
        std::generate_n(std::back_inserter(coords), numVertices, [&]() noexcept -> Coordinate {
            return {.x = static_cast<double>(vertexBuffer[startIndex++]),
                    .y = static_cast<double>(vertexBuffer[startIndex++])};
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
                                                                        const bool closeLineString) noexcept(false) {
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
    IntegerDecoder intDecoder;
};

} // namespace mlt::decoder
