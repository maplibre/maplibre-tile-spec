import {
    decodeByteRle,
    decodeDeltaVarintCoordinates,
    decodeDeltaLongVarints,
    decodeDeltaVarints,
    decodeString,
    decodeVarint,
    decodeDeltaVarintCoordinates2,
    isBitSet,
    decodeZigZagLongVarints,
    decodeLongVarints,
    decodeStringDictionary,
    decodeRle,
    decodeZigZagVarint,
    decodeUnsignedRle,
    decodeAndToVertexBuffer,
} from "./decodingUtils";

enum GeometryType {
    POINT,
    LINESTRING,
    POLYGON,
    MULTIPOINT,
    MULTILINESTRING,
    MULTIPOLYGON,
}

enum ColumnDataType {
    STRING,
    FLOAT,
    DOUBLE,
    INT_64,
    UINT_64,
    BOOLEAN,
    GEOMETRY,
    GEOMETRY_M,
    GEOMETRY_Z,
    GEOMETRY_ZM,
}

enum ColumnEncoding {
    /*
     * String -> no dictionary coding
     * Geometry -> standard unsorted encoding
     * */
    PLAIN,
    VARINT,
    DELTA_VARINT,
    RLE,
    BOOLEAN_RLE,
    BYTE_RLE,
    DICTIONARY,
    LOCALIZED_DICTIONARY,
    ORDERED_GEOMETRY_ENCODING,
    INDEXED_COORDINATE_ENCODING,
}

interface ColumnMetadata {
    columnName: string;
    columnType: ColumnDataType;
    columnEncoding: ColumnEncoding;
    streams: Map<string, StreamMetadata>;
}

interface StreamMetadata {
    numValues: number;
    byteLength: number;
}

interface LayerMetadata {
    name: string;
    numColumns: number;
    numFeatures: number;
    columnMetadata: ColumnMetadata[];
}

export interface Point {
    x: number;
    y: number;
}

export type Geometry = Array<Array<Array<Point>>>;

const ID_COLUMN_NAME = "id";
const GEOMETRY_COLUMN_NAME = "geometry";
const GEOMETRY_TYPES_STREAM_NAME = "geometry_types";
const GEOMETRY_OFFSETS_STREAM_NAME = "geometry_offsets";
const PART_OFFSETS_STREAM_NAME = "part_offsets";
const RING_OFFSETS_STREAM_NAME = "ring_offsets";
const VERTEX_OFFSETS_STREAM_NAME = "vertex_offsets";
const VERTEX_BUFFER_STREAM_NAME = "vertex_buffer";

export function decodeCovtFast(buffer: Uint8Array): any {
    const { version, numLayers, offset: layerHeaderOffset } = decodeFileHeader(buffer);

    const layers = [];
    let offset = layerHeaderOffset;
    for (let i = 0; i < numLayers; i++) {
        const { layerMetadata, offset: layerDataOffset } = decodeLayerHeader(buffer, offset);
        const { name: layerName, numFeatures, numColumns, columnMetadata } = layerMetadata;

        if (columnMetadata[0].columnName === ID_COLUMN_NAME) {
            const idMetadata = columnMetadata.shift();
            const [ids, geometryOffset] = decodeIdColumn(buffer, layerDataOffset, idMetadata, numFeatures);
            offset = geometryOffset;
        }

        const geometryMetadata = columnMetadata.shift();
        if (geometryMetadata.columnName !== GEOMETRY_COLUMN_NAME) {
            throw new Error(
                "The geometry column has to be the first or second column in the file depending on the presence ot the id column.",
            );
        }

        const { geometries, offset: nextOffset } = decodeGeometryColumn(buffer, offset, geometryMetadata, numFeatures);
        offset = nextOffset;

        for (const columnMetadata of layerMetadata.columnMetadata) {
            const { data: properties, offset: nextColumnOffset } = decodePropertyColumn(
                buffer,
                offset,
                columnMetadata,
                numFeatures,
            );

            offset = nextColumnOffset;
        }
    }

    return layers;
}

function decodeFileHeader(buffer: Uint8Array): { version: number; numLayers: number; offset: number } {
    const [version, numLayersOffset] = decodeVarint(buffer, 0);
    const [numLayers, offset] = decodeVarint(buffer, numLayersOffset);
    return { version, numLayers, offset };
}

