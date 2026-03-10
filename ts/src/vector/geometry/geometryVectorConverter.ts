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

    const containsPolygon = geometryVector.containsPolygonGeometry();
    const vertexBuffer = geometryVector.vertexBuffer;

    for (let i = 0; i < geometryVector.numGeometries; i++) {
        const geometryType = geometryVector.geometryType(i);
        switch (geometryType) {
            case GEOMETRY_TYPE.POINT:
                {
                    let x: number;
                    let y: number;
                    if (nonOffset) {
                        x = vertexBuffer[vertexBufferOffset++];
                        y = vertexBuffer[vertexBufferOffset++];
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
                        const offset = vertexOffsets[vertexOffsetsOffset++] * 2;
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
                    const points: Point[] = new Array(numPoints);
                    if (nonOffset) {
                        for (let j = 0; j < numPoints; j++) {
                            const x = vertexBuffer[vertexBufferOffset++];
                            const y = vertexBuffer[vertexBufferOffset++];
                            points[j] = new Point(x, y);
                        }
                    } else {
                        for (let j = 0; j < numPoints; j++) {
                            const offset = vertexOffsets[vertexOffsetsOffset++] * 2;
                            const x = vertexBuffer[offset];
                            const y = vertexBuffer[offset + 1];
                            points[j] = new Point(x, y);
                        }
                    }
                    geometries[geometryCounter++] = points.map((point) => [point]);
                    // MULTIPOINT must increment offset counters like POINT does
                    partOffsetCounter += numPoints;
                    ringOffsetsCounter += numPoints;
                }
                break;
            case GEOMETRY_TYPE.LINESTRING:
                {
                    let numVertices: number;
                    if (containsPolygon) {
                        numVertices = ringOffsets[ringOffsetsCounter] - ringOffsets[ringOffsetsCounter - 1];
                        ringOffsetsCounter++;
                    } else {
                        numVertices = partOffsets[partOffsetCounter] - partOffsets[partOffsetCounter - 1];
                    }
                    partOffsetCounter++;

                    let vertices: Point[];
                    if (nonOffset) {
                        vertices = getLineStringOrRing(vertexBuffer, vertexBufferOffset, numVertices, false);
                        vertexBufferOffset += numVertices * 2;
                    } else {
                        vertices = decodeDictionaryEncodedLineStringOrRing(
                            geometryVector.vertexBufferType,
                            vertexBuffer,
                            vertexOffsets,
                            vertexOffsetsOffset,
                            numVertices,
                            false,
                            mortonSettings,
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
                    const rings: CoordinatesArray = new Array(numRings - 1);
                    let shell: Point[];
                    let numVertices = ringOffsets[ringOffsetsCounter] - ringOffsets[ringOffsetsCounter - 1];
                    ringOffsetsCounter++;

                    if (nonOffset) {
                        shell = getLineStringOrRing(vertexBuffer, vertexBufferOffset, numVertices, true);
                        vertexBufferOffset += numVertices * 2;
                        for (let j = 0; j < rings.length; j++) {
                            numVertices = ringOffsets[ringOffsetsCounter] - ringOffsets[ringOffsetsCounter - 1];
                            ringOffsetsCounter++;
                            rings[j] = getLineStringOrRing(vertexBuffer, vertexBufferOffset, numVertices, true);
                            vertexBufferOffset += numVertices * 2;
                        }
                    } else {
                        shell = decodeDictionaryEncodedLineStringOrRing(
                            geometryVector.vertexBufferType,
                            vertexBuffer,
                            vertexOffsets,
                            vertexOffsetsOffset,
                            numVertices,
                            true,
                            mortonSettings,
                        );
                        vertexOffsetsOffset += numVertices;
                        for (let j = 0; j < rings.length; j++) {
                            numVertices = ringOffsets[ringOffsetsCounter] - ringOffsets[ringOffsetsCounter - 1];
                            ringOffsetsCounter++;
                            rings[j] = decodeDictionaryEncodedLineStringOrRing(
                                geometryVector.vertexBufferType,
                                vertexBuffer,
                                vertexOffsets,
                                vertexOffsetsOffset,
                                numVertices,
                                true,
                                mortonSettings,
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
                        if (containsPolygon) {
                            numVertices = ringOffsets[ringOffsetsCounter] - ringOffsets[ringOffsetsCounter - 1];
                            ringOffsetsCounter++;
                        } else {
                            numVertices = partOffsets[partOffsetCounter] - partOffsets[partOffsetCounter - 1];
                        }
                        partOffsetCounter++;
                        if (nonOffset) {
                            lineStrings[j] = getLineStringOrRing(vertexBuffer, vertexBufferOffset, numVertices, false);
                            vertexBufferOffset += numVertices * 2;
                        } else {
                            const vertices = decodeDictionaryEncodedLineStringOrRing(
                                geometryVector.vertexBufferType,
                                vertexBuffer,
                                vertexOffsets,
                                vertexOffsetsOffset,
                                numVertices,
                                false,
                                mortonSettings,
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
                        let shell: Point[];
                        const rings: CoordinatesArray = new Array(numRings - 1);
                        const numVertices = ringOffsets[ringOffsetsCounter] - ringOffsets[ringOffsetsCounter - 1];
                        ringOffsetsCounter++;
                        if (nonOffset) {
                            shell = getLineStringOrRing(vertexBuffer, vertexBufferOffset, numVertices, true);
                            vertexBufferOffset += numVertices * 2;
                        } else {
                            shell = decodeDictionaryEncodedLineStringOrRing(
                                geometryVector.vertexBufferType,
                                vertexBuffer,
                                vertexOffsets,
                                vertexOffsetsOffset,
                                numVertices,
                                true,
                                mortonSettings,
                            );
                            vertexOffsetsOffset += numVertices;
                        }
                        for (let k = 0; k < rings.length; k++) {
                            const numRingVertices =
                                ringOffsets[ringOffsetsCounter] - ringOffsets[ringOffsetsCounter - 1];
                            ringOffsetsCounter++;
                            if (nonOffset) {
                                rings[k] = getLineStringOrRing(vertexBuffer, vertexBufferOffset, numRingVertices, true);
                                vertexBufferOffset += numRingVertices * 2;
                            } else {
                                rings[k] = decodeDictionaryEncodedLineStringOrRing(
                                    geometryVector.vertexBufferType,
                                    vertexBuffer,
                                    vertexOffsets,
                                    vertexOffsetsOffset,
                                    numRingVertices,
                                    true,
                                    mortonSettings,
                                );
                                vertexOffsetsOffset += numRingVertices;
                            }
                        }
                        polygons[j] = [shell].concat(rings);
                    }
                    geometries[geometryCounter++] = polygons.flat();
                }
                break;
            default:
                throw new Error("The specified geometry type is currently not supported.");
        }
    }

    return geometries;
}

function decodeDictionaryEncodedLineStringOrRing(
    vertexBufferType: VertexBufferType,
    vertexBuffer: Int32Array | Uint32Array,
    vertexOffsets: Uint32Array,
    vertexOffset: number,
    numVertices: number,
    closeLineString: boolean,
    mortonSettings: MortonSettings,
): Point[] {
    if (vertexBufferType === VertexBufferType.MORTON) {
        return decodeMortonDictionaryEncodedLineString(
            vertexBuffer,
            vertexOffsets,
            vertexOffset,
            numVertices,
            closeLineString,
            mortonSettings,
        );
    } else {
        return decodeDictionaryEncodedLineString(
            vertexBuffer,
            vertexOffsets,
            vertexOffset,
            numVertices,
            closeLineString,
        );
    }
}

function getLineStringOrRing(
    vertexBuffer: Int32Array | Uint32Array,
    startIndex: number,
    numVertices: number,
    closeLineString: boolean,
): Point[] {
    const vertices: Point[] = new Array(closeLineString ? numVertices + 1 : numVertices);
    for (let i = 0; i < numVertices * 2; i += 2) {
        const x = vertexBuffer[startIndex + i];
        const y = vertexBuffer[startIndex + i + 1];
        vertices[i / 2] = new Point(x, y);
    }

    if (closeLineString) {
        vertices[vertices.length - 1] = vertices[0];
    }
    return vertices;
}

function decodeDictionaryEncodedLineString(
    vertexBuffer: Int32Array | Uint32Array,
    vertexOffsets: Uint32Array,
    vertexOffset: number,
    numVertices: number,
    closeLineString: boolean,
): Point[] {
    const vertices: Point[] = new Array(closeLineString ? numVertices + 1 : numVertices);
    for (let i = 0; i < numVertices * 2; i += 2) {
        const offset = vertexOffsets[vertexOffset + i / 2] * 2;
        const x = vertexBuffer[offset];
        const y = vertexBuffer[offset + 1];
        vertices[i / 2] = new Point(x, y);
    }

    if (closeLineString) {
        vertices[vertices.length - 1] = vertices[0];
    }
    return vertices;
}

function decodeMortonDictionaryEncodedLineString(
    vertexBuffer: Int32Array | Uint32Array,
    vertexOffsets: Uint32Array,
    vertexOffset: number,
    numVertices: number,
    closeLineString: boolean,
    mortonSettings: MortonSettings,
): Point[] {
    const vertices: Point[] = new Array(closeLineString ? numVertices + 1 : numVertices);
    for (let i = 0; i < numVertices; i++) {
        const offset = vertexOffsets[vertexOffset + i];
        const mortonEncodedVertex = vertexBuffer[offset];
        const vertex = decodeZOrderCurve(mortonEncodedVertex, mortonSettings.numBits, mortonSettings.coordinateShift);
        vertices[i] = new Point(vertex.x, vertex.y);
    }
    if (closeLineString) {
        vertices[vertices.length - 1] = vertices[0];
    }

    return vertices;
}
