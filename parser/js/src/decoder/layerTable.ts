import {
    Geometry,
    GeometryType,
    IceLineString,
    IceMultiLineString,
    LineString,
    MultiLineString,
    MultiPoint,
    MultiPolygon,
    Point,
    Polygon,
} from "./geometry";
import { ColumnMetadata, LayerMetadata } from "./covtMetadata";
import { isBitSet } from "./decodingUtils";

const nameof = <T>(name: Extract<keyof T, string>): string => name;

export interface Feature {
    id: number;
    geometry: Geometry;
    //TODO: refactor to lazy load
    properties: Map<string, unknown>;
    //getProperty(propertyName): boolean | number | string;
}

export interface GeometryColumn {
    geometryTypes: Uint8Array;
    geometryOffsets?: Uint32Array;
    partOffsets?: Uint32Array;
    ringOffsets?: Uint32Array;
    vertexOffsets?: Uint32Array;
    vertexBuffer: Int32Array;
}

export interface StringColumn {
    dictionaryStream: string[];
}

export interface StringDictionaryColumn extends StringColumn {
    presentStream: Uint8Array;
    dataStream: Uint32Array;
}

export interface LocalizedStringDictionaryColumn extends StringColumn {
    localizedStreams: Map<string, [presentStream: Uint8Array, dataStream: Uint32Array]>;
}

export interface PrimitiveTypeColumn {
    presentStream: Uint8Array;
    dataStream: Uint8Array | BigUint64Array | BigInt64Array;
}

export type PropertyColumn = PrimitiveTypeColumn | StringDictionaryColumn | LocalizedStringDictionaryColumn;

export class LayerTable implements Iterable<Feature> {
    private featureOffset = 0;
    private vertexBufferOffset = 0;
    private geometryOffsetsOffset = 0;
    private partOffsetsOffset = 0;
    private ringOffsetsOffset = 0;
    private vertexOffsetsOffset = 0;
    private readonly numFeatures: number;
    private readonly columnMetadata: ColumnMetadata[];
    private readonly dataStreamsOffsets = new Map<string, number>();

    constructor(
        private readonly _layerMetadata: LayerMetadata,
        private readonly idColumn: number[],
        private readonly geometryColumn: GeometryColumn,
        private readonly propertyColumns: Map<string, PropertyColumn>,
    ) {
        this.numFeatures = _layerMetadata.numFeatures;
        this.columnMetadata = _layerMetadata.columnMetadata;
    }

    get layerMetadata(): LayerMetadata {
        return this._layerMetadata;
    }

    [Symbol.iterator](): Iterator<Feature> {
        return {
            next: () => {
                if (this.layerMetadata.numFeatures === this.featureOffset) {
                    return { value: null, done: true };
                }

                const id = this.getId();
                const geometry = this.getGeometry();
                const properties = this.getProperties();
                this.featureOffset++;
                return { value: { id, geometry, properties }, done: false };
            },
        };
    }

    private getId(): number {
        return this.idColumn[this.featureOffset];
    }