function decodeLayerHeader(buffer: Uint8Array, offset: number): { layerMetadata: LayerMetadata; offset: number } {
    /*
     * -> LayerHeader -> LayerName, NumFeatures, NumColumns, ColumnMetadata[]
     * -> ColumnMetadata -> ColumnName, DataType, ColumnEncoding, numStreams, GeometryMetadata | PrimitiveTypeMetadata | StringDictionaryMetadata | LocalizedStringDictionaryMetadata
     * -> StreamMetadata -> StreamName, NumValues, ByteLength
     * -> IdMetadata -> PrimitiveTypeMetadata -> StreamMetadata[1]
     * -> GeometryMetadata -> StreamMetadata[max 6],
     * -> PrimitiveTypeMetadata -> StreamMetadata[2]
     * -> StringDictionaryMetadata -> StreamMetadata[4]
     * -> LocalizedStringDictionaryMetadata -> StreamMetadata[n]
     * */

    const [layerName, numFeaturesOffset] = decodeString(buffer, offset);
    const [numFeatures, numColumnsOffset] = decodeVarint(buffer, numFeaturesOffset);
    const [numColumns, layerMetadataOffset]: [value: number, offset: number] = decodeVarint(buffer, numColumnsOffset);

    const columnMetadata: ColumnMetadata[] = [];
    let metadataOffset = layerMetadataOffset;
    for (let i = 0; i < numColumns; i++) {
        const [columnName, numStreamsOffset] = decodeString(buffer, metadataOffset);
        metadataOffset = numStreamsOffset;
        const columnType: ColumnDataType = buffer[metadataOffset++];
        const columnEncoding: ColumnEncoding = buffer[metadataOffset++];
        const [numStreams, streamMetadataOffset] = decodeVarint(buffer, metadataOffset);
        metadataOffset = streamMetadataOffset;

        const streams = new Map<string, StreamMetadata>();
        for (let j = 0; j < numStreams; j++) {
            const [name, numValuesOffset] = decodeString(buffer, metadataOffset);
            const [numValues, streamByteLengthOffset] = decodeVarint(buffer, numValuesOffset);
            const [byteLength, nextStreamOffset] = decodeVarint(buffer, streamByteLengthOffset);
            streams.set(name, { numValues, byteLength });
            metadataOffset = nextStreamOffset;
        }

        columnMetadata.push({ columnName, columnType, columnEncoding, streams });
    }

    const layerMetadata = { name: layerName, numFeatures, numColumns, columnMetadata };
    return { layerMetadata, offset: metadataOffset };
}

function decodeIdColumn(
    buffer: Uint8Array,
    offset: number,
    columnMetadata: ColumnMetadata,
    numFeatures: number,
): [values: Uint32Array, offset: number] {
    const columnEncoding = columnMetadata.columnEncoding;

    switch (columnEncoding) {
        case ColumnEncoding.RLE:
            return decodeUnsignedRle(buffer, numFeatures, offset);
        case ColumnEncoding.DELTA_VARINT:
            return decodeDeltaVarints(buffer, numFeatures, offset);
        default:
            throw new Error("Currently only RLE and Delta Varint encoding supported for the id column.");
    }
}

