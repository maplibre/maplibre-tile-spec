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

/*
 * TODO:
 * - Adapt column encoding
 * - Improve string decoding performance
 * */

/*
* Basic
*   - All int are varint encoded
*   - If an id column is present it has to be the first column and geometry the second one
*   - There can be only one id and geometry column
*   - A column is separated in streams
*       -> Id -> data
*       -> Geometry -> geometryType, geometryOffsets, partOffsets, ringOffsets, vertexOffsets, vertexBuffer
*       -> Properties
*           -> int, boolean, float, double -> present, data
*           -> string plain -> present, length, data
*           -> string dictionary -> present, length, data_dictionary, data
*           -> string dictionary localized
* Covt Layout
* - File Header
*   - Version (UInt32 | Varint) | NumLayers (UInt32 | Varint)
* - Layer 1 Header
*   - Name (String) | NumFeatures (UInt32 | Varint) | numColumns (UInt32 | Varint) | Struct Column Metadata []
*   - Column Metadata Struct: Column name (String), data type (Byte), encoding (Byte), numLocalizedStreams (UInt32 | Varint) | streamSuffix (String [])
*    -> also numValues and Size and numStreams and streamSize?
  - Layer 1 Data
  *     - Id Column
  *         - DataType: UInt64
  *         - If present has to be the first column in the file with the name "id"
  *         - Encodings: Plain, RLE, Delta Varint -> Plain really needed as only 1 byte overhead for 128 values
  *             -> for 40k values 312 bytes overhead
  *     - Geometry Column
  *         - Streams
  *             - General
  *                 - GeometryTypes and VertexBuffer are mandatory
  *                 - GeometryTypes is always the first stream and VertexBuffer the last stream in the geometry column
  *             - GeometryTypes: UInt8 [] (Byte RLE)
  *             - GeometryOffsets: UInt32 [] (Delta ZigZag Varint)
  *             - PartOffsets: UInt32 [] (Delta ZigZag Varint)
  *             - RingOffsets: UInt32 [] (Delta ZigZag Varint)
  *             - VertexOffsets: UInt32 [] (Delta ZigZag Varint)
  *             - VertexBuffer: Uint32 [] UInt32 [] (Delta ZigZag Varint)
  *     - Property Columns
  *         - Data types
  *             - Boolean -> Boolean RLE and BitVector encoding
  *             - Int64
  *                 - Plain encoding -> ZigZag Varint encoding
  *                 - RLE encoding
  *             - UInt64
  *                 - Plain encoding -> Varint encocing
  *                 - RLE encoding
  *             - String
  *                 - Plain
  *                     - Streams: present, length, data
  *                 - Dictionary
  *                     - Streams: present, data (RLE), length (RLE), dictionary
  *                 - Localized Dictionary
  *                     - Central length and dictionary stream
  *                     - n present and data streams
  *             - Float
  *             - Double
  *         
* */
export function decodeCovt(buffer: Uint8Array): any {
    //{ ids: BigUint64Array; geometries: Geometry[]; [property: string]: unknown }[] {

    const { version, numLayers, offset: layerHeaderOffset } = decodeFileHeader(buffer);

    const layers = [];
    let offset = layerHeaderOffset;
    for (let i = 0; i < numLayers; i++) {
        const featureTable = {};

        const { layerMetadata, offset: layerDataOffset } = decodeLayerHeader(buffer, offset);
        const { name: layerName, numFeatures, numColumns, columnMetadata } = layerMetadata;

        let idColumn;
        if (columnMetadata[0].columnName === ID_COLUMN_NAME) {
            const idMetadata = columnMetadata.shift();
            //const [ids, geometryOffset] = decodeIdColumn(buffer, layerDataOffset, idMetadata, numFeatures);
            // const [ids2, geometryOffset2] = decodeRleSlow(buffer, numFeatures, offset);
            const [ids, geometryOffset] = decodeIdColumn(buffer, layerDataOffset, idMetadata, numFeatures);
            Object.assign(featureTable, { ids: ids });
            offset = geometryOffset;
            idColumn = ids;
        }

        const geometryMetadata = columnMetadata.shift();
        if (geometryMetadata.columnName !== GEOMETRY_COLUMN_NAME) {
            throw new Error(
                "The geometry column has to be the first or second column in the file depending on the presence ot the id column.",
            );
        }
        const { geometries, offset: propertiesOffset } = decodeGeometryColumn(
            buffer,
            offset,
            geometryMetadata,
            numFeatures,
        );
        Object.assign(featureTable, { geometries: geometries });
        offset = propertiesOffset;

        const featureProperties = new Map<string, unknown>();
        for (const columnMetadata of layerMetadata.columnMetadata) {
            const { data: properties, offset: nextColumnOffset } = decodePropertyColumn(
                buffer,
                offset,
                columnMetadata,
                numFeatures,
            );

            //Object.assign(featureProperties, { [columnMetadata.columnName]: properties });
            featureProperties.set(columnMetadata.columnName, properties);
            offset = nextColumnOffset;
        }
        //Object.assign(featureTable, { [columnMetadata.columnName]: properties });
        Object.assign(featureTable, { properties: featureProperties });

        const features = [];
        let localizedPropertyIndex = 0;
        for (let i = 0; i < numFeatures; i++) {
            const id = idColumn[i];
            const geometry = geometries[i];
            const properties = {};
            for (const [propertyName, propertyValues] of featureProperties.entries()) {
                if (propertyValues instanceof Map) {
                    for (const [streamName, streamValue] of propertyValues.entries()) {
                        Object.assign(properties, { [streamName]: streamValue[localizedPropertyIndex] });
                    }
                    localizedPropertyIndex++;
                } else {
                    const propertyValue = propertyValues[i];
                    Object.assign(properties, { [propertyName]: propertyValue });
                }
            }
            features.push({ id, geometry, properties });
        }

        layers.push(features);
        //layers.push(featureTable);
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
): [values: BigInt64Array, offset: number] {
    const columnEncoding = columnMetadata.columnEncoding;

    switch (columnEncoding) {
        case ColumnEncoding.RLE:
            return decodeRle(buffer, numFeatures, false, offset);
        case ColumnEncoding.DELTA_VARINT:
            return decodeDeltaLongVarints(buffer, numFeatures, offset);
        default:
            throw new Error("Currently only RLE and Delta Varint encoding supported for the id column.");
    }
}

//TODO: use a more efficient and faster representation as Array<Array<Point>>
function decodeVertexBuffer(
    numVertices: bigint,
    vertexOffsets: Int32Array,
    vertexOffsetCounter: number,
    vertexBuffer: Int32Array,
) {
    const vertices: Point[] = [];
    for (let i = 0; i < numVertices; i++) {
        const vertexOffset = vertexOffsets[vertexOffsetCounter++] * 2;
        const x = vertexBuffer[vertexOffset];
        const y = vertexBuffer[vertexOffset + 1];
        vertices.push({ x, y });
    }
    return { vertices, vertexOffsetCounter };
}

//TODO: store it a internal representation like arrow with references
function decodeGeometryColumn(
    buffer: Uint8Array,
    offset: number,
    columnMetadata: ColumnMetadata,
    numFeatures: number,
): { geometries: Geometry; offset: number } {
    const [geometryTypes, topologyStreamsOffset] = decodeByteRle(buffer, numFeatures, offset);
    offset = topologyStreamsOffset;
    const streams = columnMetadata.streams;

    //TODO: to support also Int32 for rle encoding
    //let geometryOffsets: Int32Array;
    let geometryOffsets: BigInt64Array;
    const geometryOffsetsMetadata = streams.get(GEOMETRY_OFFSETS_STREAM_NAME);
    if (geometryOffsetsMetadata) {
        const [values, nextOffset] = decodeRle(buffer, geometryOffsetsMetadata.numValues, false, offset);
        geometryOffsets = values;
        offset = nextOffset;
    }

    let partOffsets: BigInt64Array;
    const partOffsetsMetadata = streams.get(PART_OFFSETS_STREAM_NAME);
    if (partOffsetsMetadata) {
        const [values, nextOffset] = decodeRle(buffer, partOffsetsMetadata.numValues, false, offset);
        partOffsets = values;
        offset = nextOffset;
    }

    let ringOffsets: BigInt64Array;
    const ringOffsetsMetadata = streams.get(RING_OFFSETS_STREAM_NAME);
    if (ringOffsetsMetadata) {
        const [values, nextOffset] = decodeRle(buffer, ringOffsetsMetadata.numValues, false, offset);
        ringOffsets = values;
        offset = nextOffset;
    }

    let vertexOffsets: Uint32Array;
    let vertexBuffer: Int32Array;
    const vertexOffsetsMetadata = streams.get(VERTEX_OFFSETS_STREAM_NAME);
    const vertexBufferMetadata = streams.get(VERTEX_BUFFER_STREAM_NAME);
    if (vertexOffsetsMetadata) {
        const [values, vertexBufferOffset] = decodeDeltaVarints(buffer, vertexOffsetsMetadata.numValues, offset);
        const [coordinates, nextOffset] = decodeDeltaVarintCoordinates(
            buffer,
            vertexBufferMetadata.numValues,
            vertexBufferOffset,
        );

        vertexOffsets = values;
        vertexBuffer = coordinates;
        offset = nextOffset;
    }

    /*
     * - Depending on the geometryType and encoding read geometryOffsets, partOffsets, ringOffsets and vertexOffsets
     * Logical Representation
     * - Point: no stream
     * - LineString: Part offsets
     * - Polygon: Part offsets (Polygon), Ring offsets (LinearRing)
     * - MultiPoint: Geometry offsets -> array of offsets indicate where the vertices of each MultiPoint start
     * - MultiLineString: Geometry offsets, Part offsets (LineString)
     * - MultiPolygon -> Geometry offsets, Part offsets (Polygon), Ring offsets (LinearRing)
     * -> In addition when Indexed Coordinate Encoding (ICE) is used Vertex offsets stream is added
     * Physical Representation
     **/

    const geometries: Geometry = [];
    let vertexOffsetCounter = 0;
    //const vertexBufferCounter = 0;
    let partOffsetCounter = 0;
    let ringOffsetCounter = 0;
    let geometryOffsetCounter = 0;
    for (const geometryType of geometryTypes) {
        switch (geometryType) {
            case GeometryType.POINT: {
                //TODO: refactor for performance
                const [x, nextYOffset] = decodeZigZagVarint(buffer, offset);
                const [y, nextXOffset] = decodeZigZagVarint(buffer, nextYOffset);
                geometries.push([[{ x, y }]]);
                offset = nextXOffset;
                break;
            }
            case GeometryType.LINESTRING: {
                const numVertices = partOffsets[partOffsetCounter++];
                if (columnMetadata.columnEncoding === ColumnEncoding.INDEXED_COORDINATE_ENCODING) {
                    const { vertices, vertexOffsetCounter: nextCounter } = decodeVertexBuffer(
                        numVertices,
                        vertexOffsets as any,
                        vertexOffsetCounter,
                        vertexBuffer,
                    );
                    vertexOffsetCounter = nextCounter;
                    geometries.push([vertices]);
                } else if (columnMetadata.columnEncoding === ColumnEncoding.PLAIN) {
                    //TODO: get rid of BigInts when using the offsets
                    const [vertices, nextOffset] = decodeDeltaVarintCoordinates2(buffer, Number(numVertices), offset);
                    geometries.push([vertices]);
                    offset = nextOffset;
                } else {
                    throw new Error("The specified geometry encoding is not supported (yet).");
                }

                break;
            }
            case GeometryType.POLYGON: {
                const numRings = partOffsets[partOffsetCounter++];
                const rings: Array<Array<{ x: number; y: number }>> = [];
                for (let i = 0; i < numRings; i++) {
                    const numVertices = ringOffsets[ringOffsetCounter++];
                    const [vertices, nextOffset] = decodeDeltaVarintCoordinates2(buffer, Number(numVertices), offset);
                    /* Add redundant end point to be simple feature access conform */
                    vertices.push(vertices[0]);
                    rings.push(vertices);
                    offset = nextOffset;
                }
                geometries.push(rings);
                break;
            }
            case GeometryType.MULTIPOINT: {
                throw new Error("MultiPoint geometry type not (yet) supported.");
            }
            case GeometryType.MULTILINESTRING: {
                const numLineStrings = geometryOffsets[geometryOffsetCounter++];
                const lineStrings = [];
                if (columnMetadata.columnEncoding === ColumnEncoding.INDEXED_COORDINATE_ENCODING) {
                    for (let i = 0; i < numLineStrings; i++) {
                        const numVertices = partOffsets[partOffsetCounter++];
                        const { vertices, vertexOffsetCounter: nextCounter } = decodeVertexBuffer(
                            numVertices,
                            vertexOffsets as any,
                            vertexOffsetCounter,
                            vertexBuffer,
                        );
                        vertexOffsetCounter = nextCounter;
                        lineStrings.push(vertices);
                    }
                } else if (columnMetadata.columnEncoding === ColumnEncoding.PLAIN) {
                    for (let i = 0; i < numLineStrings; i++) {
                        const numVertices = partOffsets[partOffsetCounter++];
                        const [vertices, nextOffset] = decodeDeltaVarintCoordinates2(
                            buffer,
                            Number(numVertices),
                            offset,
                        );
                        lineStrings.push(vertices);
                        offset = nextOffset;
                    }
                } else {
                    throw new Error("The specified geometry encoding is not supported (yet).");
                }
                geometries.push(lineStrings);
                break;
            }
            case GeometryType.MULTIPOLYGON: {
                const numPolygons = geometryOffsets[geometryOffsetCounter++];
                const rings: Array<Array<{ x: number; y: number }>> = [];
                for (let i = 0; i < numPolygons; i++) {
                    const numRings = partOffsets[partOffsetCounter++];
                    for (let j = 0; j < numRings; j++) {
                        const numVertices = ringOffsets[ringOffsetCounter++];
                        const [vertices, nextOffset] = decodeDeltaVarintCoordinates2(
                            buffer,
                            Number(numVertices),
                            offset,
                        );
                        /* Add redundant end point to be simple feature access conform */
                        vertices.push(vertices[0]);
                        rings.push(vertices);
                        offset = nextOffset;
                    }
                }
                geometries.push(rings);
                break;
            }
        }
    }

    return { geometries, offset };
}

function decodePropertyColumn(
    buffer: Uint8Array,
    offset: number,
    columnMetadata: ColumnMetadata,
    numFeatures: number,
): { data: (boolean | number | string)[] | Map<string, string[]>; offset: number } {
    if (columnMetadata.columnEncoding === ColumnEncoding.LOCALIZED_DICTIONARY) {
        /*
         * -> central length and dictionary stream followed by n localized streams
         * -> localized stream -> present data e.g. name:en
         * -> streams: length, dictionary, present1, data1, present2, data2
         * */

        const streams = columnMetadata.streams;
        //TODO: get rid of
        const lengthDictionaryOffset =
            offset +
            Array.from(streams)
                .filter(([name, data]) => name !== "length" && name !== "dictionary")
                .reduce((p, [name, data]) => p + data.byteLength, 0);

        const numLengthValues = streams.get("length").numValues;
        const [lengths, dictionaryStreamOffset] = decodeRle(buffer, numLengthValues, false, lengthDictionaryOffset);
        const [dictionary, nextColumnOffset] = decodeStringDictionary(buffer, dictionaryStreamOffset, lengths as any);

        const localizedColumns = new Map<string, string[]>();
        let presentStream: Uint8Array = null;
        let i = 0;
        for (const [streamName, streamData] of streams) {
            /* the last two streams are the length and dictionary stream */
            if (i >= streams.size - 2) {
                break;
            }

            /* every second stream is a present stream */
            if (i % 2 === 0) {
                /* byteLength of the present stream has a control byte included */
                const [nextPresentStream, dataOffset] = decodeByteRle(buffer, streamData.byteLength - 1, offset);
                presentStream = nextPresentStream;
                offset = dataOffset;
            } else {
                const [data, nextStreamOffset] = decodeRle(buffer, streamData.numValues, false, offset);
                const properties = decodeStringDataColumn(dictionary, data, presentStream, numFeatures);
                const columnName = columnMetadata.columnName;
                const propertyName = columnName === streamName ? columnName : `${columnName}:${streamName}`;
                localizedColumns.set(propertyName, properties);
                offset = nextStreamOffset;
            }

            i++;
        }

        /* As the localized dictionary is the last stream in the column use this offset */
        return { data: localizedColumns, offset: nextColumnOffset };
    }

    const numBytes = Math.ceil(numFeatures / 8);
    const [presentStream, dataOffset] = decodeByteRle(buffer, numBytes, offset);

    switch (columnMetadata.columnType) {
        case ColumnDataType.BOOLEAN: {
            if (columnMetadata.columnEncoding !== ColumnEncoding.BOOLEAN_RLE) {
                throw new Error("Specified encoding not supported for a boolean property type.");
            }

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
            //TODO: store it a internal representation like arrow with references
            if (columnMetadata.columnEncoding === ColumnEncoding.DICTIONARY) {
                const [data, lengthStreamOffset] = decodeRle(buffer, numDataValues, false, dataOffset);
                const [lengths, dictionaryStreamOffset] = decodeRle(buffer, numLengthValues, false, lengthStreamOffset);
                const [dictionary, nextColumnOffset] = decodeStringDictionary(
                    buffer,
                    dictionaryStreamOffset,
                    lengths as any,
                );

                const properties = decodeStringDataColumn(dictionary, data, presentStream, numFeatures);
                return { data: properties, offset: nextColumnOffset };
            } else {
                throw new Error("Specified encoding not supported for a String property type.");
            }
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
    data: BigInt64Array,
    presentStream: Uint8Array,
    numFeatures: number,
): string[] {
    const properties: string[] = [];
    let dataCounter = 0;
    for (let i = 0; i < numFeatures; i++) {
        if (isBitSet(presentStream, i)) {
            //TODO: refactor
            const index = Number(data[dataCounter++]);
            const value = dictionary[index];
            properties.push(value);
        } else {
            //TODO: refactor -> setting undefined in that case is not valid
            properties.push(undefined);
        }
    }

    return properties;
}