    private getGeometry(): Geometry {
        const geometryType = this.geometryColumn.geometryTypes[this.featureOffset];

        const vertexBuffer = this.geometryColumn.vertexBuffer;
        switch (geometryType) {
            case GeometryType.POINT: {
                const nextOffset = this.vertexBufferOffset + 2;
                const vertexBufferSlice = vertexBuffer.subarray(this.vertexBufferOffset, nextOffset);
                this.vertexBufferOffset = nextOffset;
                return new Point(vertexBufferSlice);
            }
            case GeometryType.LINESTRING: {
                const numVertices = this.geometryColumn.partOffsets[this.partOffsetsOffset++];
                if (this.geometryColumn.vertexOffsets) {
                    /* ICE encoding -> Just return references to the vertexBuffer to save space */
                    const vertexOffsetsEndOffset = this.vertexOffsetsOffset + numVertices;
                    const vertexOffsetSlice = this.geometryColumn.vertexOffsets.subarray(
                        this.vertexOffsetsOffset,
                        vertexOffsetsEndOffset,
                    );
                    this.vertexOffsetsOffset = vertexOffsetsEndOffset;
                    return new IceLineString(vertexBuffer, vertexOffsetSlice);
                } else {
                    /* Plain encoding */
                    const nextOffset = this.vertexBufferOffset + numVertices * 2;
                    const vertexBufferSlice = vertexBuffer.subarray(this.vertexBufferOffset, nextOffset);
                    this.vertexBufferOffset = nextOffset;
                    return new LineString(vertexBufferSlice);
                }
                break;
            }
            case GeometryType.POLYGON: {
                const endRingOffsetsOffset =
                    this.ringOffsetsOffset + this.geometryColumn.partOffsets[this.partOffsetsOffset++];
                const ringOffsetsSlice = this.geometryColumn.ringOffsets.subarray(
                    this.ringOffsetsOffset,
                    endRingOffsetsOffset,
                );
                //TODO: get rid of that by shifting to absolute offsets instead of the relative number
                const numVertices = ringOffsetsSlice.reduce((p, c) => p + c, 0);
                const vertexBufferEndOffset = this.vertexBufferOffset + numVertices * 2;
                const vertexBufferSlice = vertexBuffer.subarray(this.vertexBufferOffset, vertexBufferEndOffset);
                this.ringOffsetsOffset = endRingOffsetsOffset;
                this.vertexBufferOffset = vertexBufferEndOffset;
                return new Polygon(vertexBufferSlice, ringOffsetsSlice);
            }
            /* Nesting of the Multi* geometries currently not supported like defined in OGC SFA standard */
            case GeometryType.MULTI_POINT: {
                const nextOffset =
                    this.vertexBufferOffset + this.geometryColumn.geometryOffsets[this.geometryOffsetsOffset++] * 2;
                const vertexBufferSlice = vertexBuffer.subarray(this.vertexBufferOffset, nextOffset);
                this.vertexBufferOffset = nextOffset;
                return new MultiPoint(vertexBufferSlice);
            }
            case GeometryType.MULTI_LINESTRING: {
                const endPartOffsetsOffset =
                    this.partOffsetsOffset + this.geometryColumn.geometryOffsets[this.geometryOffsetsOffset++];
                const partOffsetsSlice = this.geometryColumn.partOffsets.subarray(
                    this.partOffsetsOffset,
                    endPartOffsetsOffset,
                );
                //TODO: get rid of that by shifting to absolute offsets instead of the relative number
                const numVertices = partOffsetsSlice.reduce((p, c) => p + c, 0);
                if (this.geometryColumn.vertexOffsets) {
                    /* ICE encoding -> Just return references to the vertexBuffer to save space */
                    const vertexOffsetsEndOffset = this.vertexOffsetsOffset + numVertices;
                    const vertexOffsetSlice = this.geometryColumn.vertexOffsets.subarray(
                        this.vertexOffsetsOffset,
                        vertexOffsetsEndOffset,
                    );
                    this.partOffsetsOffset = endPartOffsetsOffset;
                    this.vertexOffsetsOffset = vertexOffsetsEndOffset;
                    return new IceMultiLineString(vertexBuffer, partOffsetsSlice, vertexOffsetSlice);
                } else {
                    /* Plain encoding */
                    const vertexBufferEndOffset = this.vertexBufferOffset + numVertices * 2;
                    const vertexBufferSlice = vertexBuffer.subarray(this.vertexBufferOffset, vertexBufferEndOffset);
                    this.partOffsetsOffset = endPartOffsetsOffset;
                    this.vertexBufferOffset = vertexBufferEndOffset;
                    return new MultiLineString(vertexBufferSlice, partOffsetsSlice);
                }
            }
            case GeometryType.MULTI_POLYGON: {
                const partOffsetsEndOffset =
                    this.partOffsetsOffset + this.geometryColumn.geometryOffsets[this.geometryOffsetsOffset++];
                const partOffsetsSlice = this.geometryColumn.partOffsets.slice(
                    this.partOffsetsOffset,
                    partOffsetsEndOffset,
                );
                this.partOffsetsOffset = partOffsetsEndOffset;

                //TODO: get rid of that by shifting to absolute offsets instead of the relative number
                const numTotalRings = partOffsetsSlice.reduce((p, c) => p + c, 0);
                const ringOffsetsEndOffset = this.ringOffsetsOffset + numTotalRings;
                const ringOffsetsSlice = this.geometryColumn.ringOffsets.subarray(
                    this.ringOffsetsOffset,
                    ringOffsetsEndOffset,
                );
                this.ringOffsetsOffset = ringOffsetsEndOffset;

                //TODO: get rid of that by shifting to absolute offsets instead of the relative number
                const numTotalVertices = ringOffsetsSlice.reduce((p, c) => p + c, 0);
                const vertexBufferEndOffset = this.vertexBufferOffset + numTotalVertices * 2;
                const vertexBufferSlice = vertexBuffer.subarray(this.vertexBufferOffset, vertexBufferEndOffset);
                this.vertexBufferOffset = vertexBufferEndOffset;

                return new MultiPolygon(vertexBufferSlice, partOffsetsSlice, ringOffsetsSlice);
            }
        }
    }

