import {
    decodeByteRle,
    decodeDeltaNumberVarints,
    decodeDeltaVarints,
    decodeInt64Rle,
    decodeInt64Varints,
    decodeNumberRle,
    decodeString,
    decodeStringField,
    decodeUint32Rle,
    decodeUInt64Rle,
    decodeUint64Varints,
    decodeVarint,
    decodeZigZagVarint,
} from "./decodingUtils";
import { decodePropertyColumn } from "./propertyDecoder";
import { GeometryColumn, LayerTable, PropertyColumn } from "./layerTable";
import { TileSetMetadata } from "./mlt_tileset_metadata_pb";
import { ColumnDataType, ColumnEncoding, ColumnMetadata, LayerMetadata, StreamMetadata } from "./covtMetadata";
import { GeometryType } from "./geometry";
import Pbf from "pbf";

export class CovtDecoder {
    private static readonly ID_COLUMN_NAME = "id";
    private static readonly GEOMETRY_COLUMN_NAME = "geometry";
    private static readonly GEOMETRY_OFFSETS_STREAM_NAME = "geometry_offsets";
    private static readonly PART_OFFSETS_STREAM_NAME = "part_offsets";
    private static readonly RING_OFFSETS_STREAM_NAME = "ring_offsets";
    private static readonly VERTEX_OFFSETS_STREAM_NAME = "vertex_offsets";
    private static readonly VERTEX_BUFFER_STREAM_NAME = "vertex_buffer";

    private readonly layerTables = new Map<string, LayerTable>();

    // TODO real typescript interface for this
    constructor(private readonly covTile: Uint8Array, private readonly tilesetMetadata: TileSetMetadata) {
        const {
            version,
            featureTableId,
            numLayers,
            offset: layerHeaderOffset
        } = this.decodeFileHeader(covTile);
        console.log({ version, numLayers, layerHeaderOffset });
        console.log(tilesetMetadata);

        let offset = layerHeaderOffset;
        const metadata = tilesetMetadata.featureTables[featureTableId];

        for (const columnMetadata of metadata.columns) {
            const columnName = columnMetadata.name;
            const [numStreams, numStreamsOffset] = decodeVarint(covTile, offset);
            offset = numStreamsOffset;

            console.log(columnName, numStreams);

            if (columnName === CovtDecoder.ID_COLUMN_NAME) {
                if (numStreams === 2) {
                    // TODO
                }

                // TODO
            }
            else if(columnName === CovtDecoder.GEOMETRY_COLUMN_NAME) {
                /*
                const { geometryColumn, offset: nextOffset } = this.decodeGeometryColumn(
                    covTile,
                    offset,
                    numFeatures,
                    geometryMetadata,
                );
                offset = nextOffset;
                */
                console.log(columnMetadata);
            }
            else {
                const propertyColumn = decodePropertyColumn(covTile, offset, columnMetadata, numStreams);
                console.log(columnName, propertyColumn);
                // TODO other properties
            }
        }

        for (let i = 0; i < numLayers; i++) {
            const { layerMetadata, offset: layerDataOffset } = this.decodeLayerHeader(covTile, offset);
            const { name: layerName, numFeatures, numColumns, columnMetadata } = layerMetadata;

            let idColumn;
            offset = layerDataOffset;
            if (columnMetadata[0].name === CovtDecoder.ID_COLUMN_NAME) {
                const idMetadata = columnMetadata.shift();
                /* This solution is limited to 53 bits but this is also the case in the mapbox vector-tile-js lib */
                const [ids, geometryOffset] = this.decodeIdColumn(
                    covTile,
                    offset,
                    numFeatures,
                    idMetadata.columnEncoding,
                );
                idColumn = ids;

                offset = geometryOffset;
            }

            const geometryMetadata = columnMetadata.shift();
            if (geometryMetadata.name !== CovtDecoder.GEOMETRY_COLUMN_NAME) {
                throw new Error(
                    "The geometry column has to be the first or second column in the file depending on the presence ot the id column.",
                );
            }

            const { geometryColumn, offset: nextOffset } = this.decodeGeometryColumn(
                covTile,
                offset,
                numFeatures,
                geometryMetadata,
            );
            offset = nextOffset;

            const propertyColumns = new Map<string, PropertyColumn>();
            for (const columnMetadata of layerMetadata.columnMetadata) {
                const { data: properties, offset: nextColumnOffset } = decodePropertyColumn(
                    covTile,
                    offset,
                    null, // columnMetadata,
                    numFeatures,
                );
                propertyColumns.set(columnMetadata.name, properties);

                offset = nextColumnOffset;
            }

            const layer = new LayerTable(layerMetadata, idColumn, geometryColumn, propertyColumns);
            this.layerTables.set(layerName, layer);
        }
    }

    get layerNames(): string[] {
        return Array.from(this.layerTables.keys());
    }

