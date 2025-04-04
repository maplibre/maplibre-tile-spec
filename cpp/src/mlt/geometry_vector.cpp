#include <mlt/geometry_vector.hpp>

#include <mlt/coordinate.hpp>
#include <mlt/polyfill.hpp>
#include <mlt/util/morton_curve.hpp>
#include <mlt/util/stl.hpp>

namespace mlt::geometry {

#if !defined(MLT_GEOMETRY_CHECKS)
#define MLT_GEOMETRY_CHECKS 1
#endif
namespace {
#if MLT_GEOMETRY_CHECKS
void checkBuffer(std::size_t index, std::size_t size, std::string_view name) {
    if (index >= size) {
        throw std::runtime_error(std::string(name) + " underflow");
    }
}
#define CHECK_BUFFER(I, B) checkBuffer(I, B.size(), #B)
#else
void checkBuffer(std::size_t, std::size_t, std::string_view) {}
#define CHECK_BUFFER(I, B) ((void)0)
#endif

inline Coordinate coord(std::int32_t x, std::int32_t y) {
    return {static_cast<float>(x), static_cast<float>(y)};
}

std::vector<Coordinate> getLineStringCoords(const std::vector<std::int32_t>& vertexBuffer,
                                            std::uint32_t startIndex,
                                            const std::uint32_t numVertices,
                                            bool closeLineString) {
    std::vector<Coordinate> coords;
    coords.reserve(numVertices + 1);
    CHECK_BUFFER(startIndex + numVertices - 1, vertexBuffer);
    for (std::uint32_t i = 0; i < numVertices; ++i) {
        const auto x = vertexBuffer[startIndex++];
        coords.push_back(coord(x, vertexBuffer[startIndex++]));
    }
    if (closeLineString && !coords.empty() && coords.front() != coords.back()) {
        coords.push_back(coords.front());
    }
    return coords;
}

std::vector<Coordinate> getDictionaryEncodedLineStringCoords(const std::vector<std::int32_t>& vertexBuffer,
                                                             const std::vector<std::uint32_t>& vertexOffsets,
                                                             std::uint32_t vertexOffset,
                                                             const std::uint32_t numVertices,
                                                             const bool closeLineString) {
    std::vector<Coordinate> coords;
    coords.reserve(numVertices + 1);
    CHECK_BUFFER(vertexOffset + numVertices - 1, vertexOffsets);
    for (std::uint32_t i = 0; i < numVertices; ++i) {
        const auto offset = 2 * vertexOffsets[vertexOffset++];
        CHECK_BUFFER(offset + 1, vertexBuffer);
        coords.push_back(coord(vertexBuffer[offset], vertexBuffer[offset + 1]));
    }
    if (closeLineString && !coords.empty() && coords.front() != coords.back()) {
        coords.push_back(coords.front());
    }
    return coords;
}

std::vector<Coordinate> getMortonEncodedLineStringCoords(const std::vector<std::int32_t>& vertexBuffer,
                                                         const std::vector<std::uint32_t>& vertexOffsets,
                                                         std::uint32_t vertexOffset,
                                                         const std::uint32_t numVertices,
                                                         const MortonSettings& mortonSettings,
                                                         const bool closeLineString) {
    std::vector<Coordinate> coords;
    coords.reserve(numVertices + 1);
    CHECK_BUFFER(vertexOffset + numVertices - 1, vertexOffsets);
    for (std::uint32_t i = 1; i < numVertices; ++i) {
        const auto offset = vertexOffsets[vertexOffset++];
        CHECK_BUFFER(offset, vertexBuffer);
        coords.push_back(
            util::MortonCurve::decode(vertexBuffer[offset], mortonSettings.numBits, mortonSettings.coordinateShift));
    }
    if (closeLineString && !coords.empty() && coords.front() != coords.back()) {
        coords.push_back(coords.front());
    }
    return coords;
}

} // namespace

std::vector<std::unique_ptr<Geometry>> GeometryVector::getGeometries(const GeometryFactory& factory) const {
    std::vector<std::unique_ptr<Geometry>> geometries;
    geometries.reserve(numGeometries);

    std::uint32_t partOffsetCounter = 1;
    std::uint32_t ringOffsetsCounter = 1;
    std::uint32_t geometryOffsetsCounter = 1;
    std::uint32_t vertexBufferOffset = 0;
    std::uint32_t vertexOffsetsOffset = 0;

    if (!topologyVector) {
        throw std::runtime_error("Missing topology vector");
    }

    const auto& geometryOffsets = topologyVector->getGeometryOffsets();
    const auto& partOffsets = topologyVector->getPartOffsets();
    const auto& ringOffsets = topologyVector->getRingOffsets();

    const auto containsPolygon = containsPolygonGeometry();

    for (std::size_t i = 0; i < numGeometries; ++i) {
        using metadata::tileset::GeometryType;
        const auto geomType = getGeometryType(i);

        switch (geomType) {
            case GeometryType::POINT:
                if (vertexOffsets.empty()) {
                    CHECK_BUFFER(vertexBufferOffset + 1, vertexBuffer);
                    geometries.push_back(
                        factory.createPoint({static_cast<float>(vertexBuffer[vertexBufferOffset]),
                                             static_cast<float>(vertexBuffer[vertexBufferOffset + 1])}));
                } else if (vertexBufferType == VertexBufferType::VEC_2) {
                    CHECK_BUFFER(vertexOffsetsOffset, vertexOffsets);
                    const auto vertexOffset = vertexOffsets[vertexOffsetsOffset++];
                    CHECK_BUFFER(vertexOffset + 1, vertexBuffer);
                    geometries.push_back(factory.createPoint({static_cast<float>(vertexBuffer[vertexOffset]),
                                                              static_cast<float>(vertexBuffer[vertexOffset + 1])}));
                } else {
                    assert(vertexBufferType == VertexBufferType::MORTON);
                    CHECK_BUFFER(vertexOffsetsOffset, vertexOffsets);
                    const auto mortonCode = vertexOffsets[vertexOffsetsOffset++];
                    geometries.push_back(factory.createPoint(util::MortonCurve::decode(
                        mortonCode, mortonSettings->numBits, mortonSettings->coordinateShift)));
                }
                if (!geometryOffsets.empty()) {
                    geometryOffsetsCounter++;
                }
                if (!partOffsets.empty()) {
                    partOffsetCounter++;
                }
                if (!ringOffsets.empty()) {
                    ringOffsetsCounter++;
                }
                break;
            case GeometryType::MULTIPOINT: {
                CHECK_BUFFER(geometryOffsetsCounter, geometryOffsets);
                const auto numPoints = geometryOffsets[geometryOffsetsCounter] -
                                       geometryOffsets[geometryOffsetsCounter - 1];
                geometryOffsetsCounter++;

                if (numPoints) {
                    std::vector<Coordinate> vertices;
                    vertices.reserve(numPoints);

                    if (vertexOffsets.empty()) {
                        CHECK_BUFFER(vertexBufferOffset + (2 * numPoints) - 1, vertexBuffer);
                        for (std::uint32_t i = 0; i < numPoints; ++i) {
                            const auto x = vertexBuffer[vertexBufferOffset++];
                            vertices.push_back(coord(x, vertexBuffer[vertexBufferOffset++]));
                        }
                    } else {
                        CHECK_BUFFER(vertexOffsetsOffset + numPoints - 1, vertexOffsets);
                        for (std::uint32_t i = 0; i < numPoints; ++i) {
                            const auto offset = vertexOffsets[vertexOffsetsOffset++] * 2;
                            vertices.push_back(coord(vertexBuffer[offset], vertexBuffer[offset + 1]));
                        }
                    }
                    geometries.push_back(factory.createMultiPoint(std::move(vertices)));
                }
                break;
            }
            case GeometryType::LINESTRING: {
                std::uint32_t numVertices = 0;
                if (containsPolygon) {
                    CHECK_BUFFER(ringOffsetsCounter, ringOffsets);
                    numVertices = ringOffsets[ringOffsetsCounter] - ringOffsets[ringOffsetsCounter - 1];
                    ringOffsetsCounter++;
                } else {
                    CHECK_BUFFER(partOffsetCounter, partOffsets);
                    numVertices = partOffsets[partOffsetCounter] - partOffsets[partOffsetCounter - 1];
                }
                //?
                if (!partOffsets.empty()) {
                    partOffsetCounter++;
                }

                std::vector<Coordinate> vertices;
                if (vertexOffsets.empty()) {
                    vertices = getLineStringCoords(
                        vertexBuffer, vertexBufferOffset, numVertices, /*closeLineString=*/false);
                    vertexBufferOffset += numVertices * 2;
                } else {
                    vertices = (vertexBufferType == VertexBufferType::VEC_2)
                                   ? getDictionaryEncodedLineStringCoords(vertexBuffer,
                                                                          vertexOffsets,
                                                                          vertexOffsetsOffset,
                                                                          numVertices,
                                                                          /*closeLineString=*/false)
                                   : getMortonEncodedLineStringCoords(vertexBuffer,
                                                                      vertexOffsets,
                                                                      vertexOffsetsOffset,
                                                                      numVertices,
                                                                      *mortonSettings,
                                                                      /*closeLineString=*/false);
                    vertexOffsetsOffset += numVertices;
                }

                geometries.push_back(factory.createLineString(std::move(vertices)));
                break;
            }
            case GeometryType::POLYGON: {
                CHECK_BUFFER(partOffsetCounter, partOffsets);
                const auto numRings = partOffsets[partOffsetCounter] - partOffsets[partOffsetCounter - 1];
                partOffsetCounter++;

                CHECK_BUFFER(ringOffsetsCounter, ringOffsets);
                const auto numVertices = ringOffsets[ringOffsetsCounter] - ringOffsets[ringOffsetsCounter - 1];
                ringOffsetsCounter++;

                std::vector<CoordVec> rings;
                rings.reserve(numRings - 1);

                if (vertexOffsets.empty()) {
                    auto shell = getLineStringCoords(
                        vertexBuffer, vertexBufferOffset, numVertices, /*closeLineString=*/true);
                    vertexBufferOffset += numVertices * 2;

                    for (std::uint32_t i = 1; i < numRings; ++i) {
                        CHECK_BUFFER(ringOffsetsCounter, ringOffsets);
                        const auto numRingVertices = ringOffsets[ringOffsetsCounter] -
                                                     ringOffsets[ringOffsetsCounter - 1];
                        ringOffsetsCounter++;
                        const auto vbo = vertexBufferOffset;
                        vertexBufferOffset += numRingVertices * 2;
                        rings.push_back(
                            getLineStringCoords(vertexBuffer, vbo, numRingVertices, /*closeLineString=*/true));
                    }
                    geometries.push_back(factory.createPolygon(std::move(shell), std::move(rings)));
                } else {
                    auto shell = (vertexBufferType == VertexBufferType::VEC_2)
                                     ? getDictionaryEncodedLineStringCoords(vertexBuffer,
                                                                            vertexOffsets,
                                                                            vertexOffsetsOffset,
                                                                            numVertices,
                                                                            /*closeLineString=*/true)
                                     : getMortonEncodedLineStringCoords(vertexBuffer,
                                                                        vertexOffsets,
                                                                        vertexOffsetsOffset,
                                                                        numVertices,
                                                                        *mortonSettings,
                                                                        /*closeLineString=*/true);
                    vertexOffsetsOffset += numVertices;

                    for (std::uint32_t i = 1; i < numRings; ++i) {
                        CHECK_BUFFER(ringOffsetsCounter, ringOffsets);
                        const auto numRingVertices = ringOffsets[ringOffsetsCounter] -
                                                     ringOffsets[ringOffsetsCounter - 1];
                        ringOffsetsCounter++;

                        const auto offset = vertexOffsetsOffset;
                        vertexOffsetsOffset += numRingVertices;

                        rings.push_back(
                            (vertexBufferType == VertexBufferType::VEC_2)
                                ? getDictionaryEncodedLineStringCoords(
                                      vertexBuffer, vertexOffsets, offset, numRingVertices, /*closeLineString=*/true)
                                : getMortonEncodedLineStringCoords(vertexBuffer,
                                                                   vertexOffsets,
                                                                   offset,
                                                                   numRingVertices,
                                                                   *mortonSettings,
                                                                   /*closeLineString=*/true));
                    }
                    geometries.push_back(factory.createPolygon(std::move(shell), std::move(rings)));
                }

                if (!geometryOffsets.empty()) {
                    geometryOffsetsCounter++;
                }
                break;
            }
            case GeometryType::MULTILINESTRING: {
                CHECK_BUFFER(geometryOffsetsCounter, geometryOffsets);
                const auto numLineStrings = geometryOffsets[geometryOffsetsCounter] -
                                            geometryOffsets[geometryOffsetsCounter - 1];
                geometryOffsetsCounter++;

                std::vector<CoordVec> lineStrings;
                lineStrings.reserve(numLineStrings);

                if (vertexOffsets.empty()) {
                    for (int j = 0; j < numLineStrings; j++) {
                        std::uint32_t numVertices = 0;
                        if (containsPolygon) {
                            CHECK_BUFFER(ringOffsetsCounter, ringOffsets);
                            numVertices = ringOffsets[ringOffsetsCounter] - ringOffsets[ringOffsetsCounter - 1];
                            ringOffsetsCounter++;
                        } else {
                            CHECK_BUFFER(partOffsetCounter, partOffsets);
                            numVertices = partOffsets[partOffsetCounter] - partOffsets[partOffsetCounter - 1];
                        }

                        // ?
                        if (!partOffsets.empty()) {
                            partOffsetCounter++;
                        }

                        lineStrings.push_back(getLineStringCoords(
                            vertexBuffer, vertexBufferOffset, numVertices, /*closeLineString=*/false));
                        vertexBufferOffset += numVertices * 2;
                    }
                    geometries.push_back(factory.createMultiLineString(std::move(lineStrings)));
                } else {
                    for (std::uint32_t i = 0; i < numLineStrings; ++i) {
                        std::uint32_t numVertices = 0;
                        if (containsPolygon) {
                            CHECK_BUFFER(ringOffsetsCounter, ringOffsets);
                            numVertices = ringOffsets[ringOffsetsCounter] - ringOffsets[ringOffsetsCounter - 1];
                            ringOffsetsCounter++;
                        } else {
                            CHECK_BUFFER(partOffsetCounter, partOffsets);
                            numVertices = partOffsets[partOffsetCounter] - partOffsets[partOffsetCounter - 1];
                        }

                        // ?
                        if (!partOffsets.empty()) {
                            partOffsetCounter++;
                        }

                        lineStrings.push_back((vertexBufferType == VertexBufferType::VEC_2)
                                                  ? getDictionaryEncodedLineStringCoords(vertexBuffer,
                                                                                         vertexOffsets,
                                                                                         vertexOffsetsOffset,
                                                                                         numVertices,
                                                                                         /*closeLineString=*/false)
                                                  : getMortonEncodedLineStringCoords(vertexBuffer,
                                                                                     vertexOffsets,
                                                                                     vertexOffsetsOffset,
                                                                                     numVertices,
                                                                                     *mortonSettings,
                                                                                     /*closeLineString=*/false));
                        vertexOffsetsOffset += numVertices;
                    }
                    geometries.push_back(factory.createMultiLineString(std::move(lineStrings)));
                }
                break;
            }
            case GeometryType::MULTIPOLYGON: {
                CHECK_BUFFER(geometryOffsetsCounter, geometryOffsets);
                const auto numPolygons = geometryOffsets[geometryOffsetsCounter] -
                                         geometryOffsets[geometryOffsetsCounter - 1];
                geometryOffsetsCounter++;

                std::vector<MultiPolygon::ShellRingsPair> polygons;
                polygons.reserve(numPolygons);

                std::uint32_t numVertices = 0;
                if (vertexOffsets.empty()) {
                    for (std::uint32_t i = 0; i < numPolygons; ++i) {
                        CHECK_BUFFER(partOffsetCounter, partOffsets);
                        const auto numRings = partOffsets[partOffsetCounter] - partOffsets[partOffsetCounter - 1];
                        partOffsetCounter++;

                        CHECK_BUFFER(ringOffsetsCounter, ringOffsets);
                        numVertices = ringOffsets[ringOffsetsCounter] - ringOffsets[ringOffsetsCounter - 1];
                        ringOffsetsCounter++;

                        std::vector<CoordVec> rings;
                        rings.reserve(numRings - 1);

                        auto shell = getLineStringCoords(
                            vertexBuffer, vertexBufferOffset, numVertices, /*closeLineString=*/true);
                        vertexBufferOffset += numVertices * 2;

                        for (std::uint32_t j = 1; j < numRings; ++j) {
                            CHECK_BUFFER(ringOffsetsCounter, ringOffsets);
                            const auto numRingVertices = ringOffsets[ringOffsetsCounter] -
                                                         ringOffsets[ringOffsetsCounter - 1];
                            ringOffsetsCounter++;

                            rings.push_back(getLineStringCoords(
                                vertexBuffer, vertexBufferOffset, numRingVertices, /*closeLineString=*/true));
                            vertexBufferOffset += numRingVertices * 2;
                        }

                        polygons.push_back({std::move(shell), std::move(rings)});
                    }
                    geometries.push_back(factory.createMultiPolygon(std::move(polygons)));
                } else {
                    for (std::uint32_t i = 0; i < numPolygons; ++i) {
                        CHECK_BUFFER(partOffsetCounter, partOffsets);
                        const auto numRings = partOffsets[partOffsetCounter] - partOffsets[partOffsetCounter - 1];
                        partOffsetCounter++;

                        CHECK_BUFFER(ringOffsetsCounter, ringOffsets);
                        numVertices = ringOffsets[ringOffsetsCounter] - ringOffsets[ringOffsetsCounter - 1];
                        ringOffsetsCounter++;

                        std::vector<CoordVec> rings;
                        rings.reserve(numRings - 1);

                        auto shell = (vertexBufferType == VertexBufferType::VEC_2)
                                         ? getDictionaryEncodedLineStringCoords(vertexBuffer,
                                                                                vertexOffsets,
                                                                                vertexOffsetsOffset,
                                                                                numVertices,
                                                                                /*closeLineString=*/true)
                                         : getMortonEncodedLineStringCoords(vertexBuffer,
                                                                            vertexOffsets,
                                                                            vertexOffsetsOffset,
                                                                            numVertices,
                                                                            *mortonSettings,
                                                                            /*closeLineString=*/true);
                        vertexOffsetsOffset += numVertices;

                        for (std::uint32_t j = 1; j < numRings; ++j) {
                            CHECK_BUFFER(ringOffsetsCounter, ringOffsets);
                            const auto numRingVertices = ringOffsets[ringOffsetsCounter] -
                                                         ringOffsets[ringOffsetsCounter - 1];
                            ringOffsetsCounter++;

                            rings.push_back((vertexBufferType == VertexBufferType::VEC_2)
                                                ? getDictionaryEncodedLineStringCoords(vertexBuffer,
                                                                                       vertexOffsets,
                                                                                       vertexOffsetsOffset,
                                                                                       numRingVertices,
                                                                                       /*closeLineString=*/true)
                                                : getMortonEncodedLineStringCoords(vertexBuffer,
                                                                                   vertexOffsets,
                                                                                   vertexOffsetsOffset,
                                                                                   numRingVertices,
                                                                                   *mortonSettings,
                                                                                   /*closeLineString=*/true));
                            vertexOffsetsOffset += numRingVertices;
                        }

                        polygons.emplace_back(std::move(shell), std::move(rings));
                    }
                    geometries.push_back(factory.createMultiPolygon(std::move(polygons)));
                }
                break;
            }
            default:
                throw std::runtime_error("Unsupported geometry type: " + std::to_string(std::to_underlying(geomType)));
        }
    }
    return geometries;
}

} // namespace mlt::geometry
