import { decodeStreamMetadata } from "../metadata/tile/streamMetadataDecoder";
import { StringFlatVector } from "../vector/flat/stringFlatVector";
import { StringDictionaryVector } from "../vector/dictionary/stringDictionaryVector";
import type IntWrapper from "./intWrapper";
import BitVector from "../vector/flat/bitVector";
import type Vector from "../vector/vector";
import { PhysicalStreamType } from "../metadata/tile/physicalStreamType";
import { DictionaryType } from "../metadata/tile/dictionaryType";
import { LengthType } from "../metadata/tile/lengthType";
import { decodeIntStream, decodeLengthStreamToOffsetBuffer, decodeNullableIntStream } from "./integerStreamDecoder";
import { type Column, ScalarType } from "../metadata/tileset/tilesetMetadata";
import { decodeVarintInt32 } from "./integerDecodingUtils";
import { decodeBooleanRle, skipColumn } from "./decodingUtils";
import { StringFsstDictionaryVector } from "../vector/fsst-dictionary/stringFsstDictionaryVector";

const ROOT_COLUMN_NAME = "default";
const NESTED_COLUMN_SEPARATOR = ":";

export function decodeString(
    name: string,
    data: Uint8Array,
    offset: IntWrapper,
    numStreams: number,
    bitVector?: BitVector,
): Vector {
    let dictionaryLengthStream: Int32Array = null;
    let offsetStream: Int32Array = null;
    let dictionaryStream: Uint8Array = null;
    let symbolLengthStream: Int32Array = null;
    let symbolTableStream: Uint8Array = null;
    let presentStream: BitVector = null;
    let plainLengthStream: Int32Array = null;
    let plainDataStream: Uint8Array = null;

    for (let i = 0; i < numStreams; i++) {
        const streamMetadata = decodeStreamMetadata(data, offset);
        if (streamMetadata.byteLength === 0) {
            continue;
        }

        switch (streamMetadata.physicalStreamType) {
            case PhysicalStreamType.PRESENT: {
                const presentData = decodeBooleanRle(data, streamMetadata.numValues, offset);
                presentStream = new BitVector(presentData, streamMetadata.numValues);
                break;
            }
            case PhysicalStreamType.OFFSET: {
                const isNullable = bitVector != null || presentStream != null;
                const nullabilityBuffer = bitVector ?? presentStream;
                offsetStream = isNullable
                    ? decodeNullableIntStream(data, offset, streamMetadata, false, nullabilityBuffer)
                    : decodeIntStream(data, offset, streamMetadata, false);
                break;
            }
            case PhysicalStreamType.LENGTH: {
                const ls = decodeLengthStreamToOffsetBuffer(data, offset, streamMetadata);
                if (LengthType.DICTIONARY === streamMetadata.logicalStreamType.lengthType) {
                    dictionaryLengthStream = ls;
                } else if (LengthType.SYMBOL === streamMetadata.logicalStreamType.lengthType) {
                    symbolLengthStream = ls;
                } else {
                    // Plain string encoding uses VAR_BINARY length type
                    plainLengthStream = ls;
                }
                break;
            }
            case PhysicalStreamType.DATA: {
                const ds = data.subarray(offset.get(), offset.get() + streamMetadata.byteLength);
                offset.add(streamMetadata.byteLength);
                const dictType = streamMetadata.logicalStreamType.dictionaryType;
                if (DictionaryType.FSST === dictType) {
                    symbolTableStream = ds;
                } else if (DictionaryType.SINGLE === dictType || DictionaryType.SHARED === dictType) {
                    dictionaryStream = ds;
                } else if (DictionaryType.NONE === dictType) {
                    plainDataStream = ds;
                }
                break;
            }
        }
    }

    return (
        decodeFsstDictionaryVector(
            name,
            symbolTableStream,
            offsetStream,
            dictionaryLengthStream,
            dictionaryStream,
            symbolLengthStream,
            bitVector ?? presentStream,
        ) ??
        decodeDictionaryVector(
            name,
            dictionaryStream,
            offsetStream,
            dictionaryLengthStream,
            bitVector ?? presentStream,
        ) ??
        decodePlainStringVector(name, plainLengthStream, plainDataStream, offsetStream, bitVector ?? presentStream)
    );
}

function decodeFsstDictionaryVector(
    name: string,
    symbolTableStream: Uint8Array | null,
    offsetStream: Int32Array | null,
    dictionaryLengthStream: Int32Array | null,
    dictionaryStream: Uint8Array | null,
    symbolLengthStream: Int32Array | null,
    nullabilityBuffer: BitVector | null,
): Vector | null {
    if (!symbolTableStream) {
        return null;
    }
    return new StringFsstDictionaryVector(
        name,
        offsetStream,
        dictionaryLengthStream,
        dictionaryStream,
        symbolLengthStream,
        symbolTableStream,
        nullabilityBuffer,
    );
}

function decodeDictionaryVector(
    name: string,
    dictionaryStream: Uint8Array | null,
    offsetStream: Int32Array | null,
    dictionaryLengthStream: Int32Array | null,
    nullabilityBuffer: BitVector | null,
): Vector | null {
    if (!dictionaryStream) {
        return null;
    }
    return nullabilityBuffer
        ? new StringDictionaryVector(name, offsetStream, dictionaryLengthStream, dictionaryStream, nullabilityBuffer)
        : new StringDictionaryVector(name, offsetStream, dictionaryLengthStream, dictionaryStream);
}

