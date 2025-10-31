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

        for (let i = 0; i < numStreams; i++) {
            const streamMetadata = StreamMetadataDecoder.decode(data, offset);
            if (streamMetadata.byteLength === 0) {
                continue;
            }

            switch (streamMetadata.physicalStreamType) {
                case PhysicalStreamType.OFFSET: {
                    const isNullable = bitVector != null;
                    offsetStream = isNullable
                        ? IntegerStreamDecoder.decodeNullableIntStream(data, offset, streamMetadata, false, bitVector)
                        : IntegerStreamDecoder.decodeIntStream(data, offset, streamMetadata, false);
                    break;
                }
                case PhysicalStreamType.LENGTH: {
                    const ls = IntegerStreamDecoder.decodeLengthStreamToOffsetBuffer(data, offset, streamMetadata);
                    if (LengthType.DICTIONARY === streamMetadata.logicalStreamType.lengthType) {
                        dictionaryLengthStream = ls;
                    } else {
                        symbolLengthStream = ls;
                    }
                    break;
                }
                case PhysicalStreamType.DATA: {
                    const ds = data.subarray(offset.get(), offset.get() + streamMetadata.byteLength);
                    offset.add(streamMetadata.byteLength);
                    if (DictionaryType.SINGLE === streamMetadata.logicalStreamType.dictionaryType) {
                        dictionaryStream = ds;
                    } else {
                        symbolTableStream = ds;
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
                bitVector,
            );
        } else if (dictionaryStream) {
            return bitVector
                ? new StringDictionaryVector(name, offsetStream, dictionaryLengthStream, dictionaryStream, bitVector)
                : new StringDictionaryVector(name, offsetStream, dictionaryLengthStream, dictionaryStream);
        }

        return bitVector
            ? new StringFlatVector(name, offsetStream, dictionaryStream, bitVector)
            : new StringFlatVector(name, offsetStream, dictionaryStream);
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
            const isNullable =
                (offsetStreamMetadata instanceof RleEncodedStreamMetadata
                    ? offsetStreamMetadata.numRleValues
                    : offsetStreamMetadata.numValues) !== numFeatures;
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
