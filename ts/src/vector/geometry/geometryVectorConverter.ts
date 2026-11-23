import type { GeometryVector, MortonSettings, CoordinatesArray } from "./geometryVector";
import { decodeZOrderCurve } from "./zOrderCurve";
import { GEOMETRY_TYPE } from "./geometryType";
import { VertexBufferType } from "./vertexBufferType";
import Point from "@mapbox/point-geometry";

export function convertGeometryVector(geometryVector: GeometryVector): CoordinatesArray[] {
    const geometries: CoordinatesArray[] = new Array(geometryVector.numGeometries);
    let partOffsetCounter = 1;
    let ringOffsetsCounter = 1;
    let geometryOffsetsCounter = 1;
    let geometryCounter = 0;
    let vertexBufferOffset = 0;
    let vertexOffsetsOffset = 0;

    const mortonSettings = geometryVector.mortonSettings;
    const topologyVector = geometryVector.topologyVector;
    const geometryOffsets = topologyVector.geometryOffsets;
    const partOffsets = topologyVector.partOffsets;
    const ringOffsets = topologyVector.ringOffsets;
    const vertexOffsets = geometryVector.vertexOffsets;
    const nonOffset = !vertexOffsets || vertexOffsets.length === 0;

    const hasRingOffsets = Boolean(ringOffsets && ringOffsets.length > 0);
    const stride = geometryVector.vertexBufferType === VertexBufferType.VEC_3 ? 3 : 2;
    const vertexBuffer = geometryVector.vertexBuffer;

    for (let i = 0; i < geometryVector.numGeometries; i++) {
        const geometryType = geometryVector.geometryType(i);
        switch (geometryType) {
            case GEOMETRY_TYPE.POINT:
                {
                    let x: number;
                    let y: number;
                    if (nonOffset) {
                        x = vertexBuffer[vertexBufferOffset];
                        y = vertexBuffer[vertexBufferOffset + 1];
                        vertexBufferOffset += stride;
                    } else if (geometryVector.vertexBufferType === VertexBufferType.MORTON) {
                        const offset = vertexOffsets[vertexOffsetsOffset++];
                        const mortonCode = vertexBuffer[offset];
                        const vertex = decodeZOrderCurve(
                            mortonCode,
                            mortonSettings.numBits,
                            mortonSettings.coordinateShift,
                        );
                        x = vertex.x;
                        y = vertex.y;
                    } else {
                        const offset = vertexOffsets[vertexOffsetsOffset++] * stride;
                        x = vertexBuffer[offset];
                        y = vertexBuffer[offset + 1];
                    }
                    geometries[geometryCounter++] = [[new Point(x, y)]];
                    if (geometryOffsets) geometryOffsetsCounter++;
                    if (partOffsets) partOffsetCounter++;
                    if (ringOffsets) ringOffsetsCounter++;
                }
                break;
            case GEOMETRY_TYPE.MULTIPOINT:
                {
                    const numPoints =
                        geometryOffsets[geometryOffsetsCounter] - geometryOffsets[geometryOffsetsCounter - 1];
                    geometryOffsetsCounter++;
                    let points: Point[];
                    if (nonOffset) {
                        points = new Array(numPoints);
                        for (let j = 0; j < numPoints; j++) {
                            const x = vertexBuffer[vertexBufferOffset];
                            const y = vertexBuffer[vertexBufferOffset + 1];
                            vertexBufferOffset += stride;
                            points[j] = new Point(x, y);
                        }
                    } else {
                        points = decodeDictionaryEncodedVertices(
                            geometryVector.vertexBufferType,
                            vertexBuffer,
                            vertexOffsets,
                            vertexOffsetsOffset,
                            numPoints,
                            false,
                            mortonSettings,
                            stride,
                        );
                        vertexOffsetsOffset += numPoints;
                    }
                    geometries[geometryCounter++] = points.map((point) => [point]);
                    if (partOffsets) partOffsetCounter += numPoints;
                    if (ringOffsets) ringOffsetsCounter += numPoints;
                }
                break;
            case GEOMETRY_TYPE.LINESTRING:
                {
                    let numVertices: number;
                    if (hasRingOffsets) {
                        numVertices = ringOffsets[ringOffsetsCounter] - ringOffsets[ringOffsetsCounter - 1];
                        ringOffsetsCounter++;
                    } else {
                        numVertices = partOffsets[partOffsetCounter] - partOffsets[partOffsetCounter - 1];
                    }
                    if (partOffsets) partOffsetCounter++;

                    let vertices: Point[];
                    if (nonOffset) {
                        vertices = getLineStringOrRing(vertexBuffer, vertexBufferOffset, numVertices, false, stride);
                        vertexBufferOffset += numVertices * stride;
                    } else {
                        vertices = decodeDictionaryEncodedVertices(
                            geometryVector.vertexBufferType,
                            vertexBuffer,
                            vertexOffsets,
                            vertexOffsetsOffset,
                            numVertices,
                            false,
                            mortonSettings,
                            stride,
                        );
                        vertexOffsetsOffset += numVertices;
                    }

                    geometries[geometryCounter++] = [vertices];

                    if (geometryOffsets) geometryOffsetsCounter++;
                }
                break;
            case GEOMETRY_TYPE.POLYGON:
                {
                    const numRings = partOffsets[partOffsetCounter] - partOffsets[partOffsetCounter - 1];
                    partOffsetCounter++;
                    if (numRings === 0) {
                        geometries[geometryCounter++] = [[]];
                        if (geometryOffsets) geometryOffsetsCounter++;
                        break;
                    }
                    const rings: CoordinatesArray = new Array(numRings - 1);
                    let shell: Point[];
                    let numVertices = ringOffsets[ringOffsetsCounter] - ringOffsets[ringOffsetsCounter - 1];
                    ringOffsetsCounter++;

                    if (nonOffset) {
                        shell = getLineStringOrRing(vertexBuffer, vertexBufferOffset, numVertices, true, stride);
                        vertexBufferOffset += numVertices * stride;
                        for (let j = 0; j < rings.length; j++) {
                            numVertices = ringOffsets[ringOffsetsCounter] - ringOffsets[ringOffsetsCounter - 1];
                            ringOffsetsCounter++;
                            rings[j] = getLineStringOrRing(vertexBuffer, vertexBufferOffset, numVertices, true, stride);
                            vertexBufferOffset += numVertices * stride;
                        }
                    } else {
                        shell = decodeDictionaryEncodedVertices(
                            geometryVector.vertexBufferType,
                            vertexBuffer,
                            vertexOffsets,
                            vertexOffsetsOffset,
                            numVertices,
                            true,
                            mortonSettings,
                            stride,
                        );
                        vertexOffsetsOffset += numVertices;
                        for (let j = 0; j < rings.length; j++) {
                            numVertices = ringOffsets[ringOffsetsCounter] - ringOffsets[ringOffsetsCounter - 1];
                            ringOffsetsCounter++;
                            rings[j] = decodeDictionaryEncodedVertices(
                                geometryVector.vertexBufferType,
                                vertexBuffer,
                                vertexOffsets,
                                vertexOffsetsOffset,
                                numVertices,
                                true,
                                mortonSettings,
                                stride,
                            );
                            vertexOffsetsOffset += numVertices;
                        }
                    }
                    geometries[geometryCounter++] = [shell].concat(rings);
                    if (geometryOffsets) geometryOffsetsCounter++;
                }
                break;
            case GEOMETRY_TYPE.MULTILINESTRING:
                {
                    const numLineStrings =
                        geometryOffsets[geometryOffsetsCounter] - geometryOffsets[geometryOffsetsCounter - 1];
                    geometryOffsetsCounter++;
                    const lineStrings: CoordinatesArray = new Array(numLineStrings);
                    for (let j = 0; j < numLineStrings; j++) {
                        let numVertices: number;
                        if (hasRingOffsets) {
                            numVertices = ringOffsets[ringOffsetsCounter] - ringOffsets[ringOffsetsCounter - 1];
                            ringOffsetsCounter++;
                        } else {
                            numVertices = partOffsets[partOffsetCounter] - partOffsets[partOffsetCounter - 1];
                        }
                        if (partOffsets) partOffsetCounter++;
                        if (nonOffset) {
                            lineStrings[j] = getLineStringOrRing(
                                vertexBuffer,
                                vertexBufferOffset,
                                numVertices,
                                false,
                                stride,
                            );
                            vertexBufferOffset += numVertices * stride;
                        } else {
                            const vertices = decodeDictionaryEncodedVertices(
                                geometryVector.vertexBufferType,
                                vertexBuffer,
                                vertexOffsets,
                                vertexOffsetsOffset,
                                numVertices,
                                false,
                                mortonSettings,
                                stride,
                            );
                            lineStrings[j] = vertices;
                            vertexOffsetsOffset += numVertices;
                        }
                    }
                    geometries[geometryCounter++] = lineStrings;
                }
                break;
            case GEOMETRY_TYPE.MULTIPOLYGON:
                {
                    const numPolygons =
                        geometryOffsets[geometryOffsetsCounter] - geometryOffsets[geometryOffsetsCounter - 1];
                    geometryOffsetsCounter++;
                    const polygons: CoordinatesArray[] = new Array(numPolygons);
                    for (let j = 0; j < numPolygons; j++) {
                        const numRings = partOffsets[partOffsetCounter] - partOffsets[partOffsetCounter - 1];
                        partOffsetCounter++;
                        if (numRings === 0) {
                            polygons[j] = [[]];
                            continue;
                        }
                        let shell: Point[];
                        const rings: CoordinatesArray = new Array(numRings - 1);
                        const numVertices = ringOffsets[ringOffsetsCounter] - ringOffsets[ringOffsetsCounter - 1];
                        ringOffsetsCounter++;
                        if (nonOffset) {
                            shell = getLineStringOrRing(vertexBuffer, vertexBufferOffset, numVertices, true, stride);
                            vertexBufferOffset += numVertices * stride;
                        } else {
                            shell = decodeDictionaryEncodedVertices(
                                geometryVector.vertexBufferType,
                                vertexBuffer,
                                vertexOffsets,
                                vertexOffsetsOffset,
                                numVertices,
                                true,
                                mortonSettings,
                                stride,
                            );
                            vertexOffsetsOffset += numVertices;
                        }
                        for (let k = 0; k < rings.length; k++) {
                            const numRingVertices =
                                ringOffsets[ringOffsetsCounter] - ringOffsets[ringOffsetsCounter - 1];
                            ringOffsetsCounter++;
                            if (nonOffset) {
                                rings[k] = getLineStringOrRing(
                                    vertexBuffer,
                                    vertexBufferOffset,
                                    numRingVertices,
                                    true,
                                    stride,
                                );
                                vertexBufferOffset += numRingVertices * stride;
                            } else {
                                rings[k] = decodeDictionaryEncodedVertices(
                                    geometryVector.vertexBufferType,
                                    vertexBuffer,
                                    vertexOffsets,
                                    vertexOffsetsOffset,
                                    numRingVertices,
                                    true,
                                    mortonSettings,
                                    stride,
                                );
                                vertexOffsetsOffset += numRingVertices;
                            }
                        }
                        polygons[j] = [shell].concat(rings);
                    }
                    geometries[geometryCounter++] = polygons.flat();
                }
                break;
            case GEOMETRY_TYPE.MESH:
                {
                    const numVertices = geometryOffsets
                        ? geometryOffsets[geometryOffsetsCounter] - geometryOffsets[geometryOffsetsCounter - 1]
                        : 0;
                    if (geometryOffsets) geometryOffsetsCounter++;
                    const points: Point[] = new Array(numVertices);
                    for (let j = 0; j < numVertices; j++) {
                        const x = vertexBuffer[vertexBufferOffset];
                        const y = vertexBuffer[vertexBufferOffset + 1];
                        vertexBufferOffset += stride;
                        points[j] = new Point(x, y);
                    }
                    geometries[geometryCounter++] = points.map((point) => [point]);
                }
                break;
            default:
                throw new Error(`The specified geometry type (${geometryType}) is currently not supported.`);
        }
    }

    return geometries;
}

