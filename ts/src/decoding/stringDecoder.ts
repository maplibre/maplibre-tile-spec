import { decodeStreamMetadata } from "../metadata/tile/streamMetadataDecoder";
import { StringFlatVector } from "../vector/flat/stringFlatVector";
import { StringDictionaryVector } from "../vector/dictionary/stringDictionaryVector";
import type IntWrapper from "./intWrapper";
import BitVector from "../vector/flat/bitVector";
import type Vector from "../vector/vector";
import { PhysicalStreamType } from "../metadata/tile/physicalStreamType";
import { DictionaryType } from "../metadata/tile/dictionaryType";
import { LengthType } from "../metadata/tile/lengthType";
import { decodeUnsignedInt32Stream, decodeLengthStreamToOffsetBuffer } from "./integerStreamDecoder";
import { type Column, ScalarType } from "../metadata/tileset/tilesetMetadata";
import { decodeVarintInt32 } from "./integerDecodingUtils";
import { decodeBooleanRle, skipColumn } from "./decodingUtils";
import { StringFsstDictionaryVector } from "../vector/fsst-dictionary/stringFsstDictionaryVector";

export function decodeString(
    name: string,
    data: Uint8Array,
    offset: IntWrapper,
    numStreams: number,
    bitVector?: BitVector,
): Vector {
    let dictionaryLengthStream: Uint32Array = null;
    let offsetStream: Uint32Array = null;
    let dictionaryStream: Uint8Array = null;
    let symbolLengthStream: Uint32Array = null;
    let symbolTableStream: Uint8Array = null;
    let nullabilityBuffer: BitVector = bitVector ?? null;
    let plainLengthStream: Uint32Array = null;
    let plainDataStream: Uint8Array = null;

    for (let i = 0; i < numStreams; i++) {
        const streamMetadata = decodeStreamMetadata(data, offset);

        switch (streamMetadata.physicalStreamType) {
            case PhysicalStreamType.PRESENT: {
                const presentData = decodeBooleanRle(data, streamMetadata.numValues, streamMetadata.byteLength, offset);
                const presentStream = new BitVector(presentData, streamMetadata.numValues);
                nullabilityBuffer = bitVector ?? presentStream;
                break;
            }
            case PhysicalStreamType.OFFSET: {
                offsetStream = decodeUnsignedInt32Stream(data, offset, streamMetadata, undefined, nullabilityBuffer);
                break;
            }
            case PhysicalStreamType.LENGTH: {
                const lengthStream = decodeLengthStreamToOffsetBuffer(data, offset, streamMetadata);
                if (LengthType.DICTIONARY === streamMetadata.logicalStreamType.lengthType) {
                    dictionaryLengthStream = lengthStream;
                } else if (LengthType.SYMBOL === streamMetadata.logicalStreamType.lengthType) {
                    symbolLengthStream = lengthStream;
                } else {
                    // Plain string encoding uses VAR_BINARY length type
                    plainLengthStream = lengthStream;
                }
                break;
            }
            case PhysicalStreamType.DATA: {
                const dataStream = data.subarray(offset.get(), offset.get() + streamMetadata.byteLength);
                offset.add(streamMetadata.byteLength);
                const dictType = streamMetadata.logicalStreamType.dictionaryType;
                if (DictionaryType.FSST === dictType) {
                    symbolTableStream = dataStream;
                } else if (DictionaryType.SINGLE === dictType || DictionaryType.SHARED === dictType) {
                    dictionaryStream = dataStream;
                } else if (DictionaryType.NONE === dictType) {
                    plainDataStream = dataStream;
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
            nullabilityBuffer,
        ) ??
        decodeDictionaryVector(name, dictionaryStream, offsetStream, dictionaryLengthStream, nullabilityBuffer) ??
        decodePlainStringVector(name, plainLengthStream, plainDataStream, offsetStream, nullabilityBuffer)
    );
}

function decodeFsstDictionaryVector(
    name: string,
    symbolTableStream: Uint8Array | null,
    offsetStream: Uint32Array | null,
    dictionaryLengthStream: Uint32Array | null,
    dictionaryStream: Uint8Array | null,
    symbolLengthStream: Uint32Array | null,
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
    offsetStream: Uint32Array | null,
    dictionaryLengthStream: Uint32Array | null,
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
    plainLengthStream: Uint32Array | null,
    plainDataStream: Uint8Array | null,
    offsetStream: Uint32Array | null,
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
        const sparseOffsetStream = new Uint32Array(nullabilityBuffer.size());
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
    let dictionaryOffsetBuffer: Uint32Array = null;
    let dictionaryBuffer: Uint8Array = null;
    let symbolOffsetBuffer: Uint32Array = null;
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
        if (numStreams === 0) {
            /* Column is not present in the tile */
            continue;
        }

        const columnName = childField.name ? `${column.name}${childField.name}` : column.name;
        if (propertyColumnNames) {
            if (!propertyColumnNames.has(columnName)) {
                //TODO: add size of sub column to Mlt for faster skipping
                skipColumn(numStreams, data, offset);
                continue;
            }
        }

        if (childField.type !== "scalarField" || childField.scalarField.physicalType !== ScalarType.STRING) {
            throw new Error("Currently only scalar string fields are implemented for a struct.");
        }
        if ((numStreams > 1 && !childField.nullable) || (numStreams === 1 && childField.nullable)) {
            throw new Error(
                `The number of streams for the child field ${childField.name} does not match its nullability. nullibilty: ${childField.nullable}, numStreams: ${numStreams}`,
            );
        }

        let presentStreamBitVector: BitVector | undefined;
        if (childField.nullable) {
            const presentStreamMetadata = decodeStreamMetadata(data, offset);
            const presentStream = decodeBooleanRle(
                data,
                presentStreamMetadata.numValues,
                presentStreamMetadata.byteLength,
                offset,
            );
            presentStreamBitVector = new BitVector(presentStream, presentStreamMetadata.numValues);
        }
        const offsetStreamMetadata = decodeStreamMetadata(data, offset);
        const offsetCount = offsetStreamMetadata.decompressedCount;
        const isNullable = offsetCount !== numFeatures;
        const offsetStream = decodeUnsignedInt32Stream(
            data,
            offset,
            offsetStreamMetadata,
            undefined,
            isNullable ? presentStreamBitVector : undefined,
        );

        stringDictionaryVectors[i++] = symbolTableBuffer
            ? new StringFsstDictionaryVector(
                  columnName,
                  offsetStream,
                  dictionaryOffsetBuffer,
                  dictionaryBuffer,
                  symbolOffsetBuffer,
                  symbolTableBuffer,
                  presentStreamBitVector,
              )
            : new StringDictionaryVector(
                  columnName,
                  offsetStream,
                  dictionaryOffsetBuffer,
                  dictionaryBuffer,
                  presentStreamBitVector,
              );
    }

    return stringDictionaryVectors;
}
