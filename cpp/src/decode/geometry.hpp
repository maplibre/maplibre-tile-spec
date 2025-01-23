#pragma once

#include <algorithm>
#include <decode/int.hpp>
#include <feature.hpp>
#include <geometry.hpp>
#include <metadata/stream.hpp>
#include <metadata/tileset.hpp>
#include <util/buffer_stream.hpp>

#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace mlt::geometry {
struct GeometryColumn {
    GeometryColumn() = default;
    GeometryColumn(const GeometryColumn&) = delete;
    GeometryColumn(GeometryColumn&&) = default;
    ~GeometryColumn() = default;

    GeometryColumn& operator=(const GeometryColumn&) = delete;
    GeometryColumn& operator=(GeometryColumn&&) = default;

    std::vector<metadata::tileset::GeometryType> geometryTypes;
    std::vector<std::uint32_t> numGeometries;
    std::vector<std::uint32_t> numParts;
    std::vector<std::uint32_t> numRings;
    std::vector<std::uint32_t> vertexOffsets;
    std::vector<std::uint32_t> vertexList;
};
} // namespace mlt::geometry

namespace mlt::decoder {

class GeometryDecoder {
    GeometryFactory geometryFactory;

public:
    using GeometryColumn = geometry::GeometryColumn;

    GeometryColumn decodeGeometryColumn(BufferStream& tileData,
                                        const metadata::tileset::Column& column,
                                        std::uint32_t numStreams) {
        using namespace util::decoding;
        using namespace metadata::stream;

        GeometryColumn geomColumn;

        const auto geomTypeMetadata = metadata::stream::decode(tileData);
        intDecoder.decodeIntStream<uint32_t>(tileData, geomColumn.geometryTypes, *geomTypeMetadata, /*isSigned=*/false);

        for (std::uint32_t i = 1; i < numStreams; ++i) {
            const auto geomStreamMetadata = metadata::stream::decode(tileData);
            switch (geomStreamMetadata->getPhysicalStreamType()) {
                case PhysicalStreamType::LENGTH: {
                    if (!geomStreamMetadata->getLogicalStreamType() ||
                        !geomStreamMetadata->getLogicalStreamType()->getLengthType()) {
                        throw std::runtime_error("Length stream missing logical type: " + column.name);
                    }
                    const auto type = *geomStreamMetadata->getLogicalStreamType()->getLengthType();
                    geomColumn.numGeometries.resize(geomStreamMetadata->getNumValues());
                    std::optional<std::reference_wrapper<std::vector<std::uint32_t>>> target;
                    switch (type) {
                        case LengthType::GEOMETRIES:
                            target = geomColumn.numGeometries;
                            break;
                        case LengthType::PARTS:
                            target = geomColumn.numParts;
                            break;
                        case LengthType::RINGS:
                            target = geomColumn.numRings;
                            break;
                        default:
                        case LengthType::TRIANGLES:
                            throw std::runtime_error("Length stream type '" + std::to_string(std::to_underlying(type)) +
                                                     " not implemented: " + column.name);
                    }
                    intDecoder.decodeIntStream<std::uint32_t>(
                        tileData, target->get(), *geomStreamMetadata, /*isSigned=*/false);
                    break;
                }
                case PhysicalStreamType::OFFSET:
                    intDecoder.decodeIntStream<std::uint32_t>(
                        tileData, geomColumn.vertexOffsets, *geomStreamMetadata, /*isSigned=*/false);
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
                                case metadata::stream::PhysicalLevelTechnique::FAST_PFOR:
                                    throw std::runtime_error("FastPfor encoding for geometries is not yet supported.");
                                    break;
                                case metadata::stream::PhysicalLevelTechnique::NONE:
                                case metadata::stream::PhysicalLevelTechnique::ALP:
                                    // TODO: other implementations are not clear on whether these are valid
                                case metadata::stream::PhysicalLevelTechnique::VARINT:
                                    intDecoder.decodeIntStream<std::uint32_t>(
                                        tileData, geomColumn.vertexList, *geomStreamMetadata, /*isSigned=*/true);
                                    break;
                            };
                            break;
                        case DictionaryType::MORTON:
                            break;
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

    std::vector<std::unique_ptr<Geometry>> decodeGeometry(const GeometryColumn& geometryColumn) {
        using namespace geometry;
        using metadata::tileset::GeometryType;

        const auto& vertexList = geometryColumn.vertexList;
        const auto& vertexOffsets = geometryColumn.vertexOffsets;
        const auto& geometryOffsets = geometryColumn.numGeometries;
        const auto& partOffsets = geometryColumn.numParts;
        const auto& ringOffsets = geometryColumn.numRings;

        if (vertexList.empty()) {
            MLT_LOG_WARN("Warning: Vertex list is empty, skipping geometry decoding");
            return {};
        }

        offset_t vertexBufferOffset = 0;
        offset_t vertexOffsetsOffset = 0;
        offset_t geometryCounter = 0;
        offset_t geometryOffsetsCounter = 0;
        offset_t ringOffsetsCounter = 0;
        offset_t partOffsetCounter = 0;

        std::vector<std::unique_ptr<Geometry>> geometries;
        // geometries.reserve(...);
        std::vector<Coordinate> coordBuffer;

        for (const auto geomType : geometryColumn.geometryTypes) {
            switch (geomType) {
                case GeometryType::POINT: {
                    auto coord = nextPoint(geometryColumn, vertexBufferOffset, vertexOffsetsOffset);
                    geometries.push_back(geometryFactory.createPoint(coord));
                    break;
                }
                case GeometryType::MULTIPOINT: {
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
                    const auto numVertices = containsPolygon(geometryColumn) ? ringOffsets[ringOffsetsCounter++]
                                                                             : partOffsets[partOffsetCounter++];

                    if (vertexOffsets.empty()) {
                        auto vertices = getLineStringCoords(vertexList, vertexBufferOffset, numVertices, false);
                        vertexBufferOffset += numVertices * 2;
                        geometries.push_back(geometryFactory.createLineString(std::move(vertices)));
                    } else {
                        const auto vertices = getDictionaryEncodedLineStringCoords(
                            vertexList, vertexOffsets, vertexOffsetsOffset, numVertices, false);
                        vertexOffsetsOffset += numVertices;
                        geometries.push_back(geometryFactory.createLineString(std::move(vertices)));
                    }

                    break;
                }
                case GeometryType::POLYGON:
#if 0
                const numRings = partOffsets[partOffsetCounter++];
                const rings: LinearRing[] = new Array(numRings - 1);
                let numVertices = ringOffsets[ringOffsetsCounter++];
                if (!vertexOffsets || vertexOffsets.length === 0) {
                    const shell = this.getLinearRing(vertexBuffer, vertexBufferOffset, numVertices, geometryFactory);
                    vertexBufferOffset += numVertices * 2;
                    for (let i = 0; i < rings.length; i++) {
                        numVertices = ringOffsets[ringOffsetsCounter++];
                        rings[i] = this.getLinearRing(vertexBuffer, vertexBufferOffset, numVertices, geometryFactory);
                        vertexBufferOffset += numVertices * 2;
                    }
                    geometries[geometryCounter++] = geometryFactory.createPolygon(shell, rings);
                } else {
                    const shell = this.decodeDictionaryEncodedLinearRing(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, geometryFactory);
                    vertexOffsetsOffset += numVertices;
                    for (let i = 0; i < rings.length; i++) {
                        numVertices = ringOffsets[ringOffsetsCounter++];
                        rings[i] = this.decodeDictionaryEncodedLinearRing(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, geometryFactory);
                        vertexOffsetsOffset += numVertices;
                    }
                    geometries[geometryCounter++] = geometryFactory.createPolygon(shell, rings);
                }
#endif
                case GeometryType::MULTILINESTRING:
#if 0
                const numLineStrings = geometryOffsets[geometryOffsetsCounter++];
                const lineStrings: LineString[] = new Array(numLineStrings);
                if (!vertexOffsets || vertexOffsets.length === 0) {
                    for (let i = 0; i < numLineStrings; i++) {
                        const numVertices = containsPolygon
                            ? ringOffsets[ringOffsetsCounter++] : partOffsets[partOffsetCounter++];
                        const vertices = this.getLineString(vertexBuffer, vertexBufferOffset, numVertices, false);
                        lineStrings[i] = geometryFactory.createLineString(vertices);
                        vertexBufferOffset += numVertices * 2;
                    }
                    geometries[geometryCounter++] = geometryFactory.createMultiLineString(lineStrings);
                } else {
                    for (let i = 0; i < numLineStrings; i++) {
                        const numVertices = containsPolygon
                            ? ringOffsets[ringOffsetsCounter++] : partOffsets[partOffsetCounter++];
                        const vertices = this.decodeDictionaryEncodedLineString(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, false);
                        lineStrings[i] = geometryFactory.createLineString(vertices);
                        vertexOffsetsOffset += numVertices;
                    }
                    geometries[geometryCounter++] = geometryFactory.createMultiLineString(lineStrings);
                }
#endif
                case GeometryType::MULTIPOLYGON:
#if 0
                const numPolygons = geometryOffsets[geometryOffsetsCounter++];
                const polygons: Polygon[] = new Array(numPolygons);
                if (!vertexOffsets || vertexOffsets.length === 0) {
                    for (let i = 0; i < numPolygons; i++) {
                        const numRings = partOffsets[partOffsetCounter++];
                        const rings: LinearRing[] = new Array(numRings - 1);
                        const numVertices = ringOffsets[ringOffsetsCounter++];
                        const shell = this.getLinearRing(vertexBuffer, vertexBufferOffset, numVertices, geometryFactory);
                        vertexBufferOffset += numVertices * 2;
                        for (let j = 0; j < rings.length; j++) {
                            const numRingVertices = ringOffsets[ringOffsetsCounter++];
                            rings[j] = this.getLinearRing(vertexBuffer, vertexBufferOffset, numRingVertices, geometryFactory);
                            vertexBufferOffset += numRingVertices * 2;
                        }
                        polygons[i] = geometryFactory.createPolygon(shell, rings);
                    }
                    geometries[geometryCounter++] = geometryFactory.createMultiPolygon(polygons);
                } else {
                    for (let i = 0; i < numPolygons; i++) {
                        const numRings = partOffsets[partOffsetCounter++];
                        const rings: LinearRing[] = new Array(numRings - 1);
                        const numVertices = ringOffsets[ringOffsetsCounter++];
                        const shell = this.decodeDictionaryEncodedLinearRing(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, geometryFactory);
                        vertexOffsetsOffset += numVertices;
                        for (let j = 0; j < rings.length; j++) {
                            const numRingVertices = ringOffsets[ringOffsetsCounter++];
                            rings[j] = this.decodeDictionaryEncodedLinearRing(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, geometryFactory);
                            vertexOffsetsOffset += numRingVertices;
                        }
                        polygons[i] = geometryFactory.createPolygon(shell, rings);
                    }
                    geometries[geometryCounter++] = geometryFactory.createMultiPolygon(polygons);
                }
#endif
                default:
                    throw std::runtime_error("Unsupported geometry type: " +
                                             std::to_string(std::to_underlying(geomType)));
            }
        }
        return geometries;
    }

    static Coordinate nextPoint(const GeometryColumn& geometryColumn,
                                offset_t& vertexBufferOffset,
                                offset_t& vertexOffsetsOffset) {
        if (geometryColumn.vertexOffsets.empty()) {
            return {.x = static_cast<double>(geometryColumn.vertexList[vertexBufferOffset++]),
                    .y = static_cast<double>(geometryColumn.vertexList[vertexBufferOffset++])};
        }
        const auto offset = 2 * geometryColumn.vertexOffsets[vertexOffsetsOffset++];
        return {.x = static_cast<double>(geometryColumn.vertexList[offset]),
                .y = static_cast<double>(geometryColumn.vertexList[offset + 1])};
    }

    static bool containsPolygon(const GeometryColumn& geometryColumn) {
        return geometryColumn.geometryTypes.end() !=
               std::ranges::find_if(geometryColumn.geometryTypes, [](const auto type) {
                   return type == metadata::tileset::GeometryType::POLYGON ||
                          type == metadata::tileset::GeometryType::MULTIPOLYGON;
               });
    }

    static std::vector<Coordinate> getLineStringCoords(const std::vector<std::uint32_t>& vertexBuffer,
                                                       offset_t startIndex,
                                                       offset_t numVertices,
                                                       bool closeLineString) {
        std::vector<Coordinate> coords;
        coords.reserve(closeLineString ? numVertices + 1 : numVertices);
        std::generate_n(std::back_inserter(coords), numVertices, [&]() -> Coordinate {
            return {.x = static_cast<double>(vertexBuffer[startIndex++]),
                    .y = static_cast<double>(vertexBuffer[startIndex++])};
        });
        if (closeLineString && !coords.empty()) {
            coords.emplace_back(coords.front());
        }
        return coords;
    }

    static std::vector<Coordinate> getDictionaryEncodedLineStringCoords(const std::vector<std::uint32_t>& vertexBuffer,
                                                                        const std::vector<std::uint32_t>& vertexOffsets,
                                                                        offset_t vertexOffset,
                                                                        offset_t numVertices,
                                                                        bool closeLineString) {
        std::vector<Coordinate> coords;
        coords.reserve(closeLineString ? numVertices + 1 : numVertices);
        for (offset_t i = 0; i < numVertices; ++i) {
            const auto offset = 2 * vertexOffsets[vertexOffset + i];
            coords.emplace_back(static_cast<double>(vertexBuffer[offset]),
                                static_cast<double>(vertexBuffer[offset + 1]));
        }
        if (closeLineString && !coords.empty()) {
            coords.emplace_back(coords.front());
        }
        return coords;
    }

#if 0
    private static getLinearRing(vertexBuffer: number[], startIndex: number, numVertices: number, geometryFactory: GeometryFactory): LinearRing {
        const linearRing = this.getLineString(vertexBuffer, startIndex, numVertices, true);
        return geometryFactory.createLinearRing(linearRing);
    }

    private static decodeDictionaryEncodedLinearRing(vertexBuffer: number[], vertexOffsets: number[], vertexOffset: number, numVertices: number, geometryFactory: GeometryFactory): LinearRing {
        const linearRing = this.decodeDictionaryEncodedLineString(vertexBuffer, vertexOffsets, vertexOffset, numVertices, true);
        return geometryFactory.createLinearRing(linearRing);
    }
#endif

private:
    IntegerDecoder intDecoder;
};

} // namespace mlt::decoder