    getLayerTable(layerName: string): LayerTable {
        return this.layerTables.get(layerName);
    }

    private decodeFileHeader(buffer: Uint8Array): { version: number; featureTableId: number; tileExtent: number; maxTileExtent: number; numLayers: number; offset: number } {
        let nextOffset = 0;
        let version, featureTableId, tileExtent, maxTileExtent, numLayers;

        [version, nextOffset] = decodeVarint(buffer, nextOffset);
        [featureTableId, nextOffset] = decodeVarint(buffer, nextOffset);
        [tileExtent, nextOffset] = decodeVarint(buffer, nextOffset);
        [maxTileExtent, nextOffset] = decodeVarint(buffer, nextOffset);
        [numLayers, nextOffset] = decodeVarint(buffer, nextOffset);

        return {
            version,
            featureTableId,
            tileExtent,
            maxTileExtent,
            numLayers,
            offset: nextOffset
        };
    }

    private decodeLayerHeader(buffer: Uint8Array, offset: number): { layerMetadata: LayerMetadata; offset: number } {
        return { layerMetadata: null, offset: 0 };
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

        /*
        const [layerName, numFeaturesOffset] = decodeStringField(buffer, offset);
        const [numFeatures, numColumnsOffset] = decodeVarint(buffer, numFeaturesOffset);
        const [numColumns, layerMetadataOffset]: [value: number, offset: number] = decodeVarint(
            buffer,
            numColumnsOffset,
        );

        const columnMetadata: ColumnMetadata[] = [];
        let metadataOffset = layerMetadataOffset;
        for (let i = 0; i < numColumns; i++) {
            const [columnName, numStreamsOffset] = decodeStringField(buffer, metadataOffset);
            metadataOffset = numStreamsOffset;
            const columnType: ColumnDataType = buffer[metadataOffset++];
            const columnEncoding: ColumnEncoding = buffer[metadataOffset++];
            const [numStreams, streamMetadataOffset] = decodeVarint(buffer, metadataOffset);
            metadataOffset = streamMetadataOffset;

            const streams = new Map<string, StreamMetadata>();
            for (let j = 0; j < numStreams; j++) {
                const [name, numValuesOffset] = decodeStringField(buffer, metadataOffset);
                const [numValues, streamByteLengthOffset] = decodeVarint(buffer, numValuesOffset);
                const [byteLength, nextStreamOffset] = decodeVarint(buffer, streamByteLengthOffset);
                streams.set(name, { numValues, byteLength });
                metadataOffset = nextStreamOffset;
            }

            columnMetadata.push({ name, columnType, columnEncoding, streams });
        }

        const layerMetadata = { name: layerName, numFeatures, numColumns, columnMetadata };
        return { layerMetadata, offset: metadataOffset };
        */
    }

    private decodeIdColumn(
        buffer: Uint8Array,
        offset: number,
        numIds: number,
        columnEncoding: ColumnEncoding,
    ): [values: number[], offset: number] {
        switch (columnEncoding) {
            case ColumnEncoding.RLE:
                return decodeNumberRle(buffer, numIds, offset);
            case ColumnEncoding.DELTA_VARINT:
                return decodeDeltaNumberVarints(buffer, numIds, offset);
            default:
                throw new Error("Currently only RLE and Delta Varint encoding supported for the id column.");
        }
    }

    /*
     * - Depending on the geometryType the following topology streams are encoded: geometryOffsets, partOffsets, ringOffsets and vertexOffsets
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
    //TODO: use absolute offsets regarding the vertex buffer not the numer of geometries, parts, ....
    private decodeGeometryColumn(
        buffer: Uint8Array,
        offset: number,
        numFeatures: number,
        columnMetadata: ColumnMetadata,
    ): { geometryColumn: GeometryColumn; offset: number } {
        const geometryStreams = columnMetadata.streams;
        const [geometryTypes, topologyStreamsOffset] = decodeByteRle(buffer, numFeatures, offset);
        offset = topologyStreamsOffset;

        //TODO: Currently the topology streams (offsets arrays) are not implemented as absolute offsets -> change
        let geometryOffsets: Uint32Array;
        const geometryOffsetsMetadata = geometryStreams.get(CovtDecoder.GEOMETRY_OFFSETS_STREAM_NAME);
        if (geometryOffsetsMetadata) {
            const [values, nextOffset] = decodeUint32Rle(buffer, geometryOffsetsMetadata.numValues, offset);
            geometryOffsets = values;
            offset = nextOffset;
        }

        let partOffsets: Uint32Array;
        const partOffsetsMetadata = geometryStreams.get(CovtDecoder.PART_OFFSETS_STREAM_NAME);
        if (partOffsetsMetadata) {
            const [values, nextOffset] = decodeUint32Rle(buffer, partOffsetsMetadata.numValues, offset);
            partOffsets = values;
            offset = nextOffset;
        }

        const vertexBufferMetadata = geometryStreams.get(CovtDecoder.VERTEX_BUFFER_STREAM_NAME);
        if (columnMetadata.columnEncoding === ColumnEncoding.INDEXED_COORDINATE_ENCODING) {
            /* ICE encoding currently only supported for LineStrings and MultiLineStrings*/
            const vertexOffsetsMetadata = geometryStreams.get(CovtDecoder.VERTEX_OFFSETS_STREAM_NAME);
            const [vertexOffsets, vertexBufferOffset] = decodeDeltaVarints(
                buffer,
                vertexOffsetsMetadata.numValues,
                offset,
            );
            const [vertexBuffer, nextOffset] = this.decodeDeltaVarintCoordinates(
                buffer,
                vertexBufferMetadata.numValues,
                vertexBufferOffset,
            );