function decodeGeometryColumn(
    buffer: Uint8Array,
    offset: number,
    columnMetadata: ColumnMetadata,
    numFeatures: number,
): { geometries: any[]; offset: number } {
    const [geometryTypes, topologyStreamsOffset] = decodeByteRle(buffer, numFeatures, offset);
    offset = topologyStreamsOffset;
    const streams = columnMetadata.streams;

    let geometryOffsets: Uint32Array;
    const geometryOffsetsMetadata = streams.get(GEOMETRY_OFFSETS_STREAM_NAME);
    if (geometryOffsetsMetadata) {
        const [values, nextOffset] = decodeUnsignedRle(buffer, geometryOffsetsMetadata.numValues, offset);
        geometryOffsets = values;
        offset = nextOffset;
    }

    let partOffsets: Uint32Array;
    const partOffsetsMetadata = streams.get(PART_OFFSETS_STREAM_NAME);
    if (partOffsetsMetadata) {
        const [values, nextOffset] = decodeUnsignedRle(buffer, partOffsetsMetadata.numValues, offset);
        partOffsets = values;
        offset = nextOffset;
    }

    let ringOffsets: Uint32Array;
    const ringOffsetsMetadata = streams.get(RING_OFFSETS_STREAM_NAME);
    if (ringOffsetsMetadata) {
        const [values, nextOffset] = decodeUnsignedRle(buffer, ringOffsetsMetadata.numValues, offset);
        ringOffsets = values;
        offset = nextOffset;
    }

    const vertexOffsetsMetadata = streams.get(VERTEX_OFFSETS_STREAM_NAME);
    const vertexBufferMetadata = streams.get(VERTEX_BUFFER_STREAM_NAME);
    if (vertexOffsetsMetadata) {
        const [vertexOffsets, vertexBufferOffset] = decodeDeltaVarints(buffer, vertexOffsetsMetadata.numValues, offset);
        const [vertexBuffer, nextOffset] = decodeDeltaVarintCoordinates(
            buffer,
            vertexBufferMetadata.numValues,
            vertexBufferOffset,
        );

        offset = nextOffset;
        return { geometries: [geometryTypes, geometryOffsets, partOffsets, vertexOffsets, vertexBuffer], offset };
    }

    const vertexBuffer = new Uint32Array(vertexBufferMetadata.numValues * 2);
    let partOffsetCounter = 0;
    let ringOffsetCounter = 0;
    let geometryOffsetCounter = 0;
    let vertexBufferOffset = 0;
    for (const geometryType of geometryTypes) {
        switch (geometryType) {
            case GeometryType.POINT: {
                const [x, nextYOffset] = decodeZigZagVarint(buffer, offset);
                const [y, nextXOffset] = decodeZigZagVarint(buffer, nextYOffset);
                vertexBuffer[vertexBufferOffset++] = x;
                vertexBuffer[vertexBufferOffset++] = y;
                offset = nextXOffset;
                break;
            }
            case GeometryType.LINESTRING: {
                const numVertices = partOffsets[partOffsetCounter++];
                const [nextOffset, newVertexBufferOffset] = decodeAndToVertexBuffer(
                    buffer,
                    offset,
                    numVertices,
                    vertexBuffer,
                    vertexBufferOffset,
                );
                offset = nextOffset;
                vertexBufferOffset = newVertexBufferOffset;
                break;
            }
            case GeometryType.POLYGON: {
                const numRings = partOffsets[partOffsetCounter++];
                for (let i = 0; i < numRings; i++) {
                    const numVertices = ringOffsets[ringOffsetCounter++];
                    const [nextOffset, newVertexBufferOffset] = decodeAndToVertexBuffer(
                        buffer,
                        offset,
                        numVertices,
                        vertexBuffer,
                        vertexBufferOffset,
                    );
                    offset = nextOffset;
                    vertexBufferOffset = newVertexBufferOffset;
                }
                break;
            }
            case GeometryType.MULTILINESTRING: {
                const numLineStrings = geometryOffsets[geometryOffsetCounter++];
                for (let i = 0; i < numLineStrings; i++) {
                    const numVertices = partOffsets[partOffsetCounter++];
                    const [nextOffset, newVertexBufferOffset] = decodeAndToVertexBuffer(
                        buffer,
                        offset,
                        numVertices,
                        vertexBuffer,
                        vertexBufferOffset,
                    );
                    offset = nextOffset;
                    vertexBufferOffset = newVertexBufferOffset;
                }
                break;
            }
            case GeometryType.MULTIPOLYGON: {
                const numPolygons = geometryOffsets[geometryOffsetCounter++];
                for (let i = 0; i < numPolygons; i++) {
                    const numRings = partOffsets[partOffsetCounter++];
                    for (let j = 0; j < numRings; j++) {
                        const numVertices = ringOffsets[ringOffsetCounter++];
                        const [nextOffset, newVertexBufferOffset] = decodeAndToVertexBuffer(
                            buffer,
                            offset,
                            numVertices,
                            vertexBuffer,
                            vertexBufferOffset,
                        );
                        offset = nextOffset;
                        vertexBufferOffset = newVertexBufferOffset;
                    }
                }
                break;
            }
        }
    }

    return { geometries: [geometryTypes, geometryOffsets, partOffsets, ringOffsets, vertexBuffer], offset };
}