function decodeDictionaryEncodedVertices(
    vertexBufferType: VertexBufferType,
    vertexBuffer: Int32Array | Uint32Array,
    vertexOffsets: Uint32Array,
    vertexOffset: number,
    numVertices: number,
    isRing: boolean,
    mortonSettings: MortonSettings,
    stride = 2,
): Point[] {
    if (vertexBufferType === VertexBufferType.MORTON) {
        return decodeMortonDictionaryEncodedVertices(
            vertexBuffer,
            vertexOffsets,
            vertexOffset,
            numVertices,
            isRing,
            mortonSettings,
        );
    } else {
        return decodeVec2DictionaryEncodedVertices(
            vertexBuffer,
            vertexOffsets,
            vertexOffset,
            numVertices,
            isRing,
            stride,
        );
    }
}

function getLineStringOrRing(
    vertexBuffer: Int32Array | Uint32Array,
    startIndex: number,
    numVertices: number,
    isRing: boolean,
    stride = 2,
): Point[] {
    if (numVertices === 0) return [];
    const vertices: Point[] = new Array(isRing ? numVertices + 1 : numVertices);
    for (let i = 0; i < numVertices; i++) {
        const x = vertexBuffer[startIndex + i * stride];
        const y = vertexBuffer[startIndex + i * stride + 1];
        vertices[i] = new Point(x, y);
    }

    if (isRing) {
        vertices[vertices.length - 1] = vertices[0];
    }
    return vertices;
}