            offset = nextOffset;
            const geometries = { geometryTypes, geometryOffsets, partOffsets, vertexOffsets, vertexBuffer };
            return { geometryColumn: geometries, offset };
        }

        //TODO: refactor -> decode like in ICE encoding now without VertexOffset stream
        let ringOffsets: Uint32Array;
        const ringOffsetsMetadata = geometryStreams.get(CovtDecoder.RING_OFFSETS_STREAM_NAME);
        if (ringOffsetsMetadata) {
            const [values, nextOffset] = decodeUint32Rle(buffer, ringOffsetsMetadata.numValues, offset);
            ringOffsets = values;
            offset = nextOffset;
        }

        const vertexBuffer = new Int32Array(vertexBufferMetadata.numValues * 2);
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
                    const [nextOffset, newVertexBufferOffset] = this.decodeLineString(
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
                        const [nextOffset, newVertexBufferOffset] = this.decodeLineString(
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
                case GeometryType.MULTI_LINESTRING: {
                    const numLineStrings = geometryOffsets[geometryOffsetCounter++];
                    for (let i = 0; i < numLineStrings; i++) {
                        const numVertices = partOffsets[partOffsetCounter++];
                        const [nextOffset, newVertexBufferOffset] = this.decodeLineString(
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
                case GeometryType.MULTI_POLYGON: {
                    const numPolygons = geometryOffsets[geometryOffsetCounter++];
                    for (let i = 0; i < numPolygons; i++) {
                        const numRings = partOffsets[partOffsetCounter++];
                        for (let j = 0; j < numRings; j++) {
                            const numVertices = ringOffsets[ringOffsetCounter++];
                            const [nextOffset, newVertexBufferOffset] = this.decodeLineString(
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

        const geometries: GeometryColumn = { geometryTypes, geometryOffsets, partOffsets, ringOffsets, vertexBuffer };
        return { geometryColumn: geometries, offset };
    }

    private decodeLineString(
        buffer: Uint8Array,
        bufferOffset: number,
        numVertices: number,
        vertexBuffer: Int32Array,
        vertexBufferOffset,
    ): [offset: number, vertexBufferOffset: number] {
        let x = 0;
        let y = 0;
        for (let i = 0; i < numVertices; i++) {
            const [deltaX, nextYOffset] = decodeZigZagVarint(buffer, bufferOffset);
            const [deltaY, nextXOffset] = decodeZigZagVarint(buffer, nextYOffset);
            x += deltaX;
            y += deltaY;
            vertexBuffer[vertexBufferOffset++] = x;
            vertexBuffer[vertexBufferOffset++] = y;
            bufferOffset = nextXOffset;
        }

        return [bufferOffset, vertexBufferOffset];
    }

    private decodeDeltaVarintCoordinates(
        buffer: Uint8Array,
        numCoordinates: number,
        offset = 0,
    ): [vertices: Int32Array, offset: number] {
        const vertices = new Int32Array(numCoordinates * 2);

        let x = 0;
        let y = 0;
        let coordIndex = 0;
        for (let i = 0; i < numCoordinates; i++) {
            const [deltaX, nextYOffset] = decodeZigZagVarint(buffer, offset);
            const [deltaY, nextXOffset] = decodeZigZagVarint(buffer, nextYOffset);

            x += deltaX;
            y += deltaY;
            vertices[coordIndex++] = x;
            vertices[coordIndex++] = y;

            offset = nextXOffset;
        }

        return [vertices, offset];
    }

    private decodeStringDictionary(
        buffer: Uint8Array,
        offset: number,
        lengths: Uint32Array,
    ): [values: string[], offset: number] {
        const values = [];
        for (let i = 0; i < lengths.length; i++) {
            const length = lengths[i];
            const endOffset = offset + length;
            const value = decodeString(buffer, offset, endOffset);
            values.push(value);
            offset = endOffset;
        }

        return [values, offset];
    }
}