function decodePlainStringVector(
    name: string,
    plainLengthStream: Int32Array | null,
    plainDataStream: Uint8Array | null,
    offsetStream: Int32Array | null,
    nullabilityBuffer: BitVector | null,
): Vector | null {
    if (!plainLengthStream || !plainDataStream) {
        return null;
    }

    if (offsetStream) {
        return nullabilityBuffer
            ? new StringDictionaryVector(name, offsetStream, plainLengthStream, plainDataStream, nullabilityBuffer)
            : new StringDictionaryVector(name, offsetStream, plainLengthStream, plainDataStream);
    }

    if (nullabilityBuffer && nullabilityBuffer.size() !== plainLengthStream.length - 1) {
        const sparseOffsetStream = new Int32Array(nullabilityBuffer.size());
        let valueIndex = 0;
        for (let i = 0; i < nullabilityBuffer.size(); i++) {
            if (nullabilityBuffer.get(i)) {
                sparseOffsetStream[i] = valueIndex++;
            } else {
                sparseOffsetStream[i] = 0;
            }
        }
        return new StringDictionaryVector(
            name,
            sparseOffsetStream,
            plainLengthStream,
            plainDataStream,
            nullabilityBuffer,
        );
    }

    return nullabilityBuffer
        ? new StringFlatVector(name, plainLengthStream, plainDataStream, nullabilityBuffer)
        : new StringFlatVector(name, plainLengthStream, plainDataStream);
}

export function decodeSharedDictionary(
    data: Uint8Array,
    offset: IntWrapper,
    column: Column,
    numFeatures: number,
    propertyColumnNames?: Set<string>,
): Vector[] {
    let dictionaryOffsetBuffer: Int32Array = null;
    let dictionaryBuffer: Uint8Array = null;
    let symbolOffsetBuffer: Int32Array = null;
    let symbolTableBuffer: Uint8Array = null;

    let dictionaryStreamDecoded = false;
    while (!dictionaryStreamDecoded) {
        const streamMetadata = decodeStreamMetadata(data, offset);
        switch (streamMetadata.physicalStreamType) {
            case PhysicalStreamType.LENGTH:
                if (LengthType.DICTIONARY === streamMetadata.logicalStreamType.lengthType) {
                    dictionaryOffsetBuffer = decodeLengthStreamToOffsetBuffer(data, offset, streamMetadata);
                } else {
                    symbolOffsetBuffer = decodeLengthStreamToOffsetBuffer(data, offset, streamMetadata);
                }
                break;
            case PhysicalStreamType.DATA:
                if (
                    DictionaryType.SINGLE === streamMetadata.logicalStreamType.dictionaryType ||
                    DictionaryType.SHARED === streamMetadata.logicalStreamType.dictionaryType
                ) {
                    dictionaryBuffer = data.subarray(offset.get(), offset.get() + streamMetadata.byteLength);
                    dictionaryStreamDecoded = true;
                } else {
                    symbolTableBuffer = data.subarray(offset.get(), offset.get() + streamMetadata.byteLength);
                }
                offset.add(streamMetadata.byteLength);
                break;
        }
    }

    const childFields = column.complexType.children;
    const stringDictionaryVectors = [];
    let i = 0;
    for (const childField of childFields) {
        const numStreams = decodeVarintInt32(data, offset, 1)[0];
        if (numStreams == 0) {
            /* Column is not present in the tile */
            continue;
        }

        const columnName = `${column.name}${
            childField.name === ROOT_COLUMN_NAME ? "" : NESTED_COLUMN_SEPARATOR + childField.name
        }`;
        if (propertyColumnNames) {
            if (!propertyColumnNames.has(columnName)) {
                //TODO: add size of sub column to Mlt for faster skipping
                skipColumn(numStreams, data, offset);
                continue;
            }
        }

        if (
            numStreams !== 2 ||
            childField.type !== "scalarField" ||
            childField.scalarField.physicalType !== ScalarType.STRING
        ) {
            throw new Error("Currently only optional string fields are implemented for a struct.");
        }

        const presentStreamMetadata = decodeStreamMetadata(data, offset);
        const presentStream = decodeBooleanRle(data, presentStreamMetadata.numValues, offset);
        const offsetStreamMetadata = decodeStreamMetadata(data, offset);
        const offsetCount = offsetStreamMetadata.decompressedCount;
        const isNullable = offsetCount !== numFeatures;
        const offsetStream = isNullable
            ? decodeNullableIntStream(
                  data,
                  offset,
                  offsetStreamMetadata,
                  false,
                  new BitVector(presentStream, presentStreamMetadata.numValues),
              )
            : decodeIntStream(data, offset, offsetStreamMetadata, false);

        stringDictionaryVectors[i++] = symbolTableBuffer
            ? new StringFsstDictionaryVector(
                  columnName,
                  offsetStream,
                  dictionaryOffsetBuffer,
                  dictionaryBuffer,
                  symbolOffsetBuffer,
                  symbolTableBuffer,
                  new BitVector(presentStream, presentStreamMetadata.numValues),
              )
            : new StringDictionaryVector(
                  columnName,
                  offsetStream,
                  dictionaryOffsetBuffer,
                  dictionaryBuffer,
                  new BitVector(presentStream, presentStreamMetadata.numValues),
              );
    }

    return stringDictionaryVectors;
}