    //TODO: lazy evaluate to only process the used columns
    private getProperties(): Map<string, unknown> {
        const properties = new Map<string, unknown>();
        for (const [propertyColumnName, propertyColumn] of this.propertyColumns) {
            if (this.isPrimitiveColumn(propertyColumn)) {
                if (isBitSet(propertyColumn.presentStream, this.featureOffset)) {
                    const dataStreamOffset = this.dataStreamsOffsets.get(propertyColumnName) ?? 0;
                    const propertyValue = propertyColumn.dataStream[dataStreamOffset];
                    this.dataStreamsOffsets.set(propertyColumnName, dataStreamOffset + 1);
                    properties.set(propertyColumnName, propertyValue);
                }
            } else if (this.isLocalizedStringDictionaryColumn(propertyColumn)) {
                const dict = propertyColumn.dictionaryStream;
                for (const [streamName, [presentStream, dataStream]] of propertyColumn.localizedStreams) {
                    if (isBitSet(presentStream, this.featureOffset)) {
                        const dataStreamOffset = this.dataStreamsOffsets.get(streamName) ?? 0;
                        const dataOffset = dataStream[dataStreamOffset];
                        properties.set(streamName, dict[dataOffset]);
                        this.dataStreamsOffsets.set(streamName, dataStreamOffset + 1);
                    }
                }
            } else {
                if (isBitSet(propertyColumn.presentStream, this.featureOffset)) {
                    const dataStreamOffset = this.dataStreamsOffsets.get(propertyColumnName) ?? 0;
                    const dataOffset = propertyColumn.dataStream[dataStreamOffset];
                    properties.set(propertyColumnName, propertyColumn.dictionaryStream[dataOffset]);
                    this.dataStreamsOffsets.set(propertyColumnName, dataStreamOffset + 1);
                }
            }
        }

        return properties;
    }

    private isPrimitiveColumn(propertyColumn: PropertyColumn): propertyColumn is PrimitiveTypeColumn {
        return (
            Object.keys(propertyColumn).length === 2 &&
            nameof<PrimitiveTypeColumn>("presentStream") in propertyColumn &&
            nameof<PrimitiveTypeColumn>("dataStream") in propertyColumn
        );
    }

    private isLocalizedStringDictionaryColumn(
        propertyColumn: PropertyColumn,
    ): propertyColumn is LocalizedStringDictionaryColumn {
        return nameof<LocalizedStringDictionaryColumn>("localizedStreams") in propertyColumn;
    }
}