function decodeVec2DictionaryEncodedVertices(
    vertexBuffer: Int32Array | Uint32Array,
    vertexOffsets: Uint32Array,
    vertexOffset: number,
    numVertices: number,
    isRing: boolean,
    stride = 2,
): Point[] {
    if (numVertices === 0) return [];
    const vertices: Point[] = new Array(isRing ? numVertices + 1 : numVertices);
    for (let i = 0; i < numVertices; i++) {
        const offset = vertexOffsets[vertexOffset + i] * stride;
        const x = vertexBuffer[offset];
        const y = vertexBuffer[offset + 1];
        vertices[i] = new Point(x, y);
    }

    if (isRing) {
        vertices[vertices.length - 1] = vertices[0];
    }
    return vertices;
}

function decodeMortonDictionaryEncodedVertices(
    vertexBuffer: Int32Array | Uint32Array,
    vertexOffsets: Uint32Array,
    vertexOffset: number,
    numVertices: number,
    isRing: boolean,
    mortonSettings: MortonSettings,
): Point[] {
    if (numVertices === 0) return [];
    const vertices: Point[] = new Array(isRing ? numVertices + 1 : numVertices);
    for (let i = 0; i < numVertices; i++) {
        const offset = vertexOffsets[vertexOffset + i];
        const mortonEncodedVertex = vertexBuffer[offset];
        const vertex = decodeZOrderCurve(mortonEncodedVertex, mortonSettings.numBits, mortonSettings.coordinateShift);
        vertices[i] = new Point(vertex.x, vertex.y);
    }
    if (isRing) {
        vertices[vertices.length - 1] = vertices[0];
    }

    return vertices;
}
