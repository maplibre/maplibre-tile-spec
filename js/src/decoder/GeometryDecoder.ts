import { PhysicalStreamType } from '../metadata/stream/PhysicalStreamType';
import { DictionaryType } from '../metadata/stream/DictionaryType';
import { LengthType } from '../metadata/stream/LengthType';
import { MortonEncodedStreamMetadata } from '../metadata/stream/MortonEncodedStreamMetadata';
import { IntegerDecoder } from './IntegerDecoder';
import { IntWrapper } from './IntWrapper';
import { StreamMetadataDecoder } from '../metadata/stream/StreamMetadataDecoder';
import { PhysicalLevelTechnique } from '../metadata/stream/PhysicalLevelTechnique';
import { GeometryFactory, LineString, Polygon, LinearRing } from '../data/Geometry';
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


export class GeometryColumn {
    numGeometries: number[];
    numParts: number[];
    numRings: number[];
    vertexOffsets: number[];
    vertexList: number[];
    constructor(numGeometries: number[], numParts: number[], numRings: number[], vertexOffsets: number[], vertexList: number[]) {
        this.numGeometries = numGeometries;
        this.numParts = numParts;
        this.numRings = numRings;
        this.vertexOffsets = vertexOffsets;
        this.vertexList = vertexList;
    }
}

class GeometryCounters {
    partCounter: IntWrapper;
    ringCounter: IntWrapper;
    geometryCounter: IntWrapper;
    vertexBufferCounter: IntWrapper;
    vertexOffsetsCounter: IntWrapper;
    constructor() {
        this.partCounter = new IntWrapper(0);
        this.ringCounter = new IntWrapper(0);
        this.geometryCounter = new IntWrapper(0);
        this.vertexBufferCounter = new IntWrapper(0);
        this.vertexOffsetsCounter = new IntWrapper(0);
    }
}

export class GeometryDecoder {
    public static decodeGeometryColumn(tile: Uint8Array, numStreams: number, offset: IntWrapper): GeometryColumn {
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

        return new GeometryColumn( numGeometries, numParts, numRings, vertexOffsets, vertexList );
    }

