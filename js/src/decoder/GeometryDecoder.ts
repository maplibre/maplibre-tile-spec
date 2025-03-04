import { PhysicalStreamType } from '../metadata/stream/PhysicalStreamType';
import { DictionaryType } from '../metadata/stream/DictionaryType';
import { LengthType } from '../metadata/stream/LengthType';
import { MortonEncodedStreamMetadata } from '../metadata/stream/MortonEncodedStreamMetadata';
import { IntegerDecoder } from './IntegerDecoder';
import { IntWrapper } from './IntWrapper';
import { StreamMetadataDecoder } from '../metadata/stream/StreamMetadataDecoder';
import { PhysicalLevelTechnique } from '../metadata/stream/PhysicalLevelTechnique';
import { GeometryFactory, Coordinate, LineString, Polygon, LinearRing } from '../data/Geometry';
import Point = require("@mapbox/point-geometry");

export enum GeometryType {
    POINT,
    LINESTRING,
    POLYGON,
    MULTIPOINT,
    MULTILINESTRING,
    MULTIPOLYGON
}

const geometryFactory = new GeometryFactory();


export class GeometryDecoder {
    public static decodeGeometryColumn(tile: Uint8Array, numStreams: number, offset: IntWrapper): GeometryColumn {
        const geometryTypeMetadata = StreamMetadataDecoder.decode(tile, offset);
        const geometryTypes = IntegerDecoder.decodeIntStream(tile, offset, geometryTypeMetadata, false);
        let numGeometries = null;
        let numParts = null;
        let numRings = null;
        let vertexOffsets = null;
        let vertexList = [];
        for(let i = 0; i < numStreams - 1; i++) {
            const geometryStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
            const physicalStreamType = geometryStreamMetadata.physicalStreamType();
            switch (physicalStreamType) {
                case PhysicalStreamType.LENGTH: {
                    switch (geometryStreamMetadata.logicalStreamType().lengthType()){
                        case LengthType.GEOMETRIES:
                            numGeometries = IntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
                            break;
                        case LengthType.PARTS:
                            numParts = IntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
                            break;
                        case LengthType.RINGS:
                            numRings = IntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
                            break;
                        case LengthType.TRIANGLES:
                            throw new Error("Not implemented yet.");
                    }
                    break;
                }
                case PhysicalStreamType.OFFSET: {
                    vertexOffsets = IntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
                    break;
                }
                case PhysicalStreamType.DATA: {
                    if(DictionaryType.VERTEX === geometryStreamMetadata.logicalStreamType().dictionaryType()){
                        if(geometryStreamMetadata.physicalLevelTechnique() == PhysicalLevelTechnique.FAST_PFOR){
                            throw new Error("FastPfor encoding for geometries is not yet supported.");
                            // vertexBuffer = DecodingUtils.decodeFastPfor128DeltaCoordinates(tile, geometryStreamMetadata.numValues(),
                            // geometryStreamMetadata.byteLength(), offset);
                            // offset.set(offset.get() + geometryStreamMetadata.byteLength());
                        } else {
                            vertexList = IntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, true);
                        }
                    }
                    else {
                        vertexList = IntegerDecoder.decodeMortonStream(tile, offset, geometryStreamMetadata as MortonEncodedStreamMetadata);
                    }
                    break;
                }
            }
        }

