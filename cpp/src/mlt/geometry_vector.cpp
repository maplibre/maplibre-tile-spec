#include <mlt/geometry_vector.hpp>

#include <mlt/coordinate.hpp>
#include <mlt/geometry.hpp>
#include <mlt/metadata/tileset.hpp>
#include <mlt/polyfill.hpp> // NOLINT(misc-include-cleaner)
#include <mlt/util/morton_curve.hpp>

#include <algorithm>
#include <cassert>
#include <cstddef>
#include <cstdint>
#include <memory>
#include <stdexcept>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

#ifndef NDEBUG
#include <iterator>
#endif

// NOLINTBEGIN(bugprone-unchecked-optional-access)

namespace mlt::geometry {

namespace {
constexpr void checkBuffer(std::size_t index, std::size_t size, std::string_view name) {
    if (index >= size) {
        throw std::runtime_error(std::string(name) + " underflow");
    }
}
#define CHECK_BUFFER(I, B) checkBuffer(I, (B).size(), #B)

/// The number of items between two consecutive offsets, which must be there and not decrease
inline std::uint32_t offsetDelta(const std::vector<std::uint32_t>& offsets, std::size_t index, std::string_view name) {
    if (index == 0 || index >= offsets.size()) {
        throw std::runtime_error(std::string(name) + " underflow");
    }
    if (offsets[index] < offsets[index - 1]) {
        throw std::runtime_error(std::string(name) + " decreases");
    }
    return offsets[index] - offsets[index - 1];
}
#define OFFSET_DELTA(B, I) offsetDelta(B, I, #B)

/// Check that `count` items are available from `start`, where each takes `perItem` elements
constexpr void checkRange(
    std::size_t start, std::size_t count, std::size_t perItem, std::size_t size, std::string_view name) {
    if (start > size || count > (size - start) / perItem) {
        throw std::runtime_error(std::string(name) + " underflow");
    }
}
#define CHECK_RANGE(S, N, P, B) checkRange(S, N, P, (B).size(), #B)

inline Coordinate coord(std::int32_t x, std::int32_t y) {
    return {static_cast<float>(x), static_cast<float>(y)};
}

std::vector<Coordinate> getLineStringCoords(const std::vector<std::int32_t>& vertexBuffer,
                                            std::size_t startIndex,
                                            const std::uint32_t numVertices,
                                            bool closeLineString) {
    // Each vertex takes two values
    CHECK_RANGE(startIndex, numVertices, 2, vertexBuffer);
    std::vector<Coordinate> coords;
    coords.reserve(std::size_t{numVertices} + 1);
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
                                                             std::size_t vertexOffset,
                                                             const std::uint32_t numVertices,
                                                             const bool closeLineString) {
    CHECK_RANGE(vertexOffset, numVertices, 1, vertexOffsets);
    std::vector<Coordinate> coords;
    coords.reserve(std::size_t{numVertices} + 1);
    for (std::uint32_t i = 0; i < numVertices; ++i) {
        const auto offset = 2 * std::size_t{vertexOffsets[vertexOffset++]};
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
                                                         std::size_t vertexOffset,
                                                         const std::uint32_t numVertices,
                                                         const MortonSettings& mortonSettings,
                                                         const bool closeLineString) {
    CHECK_RANGE(vertexOffset, numVertices, 1, vertexOffsets);
    std::vector<Coordinate> coords;
    coords.reserve(std::size_t{numVertices} + 1);
    for (std::uint32_t i = 0; i < numVertices; ++i) {
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

void GeometryVector::applyTriangles(Geometry& geom,
                                    std::size_t& triangleOffset,
                                    std::size_t& indexBufferOffset,
                                    std::uint32_t totalVertices,
                                    [[maybe_unused]] bool multiPolygon) const {
    if (triangleCounts.empty()) {
        return;
    }

    CHECK_BUFFER(triangleOffset, triangleCounts);
    const auto numTriangles = triangleCounts[triangleOffset++];
    if (numTriangles) {
        CHECK_RANGE(indexBufferOffset, numTriangles, 3, indexBuffer);
        const std::size_t numIndexes = 3 * std::size_t{numTriangles};
        // The indexes are used by whatever draws the triangles, to refer to the vertices
        if (!std::all_of(std::next(indexBuffer.cbegin(), static_cast<std::ptrdiff_t>(indexBufferOffset)),
                         std::next(indexBuffer.cbegin(), static_cast<std::ptrdiff_t>(indexBufferOffset + numIndexes)),
                         [=](auto i) { return i < totalVertices; })) {
            throw std::runtime_error("Triangle index out of range");
        }
        geom.setTriangles({&indexBuffer[indexBufferOffset], numIndexes});
        indexBufferOffset += numIndexes;
    }
}

std::vector<std::unique_ptr<Geometry>> GeometryVector::getGeometries(const GeometryFactory& factory) const {
    std::vector<std::unique_ptr<Geometry>> geometries;
    geometries.reserve(numGeometries);

    std::size_t partOffsetCounter = 1;
    std::size_t ringOffsetsCounter = 1;
    std::size_t geometryOffsetsCounter = 1;
    std::size_t vertexBufferOffset = 0;
    std::size_t vertexOffsetsOffset = 0;
    std::size_t indexBufferOffset = 0;
    std::size_t triangleOffset = 0;

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
                    vertexBufferOffset += 2;
                } else if (vertexBufferType == VertexBufferType::VEC_2) {
                    CHECK_BUFFER(vertexOffsetsOffset, vertexOffsets);
                    const auto vertexOffset = 2 * std::size_t{vertexOffsets[vertexOffsetsOffset++]};
                    CHECK_BUFFER(vertexOffset + 1, vertexBuffer);
                    geometries.push_back(factory.createPoint({static_cast<float>(vertexBuffer[vertexOffset]),
                                                              static_cast<float>(vertexBuffer[vertexOffset + 1])}));
                } else {
                    if (vertexBufferType != VertexBufferType::MORTON || !mortonSettings) {
                        throw std::runtime_error("Invalid vertex buffer type");
                    }
                    CHECK_BUFFER(vertexOffsetsOffset, vertexOffsets);
                    const auto mortonCode = vertexOffsets[vertexOffsetsOffset++];
                    geometries.push_back(factory.createPoint(util::MortonCurve::decode(
                        mortonCode, mortonSettings->numBits, mortonSettings->coordinateShift)));
                }

                geometryOffsetsCounter++;
                partOffsetCounter++;
                ringOffsetsCounter++;

                break;
            case GeometryType::MULTIPOINT: {
                if (!topologyVector) {
                    throw std::runtime_error("Multi-point geometry without topology vector");
                }
                const auto& geometryOffsets = topologyVector->getGeometryOffsets();
                [[maybe_unused]] const auto& partOffsets = topologyVector->getPartOffsets();
                [[maybe_unused]] const auto& ringOffsets = topologyVector->getRingOffsets();

                const auto numPoints = OFFSET_DELTA(geometryOffsets, geometryOffsetsCounter);
                geometryOffsetsCounter++;

                if (numPoints) {
                    std::vector<Coordinate> vertices;

                    if (vertexOffsets.empty()) {
                        CHECK_RANGE(vertexBufferOffset, numPoints, 2, vertexBuffer);
                        vertices.reserve(numPoints);
                        for (std::uint32_t j = 0; j < numPoints; ++j) {
                            const auto x = vertexBuffer[vertexBufferOffset++];
                            vertices.push_back(coord(x, vertexBuffer[vertexBufferOffset++]));
                        }
                    } else {
                        CHECK_RANGE(vertexOffsetsOffset, numPoints, 1, vertexOffsets);
                        vertices.reserve(numPoints);
                        for (std::uint32_t j = 0; j < numPoints; ++j) {
                            const auto offset = 2 * std::size_t{vertexOffsets[vertexOffsetsOffset++]};
                            CHECK_BUFFER(offset + 1, vertexBuffer);
                            vertices.push_back(coord(vertexBuffer[offset], vertexBuffer[offset + 1]));
                        }
                    }
                    geometries.push_back(factory.createMultiPoint(std::move(vertices)));
                }

                partOffsetCounter += std::size_t{numPoints};
                ringOffsetsCounter += std::size_t{numPoints};
                break;
            }
            case GeometryType::LINESTRING: {
                if (!topologyVector) {
                    throw std::runtime_error("Linestring geometry without topology vector");
                }

                std::uint32_t numVertices = 0;
                if (containsPolygon) {
                    const auto& ringOffsets = topologyVector->getRingOffsets();
                    numVertices = OFFSET_DELTA(ringOffsets, ringOffsetsCounter);
                    ringOffsetsCounter++;
                } else {
                    const auto& partOffsets = topologyVector->getPartOffsets();
                    numVertices = OFFSET_DELTA(partOffsets, partOffsetCounter);
                }
                partOffsetCounter++;

                std::vector<Coordinate> vertices;
                if (vertexOffsets.empty()) {
                    vertices = getLineStringCoords(
                        vertexBuffer, vertexBufferOffset, numVertices, /*closeLineString=*/false);
                    vertexBufferOffset += 2 * std::size_t{numVertices};
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

                geometryOffsetsCounter++;

                geometries.push_back(factory.createLineString(std::move(vertices)));
                break;
            }
            case GeometryType::POLYGON: {
                if (!topologyVector) {
                    throw std::runtime_error("Polygon geometry without topology vector");
                }
                [[maybe_unused]] const auto& geometryOffsets = topologyVector->getGeometryOffsets();
                const auto& partOffsets = topologyVector->getPartOffsets();
                const auto& ringOffsets = topologyVector->getRingOffsets();

                const auto numRings = OFFSET_DELTA(partOffsets, partOffsetCounter);
                partOffsetCounter++;

                const auto numVertices = OFFSET_DELTA(ringOffsets, ringOffsetsCounter);
                ringOffsetsCounter++;

                std::vector<CoordVec> rings;
                rings.reserve(std::min<std::size_t>(numRings, ringOffsets.size()));

                auto totalVertices = numVertices;
                if (vertexOffsets.empty()) {
                    rings.push_back(
                        getLineStringCoords(vertexBuffer, vertexBufferOffset, numVertices, /*closeLineString=*/true));
                    vertexBufferOffset += 2 * std::size_t{numVertices};

                    for (std::uint32_t j = 1; j < numRings; ++j) {
                        const auto numRingVertices = OFFSET_DELTA(ringOffsets, ringOffsetsCounter);
                        totalVertices += numRingVertices;
                        ringOffsetsCounter++;
                        const auto vbo = vertexBufferOffset;
                        vertexBufferOffset += 2 * std::size_t{numRingVertices};
                        rings.push_back(
                            getLineStringCoords(vertexBuffer, vbo, numRingVertices, /*closeLineString=*/true));
                    }

                    auto newGeometry = factory.createPolygon(std::move(rings));
                    applyTriangles(*newGeometry, triangleOffset, indexBufferOffset, totalVertices, false);
                    geometries.push_back(std::move(newGeometry));
                } else {
                    rings.push_back((vertexBufferType == VertexBufferType::VEC_2)
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
                                                                           /*closeLineString=*/true));
                    vertexOffsetsOffset += numVertices;

                    auto dictTotalVertices = numVertices;
                    for (std::uint32_t j = 1; j < numRings; ++j) {
                        const auto numRingVertices = OFFSET_DELTA(ringOffsets, ringOffsetsCounter);
                        ringOffsetsCounter++;

                        dictTotalVertices += numRingVertices;

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

                    auto newGeometry = factory.createPolygon(std::move(rings));
                    applyTriangles(*newGeometry, triangleOffset, indexBufferOffset, dictTotalVertices, false);
                    geometries.push_back(std::move(newGeometry));
                }

                if (topologyVector && !topologyVector->getGeometryOffsets().empty()) {
                    geometryOffsetsCounter++;
                }
                break;
            }
            case GeometryType::MULTILINESTRING: {
                if (!topologyVector) {
                    throw std::runtime_error("Multi-Linestring geometry without topology vector");
                }
                const auto& geometryOffsets = topologyVector->getGeometryOffsets();
                const auto& partOffsets = topologyVector->getPartOffsets();
                const auto& ringOffsets = topologyVector->getRingOffsets();

                const auto numLineStrings = OFFSET_DELTA(geometryOffsets, geometryOffsetsCounter);
                geometryOffsetsCounter++;

                std::vector<CoordVec> lineStrings;
                lineStrings.reserve(std::min<std::size_t>(numLineStrings, partOffsets.size()));

                if (vertexOffsets.empty()) {
                    for (std::uint32_t j = 0; j < numLineStrings; j++) {
                        std::uint32_t numVertices = 0;
                        if (containsPolygon) {
                            numVertices = OFFSET_DELTA(ringOffsets, ringOffsetsCounter);
                            ringOffsetsCounter++;
                        } else {
                            numVertices = OFFSET_DELTA(partOffsets, partOffsetCounter);
                        }
                        partOffsetCounter++;

                        lineStrings.push_back(getLineStringCoords(
                            vertexBuffer, vertexBufferOffset, numVertices, /*closeLineString=*/false));
                        vertexBufferOffset += 2 * std::size_t{numVertices};
                    }
                    geometries.push_back(factory.createMultiLineString(std::move(lineStrings)));
                } else {
                    for (std::uint32_t j = 0; j < numLineStrings; ++j) {
                        std::uint32_t numVertices = 0;
                        if (containsPolygon) {
                            numVertices = OFFSET_DELTA(ringOffsets, ringOffsetsCounter);
                            ringOffsetsCounter++;
                        } else {
                            numVertices = OFFSET_DELTA(partOffsets, partOffsetCounter);
                        }
                        partOffsetCounter++;

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
                if (!topologyVector) {
                    throw std::runtime_error("Multi-polygon geometry without topology vector");
                }
                const auto& geometryOffsets = topologyVector->getGeometryOffsets();
                const auto& partOffsets = topologyVector->getPartOffsets();
                const auto& ringOffsets = topologyVector->getRingOffsets();

                const auto numPolygons = OFFSET_DELTA(geometryOffsets, geometryOffsetsCounter);
                geometryOffsetsCounter++;

                std::vector<MultiPolygon::RingVec> polygons;
                polygons.reserve(std::min<std::size_t>(numPolygons, partOffsets.size()));

                if (vertexOffsets.empty()) {
                    [[maybe_unused]] std::uint32_t plainTotalVertices = 0;
                    for (std::uint32_t j = 0; j < numPolygons; ++j) {
                        const auto numRings = OFFSET_DELTA(partOffsets, partOffsetCounter);
                        partOffsetCounter++;

                        const auto numVertices = OFFSET_DELTA(ringOffsets, ringOffsetsCounter);
                        ringOffsetsCounter++;

                        plainTotalVertices += numVertices;

                        std::vector<CoordVec> rings;
                        rings.reserve(std::min<std::size_t>(numRings, ringOffsets.size()));

                        rings.push_back(getLineStringCoords(
                            vertexBuffer, vertexBufferOffset, numVertices, /*closeLineString=*/true));
                        vertexBufferOffset += 2 * std::size_t{numVertices};

                        for (std::uint32_t k = 1; k < numRings; ++k) {
                            const auto numRingVertices = OFFSET_DELTA(ringOffsets, ringOffsetsCounter);
                            ringOffsetsCounter++;

                            plainTotalVertices += numRingVertices;

                            rings.push_back(getLineStringCoords(
                                vertexBuffer, vertexBufferOffset, numRingVertices, /*closeLineString=*/true));
                            vertexBufferOffset += 2 * std::size_t{numRingVertices};
                        }

                        polygons.push_back(std::move(rings));
                    }

                    auto newGeometry = factory.createMultiPolygon(std::move(polygons));
                    applyTriangles(*newGeometry, triangleOffset, indexBufferOffset, plainTotalVertices, true);
                    geometries.push_back(std::move(newGeometry));
                } else {
                    [[maybe_unused]] std::uint32_t dictTotalVertices = 0;
                    for (std::uint32_t j = 0; j < numPolygons; ++j) {
                        const auto numRings = OFFSET_DELTA(partOffsets, partOffsetCounter);
                        partOffsetCounter++;

                        const auto numVertices = OFFSET_DELTA(ringOffsets, ringOffsetsCounter);
                        ringOffsetsCounter++;

                        dictTotalVertices += numVertices;

                        std::vector<CoordVec> rings;
                        rings.reserve(std::min<std::size_t>(numRings, ringOffsets.size()));

                        rings.push_back((vertexBufferType == VertexBufferType::VEC_2)
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
                                                                               /*closeLineString=*/true));
                        vertexOffsetsOffset += numVertices;

                        for (std::uint32_t k = 1; k < numRings; ++k) {
                            const auto numRingVertices = OFFSET_DELTA(ringOffsets, ringOffsetsCounter);
                            ringOffsetsCounter++;

                            dictTotalVertices += numRingVertices;

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

                        polygons.push_back(std::move(rings));
                    }

                    auto newGeometry = factory.createMultiPolygon(std::move(polygons));
                    applyTriangles(*newGeometry, triangleOffset, indexBufferOffset, dictTotalVertices, true);
                    geometries.push_back(std::move(newGeometry));
                }
                break;
            }
            default:
                throw std::runtime_error("Unsupported geometry type: " + std::to_string(std::to_underlying(geomType)));
        }
    }

    // If we didn't use all the input data, the tile isn't valid.
    if (indexBufferOffset != indexBuffer.size() || triangleOffset != triangleCounts.size()) {
        throw std::runtime_error("Unused triangle data");
    }

    // TODO: this fails for `test/expected/amazon_here/8_132_85.mlt`
    // if (topologyVector && !topologyVector->getPartOffsets().empty() &&
    //     partOffsetCounter != topologyVector->getPartOffsets().size()) {
    //     throw std::runtime_error("Unused part offsets");
    // }
    if (topologyVector &&
        ((!topologyVector->getRingOffsets().empty() && ringOffsetsCounter != topologyVector->getRingOffsets().size()) ||
         (!topologyVector->getGeometryOffsets().empty() &&
          geometryOffsetsCounter != topologyVector->getGeometryOffsets().size()))) {
        throw std::runtime_error("Unused topology data");
    }

    // If we're using vertex offsets, we should have used all
    // of them, and we won't have used vertex buffer offsets.
    if (vertexOffsets.empty() ? (vertexBufferOffset != vertexBuffer.size())
                              : (vertexOffsetsOffset != vertexOffsets.size())) {
        throw std::runtime_error("Unused vertex data");
    }

    return geometries;
}

} // namespace mlt::geometry

// NOLINTEND(bugprone-unchecked-optional-access)