    static decodeGeometry(geometryType : GeometryType,
                containsPolygon : boolean,
                geometryCounter : GeometryCounters,
                geometryColumn: GeometryColumn) {
        const geometryOffsets = geometryColumn.numGeometries;
        const partOffsets = geometryColumn.numParts;
        const ringOffsets = geometryColumn.numRings;
        const vertexBufferOffset = geometryCounter.vertexBufferCounter;
        const partOffsetCounter = geometryCounter.partCounter;
        const ringOffsetsCounter = geometryCounter.ringCounter;
        const geometryOffsetsCounter = geometryCounter.geometryCounter;
        const vertexOffsetsOffset = geometryCounter.vertexOffsetsCounter;
        // const vertexOffsets = geometryColumn.vertexOffsets ? geometryColumn.vertexOffsets.map(i => i) : [];
        const vertexOffsets = geometryColumn.vertexOffsets;
        // if (geometryColumn.vertexList.length === 0) {
        //     console.log("Warning: Vertex list is empty, skipping geometry decoding.");
        //     return [];
        // }
        // const vertexBuffer = geometryColumn.vertexList.map(i => i);
        const vertexBuffer = geometryColumn.vertexList;

        if (geometryType === GeometryType.POINT) {
            if (!vertexOffsets || vertexOffsets.length === 0) {
                const x = vertexBuffer[vertexBufferOffset.increment()];
                const y = vertexBuffer[vertexBufferOffset.increment()];
                return geometryFactory.createPoint(x, y);
            } else {
                const offset = vertexOffsets[vertexOffsetsOffset.increment()] * 2;
                const x = vertexBuffer[offset];
                const y = vertexBuffer[offset + 1];
                return geometryFactory.createPoint(x, y);
            }
        } else if (geometryType === GeometryType.MULTIPOINT) {
            const numPoints = geometryOffsets[geometryOffsetsCounter.increment()];
            const points: Point[] = new Array(numPoints);
            if (!vertexOffsets || vertexOffsets.length === 0) {
                for (let i = 0; i < numPoints; i++) {
                    const x = vertexBuffer[vertexBufferOffset.increment()];
                    const y = vertexBuffer[vertexBufferOffset.increment()];
                    points[i] = geometryFactory.createPoint(x, y);
                }
                return geometryFactory.createMultiPoint(points);
            } else {
                for (let i = 0; i < numPoints; i++) {
                    const offset = vertexOffsets[vertexOffsetsOffset.increment()] * 2;
                    const x = vertexBuffer[offset];
                    const y = vertexBuffer[offset + 1];
                    points[i] = geometryFactory.createPoint(x, y);
                }
                return geometryFactory.createMultiPoint(points);
            }
        } else if (geometryType === GeometryType.LINESTRING) {
            const numVertices = containsPolygon
                ? ringOffsets[ringOffsetsCounter.increment()]
                : partOffsets[partOffsetCounter.increment()];
            if (!vertexOffsets || vertexOffsets.length === 0) {
                const vertices = this.getLineString(vertexBuffer, vertexBufferOffset, numVertices, false);
                vertexBufferOffset.add(numVertices * 2);
                return geometryFactory.createLineString(vertices);
            } else {
                const vertices = this.decodeDictionaryEncodedLineString(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, false);
                vertexOffsetsOffset.add(numVertices);
                return geometryFactory.createLineString(vertices);
            }
        } else if (geometryType === GeometryType.POLYGON) {
            const numRings = partOffsets[partOffsetCounter.increment()];
            const rings: LinearRing[] = new Array(numRings - 1);
            let numVertices = ringOffsets[ringOffsetsCounter.increment()];
            if (!vertexOffsets || vertexOffsets.length === 0) {
                const shell = this.getLinearRing(vertexBuffer, vertexBufferOffset, numVertices, geometryFactory);
                vertexBufferOffset.add(numVertices * 2);
                for (let i = 0; i < rings.length; i++) {
                    numVertices = ringOffsets[ringOffsetsCounter.increment()];
                    rings[i] = this.getLinearRing(vertexBuffer, vertexBufferOffset, numVertices, geometryFactory);
                    vertexBufferOffset.add(numVertices * 2);
                }
                return geometryFactory.createPolygon(shell, rings);
            } else {
                const shell = this.decodeDictionaryEncodedLinearRing(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, geometryFactory);
                vertexOffsetsOffset.add(numVertices);
                for (let i = 0; i < rings.length; i++) {
                    numVertices = ringOffsets[ringOffsetsCounter.increment()];
                    rings[i] = this.decodeDictionaryEncodedLinearRing(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, geometryFactory);
                    vertexOffsetsOffset.add(numVertices);
                }
                return geometryFactory.createPolygon(shell, rings);
            }
        } else if (geometryType === GeometryType.MULTILINESTRING) {
            const numLineStrings = geometryOffsets[geometryOffsetsCounter.increment()];
            const lineStrings: LineString[] = new Array(numLineStrings);
            if (!vertexOffsets || vertexOffsets.length === 0) {
                for (let i = 0; i < numLineStrings; i++) {
                    const numVertices = containsPolygon
                        ? ringOffsets[ringOffsetsCounter.increment()] : partOffsets[partOffsetCounter.increment()];
                    const vertices = this.getLineString(vertexBuffer, vertexBufferOffset, numVertices, false);
                    lineStrings[i] = geometryFactory.createLineString(vertices);
                    vertexBufferOffset.add(numVertices * 2);
                }
                return geometryFactory.createMultiLineString(lineStrings);
            } else {
                for (let i = 0; i < numLineStrings; i++) {
                    const numVertices = containsPolygon
                        ? ringOffsets[ringOffsetsCounter.increment()] : partOffsets[partOffsetCounter.increment()];
                    const vertices = this.decodeDictionaryEncodedLineString(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, false);
                    lineStrings[i] = geometryFactory.createLineString(vertices);
                    vertexOffsetsOffset.add(numVertices);
                }
                return geometryFactory.createMultiLineString(lineStrings);
            }
        } else if (geometryType === GeometryType.MULTIPOLYGON) {
            const numPolygons = geometryOffsets[geometryOffsetsCounter.increment()];
            const polygons: Polygon[] = new Array(numPolygons);
            if (!vertexOffsets || vertexOffsets.length === 0) {
                for (let i = 0; i < numPolygons; i++) {
                    const numRings = partOffsets[partOffsetCounter.increment()];
                    const rings: LinearRing[] = new Array(numRings - 1);
                    const numVertices = ringOffsets[ringOffsetsCounter.increment()];
                    const shell = this.getLinearRing(vertexBuffer, vertexBufferOffset, numVertices, geometryFactory);
                    vertexBufferOffset.add(numVertices * 2);
                    for (let j = 0; j < rings.length; j++) {
                        const numRingVertices = ringOffsets[ringOffsetsCounter.increment()];
                        rings[j] = this.getLinearRing(vertexBuffer, vertexBufferOffset, numRingVertices, geometryFactory);
                        vertexBufferOffset.add(numRingVertices * 2);
                    }
                    polygons[i] = geometryFactory.createPolygon(shell, rings);
                }
                return geometryFactory.createMultiPolygon(polygons);
            } else {
                for (let i = 0; i < numPolygons; i++) {
                    const numRings = partOffsets[partOffsetCounter.increment()];
                    const rings: LinearRing[] = new Array(numRings - 1);
                    const numVertices = ringOffsets[ringOffsetsCounter.increment()];
                    const shell = this.decodeDictionaryEncodedLinearRing(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, geometryFactory);
                    vertexOffsetsOffset.add(numVertices);
                    for (let j = 0; j < rings.length; j++) {
                        const numRingVertices = ringOffsets[ringOffsetsCounter.increment()];
                        rings[j] = this.decodeDictionaryEncodedLinearRing(vertexBuffer, vertexOffsets, vertexOffsetsOffset, numVertices, geometryFactory);
                        vertexOffsetsOffset.add(numRingVertices);
                    }
                    polygons[i] = geometryFactory.createPolygon(shell, rings);
                }
                return geometryFactory.createMultiPolygon(polygons);
            }
        } else {
            throw new Error("The specified geometry type is currently not supported: " + geometryType);
        }
    }