function decodePropertyColumn(
    buffer: Uint8Array,
    offset: number,
    columnMetadata: ColumnMetadata,
    numFeatures: number,
): { data: (boolean | number | string)[] | Map<string, string[]>; offset: number } {
    if (columnMetadata.columnEncoding === ColumnEncoding.LOCALIZED_DICTIONARY) {
        const streams = columnMetadata.streams;
        const lengthDictionaryOffset =
            offset +
            Array.from(streams)
                .filter(([name, data]) => name !== "length" && name !== "dictionary")
                .reduce((p, [name, data]) => p + data.byteLength, 0);

        const numLengthValues = streams.get("length").numValues;
        const [lengths, dictionaryStreamOffset] = decodeUnsignedRle(buffer, numLengthValues, lengthDictionaryOffset);
        const [dictionary, nextColumnOffset] = decodeStringDictionary(buffer, dictionaryStreamOffset, lengths);

        const localizedColumns = new Map<string, string[]>();
        let presentStream: Uint8Array = null;
        let i = 0;
        for (const [streamName, streamData] of streams) {
            if (i >= streams.size - 2) {
                break;
            }

            if (i % 2 === 0) {
                const [nextPresentStream, dataOffset] = decodeByteRle(buffer, streamData.byteLength - 1, offset);
                presentStream = nextPresentStream;
                offset = dataOffset;
            } else {
                const [data, nextStreamOffset] = decodeUnsignedRle(buffer, streamData.numValues, offset);
                offset = nextStreamOffset;
            }

            i++;
        }
        return { data: localizedColumns, offset: nextColumnOffset };
    }

    const numBytes = Math.ceil(numFeatures / 8);
    const [presentStream, dataOffset] = decodeByteRle(buffer, numBytes, offset);

    switch (columnMetadata.columnType) {
        case ColumnDataType.BOOLEAN: {
            const [sparseProperties, nextColumnOffset] = decodeByteRle(
                buffer,
                columnMetadata.streams.get("data").numValues,
                dataOffset,
            );

            const properties = [];
            let valueIndexCounter = 0;
            for (let i = 0; i < numFeatures; i++) {
                if (isBitSet(presentStream, i)) {
                    const value = isBitSet(sparseProperties, valueIndexCounter++);
                    properties.push(value);
                } else {
                    //TODO: refactor -> setting undefined in that case is not valid
                    properties.push(undefined);
                }
            }

            return { data: properties, offset: nextColumnOffset };
        }
        case ColumnDataType.INT_64:
        case ColumnDataType.UINT_64: {
            const numPropertyValues = columnMetadata.streams.get("data").numValues;
            if (columnMetadata.columnEncoding === ColumnEncoding.VARINT) {
                const [sparseProperties, nextColumnOffset] =
                    columnMetadata.columnType === ColumnDataType.UINT_64
                        ? decodeLongVarints(buffer, numPropertyValues, dataOffset)
                        : decodeZigZagLongVarints(buffer, numPropertyValues, dataOffset);
                return {
                    data: decodeIntDataColumn(sparseProperties, presentStream, numFeatures),
                    offset: nextColumnOffset,
                };
            } else if (columnMetadata.columnEncoding === ColumnEncoding.RLE) {
                //TODO: also handle unsigned int
                const [sparseProperties, nextColumnOffset] = decodeRle(buffer, numPropertyValues, true, dataOffset);
                return {
                    data: decodeIntDataColumn(sparseProperties, presentStream, numFeatures),
                    offset: nextColumnOffset,
                };
            } else {
                throw new Error("Specified encoding not supported for a int property type.");
            }
        }
        case ColumnDataType.STRING: {
            const numDataValues = columnMetadata.streams.get("data").numValues;
            const numLengthValues = columnMetadata.streams.get("length").numValues;
            const [data, lengthStreamOffset] = decodeUnsignedRle(buffer, numDataValues, dataOffset);
            const [lengths, dictionaryStreamOffset] = decodeUnsignedRle(buffer, numLengthValues, lengthStreamOffset);
            const [dictionary, nextColumnOffset] = decodeStringDictionary(buffer, dictionaryStreamOffset, lengths);

            //const properties = decodeStringDataColumn(dictionary, data, presentStream, numFeatures);
            //return { data: properties, offset: nextColumnOffset };
            return { data: null, offset: nextColumnOffset };
        }
    }
}

//TODO: inefficient to mix int with undefined
function decodeIntDataColumn(
    dataStream: BigInt64Array | BigUint64Array,
    presentStream: Uint8Array,
    numFeatures: number,
) {
    const properties = [];
    let valueIndexCounter = 0;
    for (let i = 0; i < numFeatures; i++) {
        if (isBitSet(presentStream, i)) {
            const value = dataStream[valueIndexCounter++];
            properties.push(value);
        } else {
            //TODO: refactor -> setting undefined in that case is not valid
            properties.push(undefined);
        }
    }

    return properties;
}

function decodeStringDataColumn(
    dictionary: string[],
    data: Uint32Array,
    presentStream: Uint8Array,
    numFeatures: number,
): string[] {
    const properties: string[] = [];
    let dataCounter = 0;
    for (let i = 0; i < numFeatures; i++) {
        if (isBitSet(presentStream, i)) {
            const index = data[dataCounter++];
            const value = dictionary[index];
            properties.push(value);
        } else {
            //TODO: refactor -> setting undefined in that case is not valid
            properties.push(undefined);
        }
    }

    return properties;
}