        return new GeometryColumn( geometryTypes, numGeometries, numParts, numRings, vertexOffsets, vertexList );
    }

    static decodeGeometry(geometryColumn: GeometryColumn) {
        const geometries = new Array(geometryColumn.geometryTypes.length);
        let partOffsetCounter = 0;
        let ringOffsetsCounter = 0;
        let geometryOffsetsCounter = 0;
        let geometryCounter = 0;
        const geometryFactory = new GeometryFactory();
        let vertexBufferOffset = 0;
        let vertexOffsetsOffset = 0;

        const geometryTypes = geometryColumn.geometryTypes;
        const geometryOffsets = geometryColumn.numGeometries;
        const partOffsets = geometryColumn.numParts;
        const ringOffsets = geometryColumn.numRings;
        const vertexOffsets = geometryColumn.vertexOffsets ? geometryColumn.vertexOffsets.map(i => i) : null;
        if (geometryColumn.vertexList.length === 0) {
            console.log("Warning: Vertex list is empty, skipping geometry decoding.");
            return [];
        }
        const vertexBuffer = geometryColumn.vertexList.map(i => i);

        const containsPolygon = geometryTypes.includes(
                GeometryType.POLYGON) || geometryTypes.includes(GeometryType.MULTIPOLYGON);
        for (const geometryTypeNum of geometryTypes) {
            const geometryType = geometryTypeNum as GeometryType;
            if (geometryType === GeometryType.POINT) {
                if (!vertexOffsets || vertexOffsets.length === 0) {
                    const x = vertexBuffer[vertexBufferOffset++];
                    const y = vertexBuffer[vertexBufferOffset++];
                    const coordinate = new Coordinate(x, y);
                    geometries[geometryCounter++] = geometryFactory.createPoint(coordinate);
                } else {
                    const offset = vertexOffsets[vertexOffsetsOffset++] * 2;
                    const x = vertexBuffer[offset];
                    const y = vertexBuffer[offset + 1];
                    const coordinate = new Coordinate(x, y);
                    geometries[geometryCounter++] = geometryFactory.createPoint(coordinate);
                }
            } else if (geometryType === GeometryType.MULTIPOINT) {
                const numPoints = geometryOffsets[geometryOffsetsCounter++];
                const points: Point[] = new Array(numPoints);
                if (!vertexOffsets || vertexOffsets.length === 0) {
                    for (let i = 0; i < numPoints; i++) {
                        const x = vertexBuffer[vertexBufferOffset++];
                        const y = vertexBuffer[vertexBufferOffset++];
                        const coordinate = new Coordinate(x, y);
                        points[i] = geometryFactory.createPoint(coordinate);
                    }
                    geometries[geometryCounter++] = geometryFactory.createMultiPoint(points);
                } else {
                    for (let i = 0; i < numPoints; i++) {
                        const offset = vertexOffsets[vertexOffsetsOffset++] * 2;
                        const x = vertexBuffer[offset];
                        const y = vertexBuffer[offset + 1];
                        const coordinate = new Coordinate(x, y);
                        points[i] = geometryFactory.createPoint(coordinate);
                    }
                    geometries[geometryCounter++] = geometryFactory.createMultiPoint(points);
                }
            } else if (geometryType === GeometryType.LINESTRING) {
                const numVertices = containsPolygon
                    ? ringOffsets[ringOffsetsCounter++]
                    : partOffsets[partOffsetCounter++];
                if (!vertexOffsets || vertexOffsets.length === 0) {
                    const vertices = this.getLineString(vertexBuffer, vertexBufferOffset, numVertices, false);
                    vertexBufferOffset += numVertices * 2;
                    geometries[geometryCounter++] = geometryFactory.createLineString(vertices);
                } else {
                    const vertices = this.decodeDictionaryEncodedLineString(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, false);
                    vertexOffsetsOffset += numVertices;
                    geometries[geometryCounter++] = geometryFactory.createLineString(vertices);
                }
            } else if (geometryType === GeometryType.POLYGON) {
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
            } else if (geometryType === GeometryType.MULTILINESTRING) {
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
            } else if (geometryType === GeometryType.MULTIPOLYGON) {
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
            } else {
                throw new Error("The specified geometry type is currently not supported: " + geometryTypeNum);
            }
        }

        return geometries;
    }

    private static getLinearRing(vertexBuffer: number[], startIndex: number, numVertices: number, geometryFactory: GeometryFactory): LinearRing {
        const linearRing = this.getLineString(vertexBuffer, startIndex, numVertices, true);
        return geometryFactory.createLinearRing(linearRing);
    }

    private static decodeDictionaryEncodedLinearRing(vertexBuffer: number[], vertexOffsets: number[], vertexOffset: number, numVertices: number, geometryFactory: GeometryFactory): LinearRing {
        const linearRing = this.decodeDictionaryEncodedLineString(vertexBuffer, vertexOffsets, vertexOffset, numVertices, true);
        return geometryFactory.createLinearRing(linearRing);
    }

    private static getLineString(vertexBuffer: number[], startIndex: number, numVertices: number, closeLineString: boolean): Coordinate[] {
        const vertices: Coordinate[] = new Array(closeLineString ? numVertices + 1 : numVertices);
        for (let i = 0; i < numVertices * 2; i += 2) {
            const x = vertexBuffer[startIndex + i];
            const y = vertexBuffer[startIndex + i + 1];
            vertices[i / 2] = new Coordinate(x, y);
        }

        if (closeLineString) {
            vertices[vertices.length - 1] = vertices[0];
        }
        return vertices;
    }

    private static decodeDictionaryEncodedLineString(vertexBuffer: number[], vertexOffsets: number[], vertexOffset: number, numVertices: number, closeLineString: boolean): Coordinate[] {
        const vertices: Coordinate[] = new Array(closeLineString ? numVertices + 1 : numVertices);
        for (let i = 0; i < numVertices * 2; i += 2) {
            const offset = vertexOffsets[vertexOffset + i / 2] * 2;
            const x = vertexBuffer[offset];
            const y = vertexBuffer[offset + 1];
            vertices[i / 2] = new Coordinate(x, y);
        }

        if (closeLineString) {
            vertices[vertices.length - 1] = vertices[0];
        }
        return vertices;
    }

}



export class GeometryColumn {
    geometryTypes: number[];
    numGeometries: number[];
    numParts: number[];
    numRings: number[];
    vertexOffsets: number[];
    vertexList: number[];
    constructor(geometryTypes: number[], numGeometries: number[], numParts: number[], numRings: number[], vertexOffsets: number[], vertexList: number[]) {
        this.geometryTypes = geometryTypes;
        this.numGeometries = numGeometries;
        this.numParts = numParts;
        this.numRings = numRings;
        this.vertexOffsets = vertexOffsets;
        this.vertexList = vertexList;
    }
}
