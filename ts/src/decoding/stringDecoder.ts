import { StreamMetadataDecoder } from "../metadata/tile/streamMetadataDecoder";
import { StringFlatVector } from "../vector/flat/stringFlatVector";
import { StringDictionaryVector } from "../vector/dictionary/stringDictionaryVector";
import type IntWrapper from "./intWrapper";
import BitVector from "../vector/flat/bitVector";
import type Vector from "../vector/vector";
import { PhysicalStreamType } from "../metadata/tile/physicalStreamType";
import { DictionaryType } from "../metadata/tile/dictionaryType";
import { LengthType } from "../metadata/tile/lengthType";
import IntegerStreamDecoder from "./integerStreamDecoder";
import { type Column, ScalarType } from "../metadata/tileset/tilesetMetadata";
import { decodeVarintInt32 } from "./integerDecodingUtils";
import { decodeBooleanRle, skipColumn } from "./decodingUtils";
import { RleEncodedStreamMetadata } from "../metadata/tile/rleEncodedStreamMetadata";
import { StringFsstDictionaryVector } from "../vector/fsst-dictionary/stringFsstDictionaryVector";

export class StringDecoder {
    private static readonly ROOT_COLUMN_NAME = "default";
    private static readonly NESTED_COLUMN_SEPARATOR = ":";

    private constructor() {}

    static decode(
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
            const streamMetadata = StreamMetadataDecoder.decode(data, offset);
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
                        ? IntegerStreamDecoder.decodeNullableIntStream(data, offset, streamMetadata, false, nullabilityBuffer)
                        : IntegerStreamDecoder.decodeIntStream(data, offset, streamMetadata, false);
                    break;
                }
                case PhysicalStreamType.LENGTH: {
                    const ls = IntegerStreamDecoder.decodeLengthStreamToOffsetBuffer(data, offset, streamMetadata);
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
                        // Symbol table for FSST encoding
                        symbolTableStream = ds;
                    } else if (DictionaryType.SINGLE === dictType || DictionaryType.SHARED === dictType) {
                        // Dictionary data (plain dictionary or FSST-compressed corpus)
                        dictionaryStream = ds;
                    } else if (DictionaryType.NONE === dictType) {
                        // Plain string data (no dictionary)
                        plainDataStream = ds;
                    }
                    break;
                }
            }
        }

        if (symbolTableStream) {
            return new StringFsstDictionaryVector(
                name,
                offsetStream,
                dictionaryLengthStream,
                dictionaryStream,
                symbolLengthStream,
                symbolTableStream,
                bitVector ?? presentStream,
            );
        } else if (dictionaryStream) {
            const nullabilityBuffer = bitVector ?? presentStream;
            return nullabilityBuffer
                ? new StringDictionaryVector(name, offsetStream, dictionaryLengthStream, dictionaryStream, nullabilityBuffer)
                : new StringDictionaryVector(name, offsetStream, dictionaryLengthStream, dictionaryStream);
        }

        // Plain string encoding: LENGTH + DATA streams
        if (plainLengthStream && plainDataStream) {
            const nullabilityBuffer = bitVector ?? presentStream;

            if (offsetStream) {
                // Has OFFSET stream: use dictionary-style indexing
                return nullabilityBuffer
                    ? new StringDictionaryVector(name, offsetStream, plainLengthStream, plainDataStream, nullabilityBuffer)
                    : new StringDictionaryVector(name, offsetStream, plainLengthStream, plainDataStream);
            } else if (nullabilityBuffer && nullabilityBuffer.size() !== plainLengthStream.length - 1) {
                // plainLengthStream.length-1 = number of actual string values
                // nullabilityBuffer.size() = number of features
                const sparseOffsetStream = new Int32Array(nullabilityBuffer.size());
                let valueIndex = 0;
                for (let i = 0; i < nullabilityBuffer.size(); i++) {
                    if (nullabilityBuffer.get(i)) {
                        sparseOffsetStream[i] = valueIndex++;
                    } else {
                        sparseOffsetStream[i] = 0; // Null values map to index 0 (will be ignored due to nullability check)
                    }
                }
                return new StringDictionaryVector(name, sparseOffsetStream, plainLengthStream, plainDataStream, nullabilityBuffer);
            } else {
                return nullabilityBuffer
                    ? new StringFlatVector(name, plainLengthStream, plainDataStream, nullabilityBuffer)
                    : new StringFlatVector(name, plainLengthStream, plainDataStream);
            }
        }
        //Return null for empty columns
        return null;
    }

    static decodeSharedDictionary(
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
            const streamMetadata = StreamMetadataDecoder.decode(data, offset);
            switch (streamMetadata.physicalStreamType) {
                case PhysicalStreamType.LENGTH:
                    if (LengthType.DICTIONARY === streamMetadata.logicalStreamType.lengthType) {
                        dictionaryOffsetBuffer = IntegerStreamDecoder.decodeLengthStreamToOffsetBuffer(
                            data,
                            offset,
                            streamMetadata,
                        );
                    } else {
                        symbolOffsetBuffer = IntegerStreamDecoder.decodeLengthStreamToOffsetBuffer(
                            data,
                            offset,
                            streamMetadata,
                        );
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
                childField.name === StringDecoder.ROOT_COLUMN_NAME
                    ? ""
                    : StringDecoder.NESTED_COLUMN_SEPARATOR + childField.name
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

            const presentStreamMetadata = StreamMetadataDecoder.decode(data, offset);
            const presentStream = decodeBooleanRle(data, presentStreamMetadata.numValues, offset);
            const offsetStreamMetadata = StreamMetadataDecoder.decode(data, offset);
            const offsetCount = (offsetStreamMetadata instanceof RleEncodedStreamMetadata
                ? offsetStreamMetadata.numRleValues
                : offsetStreamMetadata.numValues);
            const isNullable = offsetCount !== numFeatures;
            const offsetStream = isNullable
                ? IntegerStreamDecoder.decodeNullableIntStream(
                    data,
                    offset,
                    offsetStreamMetadata,
                    false,
                    new BitVector(presentStream, presentStreamMetadata.numValues),
                )
                : IntegerStreamDecoder.decodeIntStream(data, offset, offsetStreamMetadata, false);

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
}