    static decodeGeometries(geometryTypes, geometryColumn: GeometryColumn) {
        const geometries = new Array(geometryTypes.length);
        const geometryCounter = new GeometryCounters();
        let numGeometries = 0

        const containsPolygon = geometryTypes.includes(
                GeometryType.POLYGON) || geometryTypes.includes(GeometryType.MULTIPOLYGON);
        for (const geometryTypeNum of geometryTypes) {
            const geometryType = geometryTypeNum as GeometryType;
            geometries[numGeometries++] = this.decodeGeometry(
                geometryType,
                containsPolygon,
                geometryCounter,
                geometryColumn);
        }

        return geometries;
    }

    private static getLinearRing(vertexBuffer: number[], startIndex: IntWrapper, numVertices: number, geometryFactory: GeometryFactory): LinearRing {
        const linearRing = this.getLineString(vertexBuffer, startIndex, numVertices, true);
        return geometryFactory.createLinearRing(linearRing);
    }

    private static decodeDictionaryEncodedLinearRing(vertexBuffer: number[], vertexOffsets: number[], startIndex: IntWrapper, numVertices: number, geometryFactory: GeometryFactory): LinearRing {
        const linearRing = this.decodeDictionaryEncodedLineString(vertexBuffer, vertexOffsets, startIndex, numVertices, true);
        return geometryFactory.createLinearRing(linearRing);
    }

    private static getLineString(vertexBuffer: number[], startIndex: IntWrapper, numVertices: number, closeLineString: boolean): Point[] {
        const vertices: Point[] = new Array(closeLineString ? numVertices + 1 : numVertices);
        for (let i = 0; i < numVertices * 2; i += 2) {
            const x = vertexBuffer[startIndex.get() + i];
            const y = vertexBuffer[startIndex.get() + i + 1];
            vertices[i / 2] = new Point(x, y);
        }

        if (closeLineString) {
            vertices[vertices.length - 1] = vertices[0];
        }
        return vertices;
    }

    private static decodeDictionaryEncodedLineString(vertexBuffer: number[], vertexOffsets: number[], startIndex: IntWrapper, numVertices: number, closeLineString: boolean) : Point[] {
        const vertices = new Array(closeLineString ? numVertices + 1 : numVertices);
        for (let i = 0; i < numVertices * 2; i += 2) {
            const offset = vertexOffsets[startIndex.get() + i / 2] * 2;
            const x = vertexBuffer[offset];
            const y = vertexBuffer[offset + 1];
            vertices[i / 2] = new Point(x, y);
        }

        if (closeLineString) {
            vertices[vertices.length - 1] = vertices[0];
        }
        return vertices;
    }

}



