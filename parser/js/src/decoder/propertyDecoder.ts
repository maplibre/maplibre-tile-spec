import ieee754 from "ieee754";
import {
    decodeByteRle,
    decodeInt64Rle,
    decodeInt64Varints,
    decodeUint32Rle,
    decodeUInt64Rle,
    decodeUint64Varints,
} from "./decodingUtils";
import {
    ColumnDataType,
    ColumnEncoding,
    ColumnMetadata,
    LayerMetadata,
    StreamMetadata
} from "./covtMetadata";
import { Column } from "./mlt_tileset_metadata_pb";
import { GeometryColumn, LayerTable, PropertyColumn } from "./layerTable";
import { ScalarType } from "./mlt_tileset_metadata_pb";
import { decodeString } from "./stringDecoder";

export function decodePropertyColumn(
    data: Uint8Array,
    offset: number,
    column: Column,
    numStreams: number
) : {
    data: PropertyColumn;
    offset: number;
} {
    // TODO add back
    /*
    if (column.columnEncoding === ColumnEncoding.LOCALIZED_DICTIONARY) {
        const streams = column.streams;
        //TODO: optimize
        const lengthDictionaryOffset =
            offset +
            Array.from(streams)
                .filter(([name, data]) => name !== "length" && name !== "dictionary")
                .reduce((p, [name, data]) => p + data.byteLength, 0);

        const numLengthValues = streams.get("length").numValues;
        const [lengthStream, dictionaryStreamOffset] = decodeUint32Rle(
            data,
            numLengthValues,
            lengthDictionaryOffset,
        );
        const [dictionaryStream, nextColumnOffset] = this.decodeStringDictionary(
            data,
            dictionaryStreamOffset,
            lengthStream,
        );

        const localizedStreams = new Map<string, [Uint8Array, Uint32Array]>();
        let presentStream: Uint8Array = null;
        let i = 0;
        for (const [streamName, streamData] of streams) {
            if (i >= streams.size - 2) {
                break;
            }

            if (i % 2 === 0) {
                const numBytes = Math.ceil(numStreams / 8);
                const [nextPresentStream, dataOffset] = decodeByteRle(data, numBytes, offset);
                presentStream = nextPresentStream;
                offset = dataOffset;
            } else {
                const [dataStream, nextStreamOffset] = decodeUint32Rle(data, streamData.numValues, offset);
                offset = nextStreamOffset;
                const columnName = column.name;
                const propertyName = columnName === streamName ? columnName : `${columnName}:${streamName}`;
                localizedStreams.set(propertyName, [presentStream, dataStream]);
            }

            i++;
        }

        return { data: { dictionaryStream, localizedStreams }, offset: nextColumnOffset };
    }
    */

    const numBytesPresentStream = Math.ceil(numStreams / 8);
    const [presentStream, dataOffset] = decodeByteRle(data, numBytesPresentStream, offset);

    let numValues = 0;
    let presentStreamMetadata = null;

    if (numStreams > 1) {
        // TODO
        presentStreamMetadata = StreamMetadataDecoder.decode(data, offset);
        numValues = presentStreamMetadata.numValues();
        presentStream = DecodingUtils.decodeBooleanRle(data, presentStreamMetadata.numValues(), presentStreamMetadata.byteLength(), offset);
    }

    // TODO determine if scalar type first
    console.log(column);

    switch (column.type.value.type.value) {
        case ScalarType.BOOLEAN: {
            const [dataStream, nextColumnOffset] = decodeByteRle(data, numBytesPresentStream, dataOffset);
            return { data: { presentStream, dataStream }, offset: nextColumnOffset };
        }
            /*
        case ScalarType.INT_64:
        case ScalarType.UINT_64: {
            const numPropertyValues = column.streams.get("data").numValues;
            if (column.columnEncoding === ColumnEncoding.VARINT) {
                const [dataStream, nextColumnOffset] =
                    column.type === ColumnDataType.UINT_64
                        ? decodeUint64Varints(data, numPropertyValues, dataOffset)
                        : decodeInt64Varints(data, numPropertyValues, dataOffset);
                return {
                    data: { presentStream, dataStream },
                    offset: nextColumnOffset,
                };
            } else if (column.columnEncoding === ColumnEncoding.RLE) {
                const [dataStream, nextColumnOffset] =
                    column.type === ColumnDataType.UINT_64
                        ? decodeUInt64Rle(data, numPropertyValues, dataOffset)
                        : decodeInt64Rle(data, numPropertyValues, dataOffset);
                return {
                    data: { presentStream, dataStream },
                    offset: nextColumnOffset,
                };
            } else {
                throw new Error("Specified encoding not supported for a int property type.");
            }
        }
        */
            /*
        case ColumnDataType.FLOAT: {
            const numPropertyValues = column.streams.get("data").numValues;
            const dataStream = new Float32Array(numPropertyValues);
            let offset = dataOffset;
            for (let i = 0; i < numPropertyValues; i++) {
                dataStream[i] = ieee754.read(data, offset, true, 23, Float32Array.BYTES_PER_ELEMENT);
                offset += Float32Array.BYTES_PER_ELEMENT;
            }

            return {
                data: { presentStream, dataStream },
                offset,
            };
        }
        */
        case ScalarType.STRING: {
            const strValues = decodeString(data, offset, numStreams - 1, presentStream, numValues);
            return strValues.slice(-1)[0];
        }

            /*
            const numDataValues = column.streams.get("data").numValues;
            const numLengthValues = column.streams.get("length").numValues;
            const [dataStream, lengthStreamOffset] = decodeUint32Rle(data, numDataValues, dataOffset);
            const [lengthStream, dictionaryStreamOffset] = decodeUint32Rle(
                data,
                numLengthValues,
                lengthStreamOffset,
            );
            const [dictionaryStream, nextColumnOffset] = this.decodeStringDictionary(
                data,
                dictionaryStreamOffset,
                lengthStream,
            );

            return {
                data: { presentStream, dataStream, dictionaryStream },
                offset: nextColumnOffset,
            };
            */
        }
    }
    return { data: null, offset };
}
