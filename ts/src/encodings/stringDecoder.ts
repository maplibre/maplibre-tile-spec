import {StreamMetadataDecoder} from "../metadata/tile/streamMetadataDecoder";
import {StringFlatVector} from "../vector/flat/stringFlatVector";
import {StringDictionaryVector} from "../vector/dictionary/stringDictionaryVector";
import IntWrapper from "./intWrapper";
import BitVector from "../vector/flat/bitVector";
import Vector from "../vector/vector";
import {PhysicalStreamType} from "../metadata/tile/physicalStreamType";
import {DictionaryType} from "../metadata/tile/dictionaryType";
import {LengthType} from "../metadata/tile/lengthType";
import IntegerStreamDecoder from "./integerStreamDecoder";
import {Column, ComplexColumn, ComplexField, ScalarField, ScalarType} from "../metadata/tileset/tilesetMetadata";
import {decodeVarintInt32} from "./integerDecodingUtils";
import {decodeBooleanRle, skipColumn} from "./decodingUtils";
import {RleEncodedStreamMetadata} from "../metadata/tile/rleEncodedStreamMetadata";
import {StringFsstDictionaryVector} from "../vector/fsst-dictionary/stringFsstDictionaryVector";

export class StringDecoder {
    private static readonly ROOT_COLUMN_NAME = "default";
    private static readonly NESTED_COLUMN_SEPARATOR = ":";

    private constructor() {}

    static decode(name: string, data: Uint8Array, offset: IntWrapper, numStreams: number, bitVector?: BitVector): Vector {
        let dictionaryLengthStream: Int32Array = null;
        let offsetStream: Int32Array = null;
        let dictionaryStream: Uint8Array = null;
        let symbolLengthStream: Int32Array = null;
        let symbolTableStream: Uint8Array = null;

        for (let i = 0; i < numStreams; i++) {
            const streamMetadata = StreamMetadataDecoder.decode(data, offset);
            if(streamMetadata.byteLength === 0){
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
                case PhysicalStreamType.LENGTH:{
                    const ls = IntegerStreamDecoder.decodeLengthStreamToOffsetBuffer(data, offset, streamMetadata);
                    if (LengthType.DICTIONARY === streamMetadata.logicalStreamType.lengthType) {
                        dictionaryLengthStream = ls;
                    } else {
                        symbolLengthStream = ls;
                    }
                    break;
                }
                case PhysicalStreamType.DATA:{
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
                name, offsetStream, dictionaryLengthStream, dictionaryStream, symbolLengthStream, symbolTableStream, bitVector
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

    static decodeSharedDictionary(data: Uint8Array, offset: IntWrapper, column: Column, numFeatures: number,
                                  propertyColumnNames?: Set<string>): Vector[] {
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
                        dictionaryOffsetBuffer = IntegerStreamDecoder.decodeLengthStreamToOffsetBuffer(data, offset, streamMetadata);
                    } else {
                        symbolOffsetBuffer = IntegerStreamDecoder.decodeLengthStreamToOffsetBuffer(data, offset, streamMetadata);
                    }
                    break;
                case PhysicalStreamType.DATA:
                    if (DictionaryType.SINGLE === streamMetadata.logicalStreamType.dictionaryType ||
                        DictionaryType.SHARED === streamMetadata.logicalStreamType.dictionaryType) {
                        dictionaryBuffer = data.subarray(offset.get(), offset.get() + streamMetadata.byteLength);
                        dictionaryStreamDecoded = true;
                    } else {
                        symbolTableBuffer = data.subarray(offset.get(), offset.get() + streamMetadata.byteLength);
                    }
                    offset.add(streamMetadata.byteLength);
                    break;
            }
        }

        const childFields = (column.type.value as ComplexColumn).children;
        const stringDictionaryVectors = [];
        let i = 0;
        for (const childField of childFields) {
            const numStreams = decodeVarintInt32(data, offset, 1)[0];
            if(numStreams == 0){
                /* Column is not present in the tile */
                continue;
            }

            const columnName = `${column.name}${childField.name === StringDecoder.ROOT_COLUMN_NAME ? "" :
                (StringDecoder.NESTED_COLUMN_SEPARATOR + childField.name)}`;
            if(propertyColumnNames){
                if(!propertyColumnNames.has(columnName)){
                    //TODO: add size of sub column to Mlt for faster skipping
                    skipColumn(numStreams, data, offset);
                    continue;
                }
            }

            const childColumn = childField.type.value;
            if (numStreams !== 2 || childColumn instanceof ComplexField ||
                ((childColumn as ScalarField).type.value as ScalarType) !== ScalarType.STRING) {
                throw new Error("Currently only optional string fields are implemented for a struct.");
            }

            const presentStreamMetadata = StreamMetadataDecoder.decode(data, offset);
            const presentStream = decodeBooleanRle(data, presentStreamMetadata.numValues, offset);
            const offsetStreamMetadata = StreamMetadataDecoder.decode(data, offset);
            const isNullable = (offsetStreamMetadata instanceof RleEncodedStreamMetadata
                ? offsetStreamMetadata.numRleValues
                : offsetStreamMetadata.numValues) !== numFeatures;
            const offsetStream = isNullable
                ? IntegerStreamDecoder.decodeNullableIntStream(data, offset, offsetStreamMetadata, false,
                    new BitVector(presentStream, presentStreamMetadata.numValues))
                : IntegerStreamDecoder.decodeIntStream(data, offset, offsetStreamMetadata, false);

            stringDictionaryVectors[i++]  = symbolTableBuffer ?
                new StringFsstDictionaryVector(columnName, offsetStream, dictionaryOffsetBuffer, dictionaryBuffer,
                    symbolOffsetBuffer, symbolTableBuffer,
                    new BitVector(presentStream, presentStreamMetadata.numValues)) :
                new StringDictionaryVector(columnName, offsetStream, dictionaryOffsetBuffer, dictionaryBuffer,
                    new BitVector(presentStream, presentStreamMetadata.numValues));
        }

        return stringDictionaryVectors;
    }

    /*public static decodeSharedDictionarySequential(
        data: Uint8Array, offset: IntWrapper, column: Column): Vector[] {
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
                        dictionaryOffsetBuffer = IntegerStreamDecoder.decodeLengthStreamToOffsetBuffer(data, offset, streamMetadata);
                    } else {
                        symbolOffsetBuffer = IntegerStreamDecoder.decodeLengthStreamToOffsetBuffer(data, offset, streamMetadata);
                    }
                    break;
                case PhysicalStreamType.DATA:
                    if (DictionaryType.SINGLE === streamMetadata.logicalStreamType.dictionaryType ||
                        DictionaryType.SHARED === streamMetadata.logicalStreamType.dictionaryType) {
                        dictionaryBuffer = data.subarray(offset.get(), offset.get() + streamMetadata.byteLength);
                        dictionaryStreamDecoded = true;
                    } else {
                        symbolTableBuffer = data.subarray(offset.get(), offset.get() + streamMetadata.byteLength);
                    }
                    offset.add(streamMetadata.byteLength);
                    break;
            }
        }

        const childFields = (column.type.value as ComplexColumn).children;
        const stringDictionaryVectors = [];
        let i = 0;
        for (const childField of childFields) {
            const numStreams = decodeVarint(data, offset, 1)[0];
            if(numStreams == 0){
                /!* Column is not present in the tile *!/
                continue;
            }

            const childColumn = childField.type.value;
            if (numStreams !== 2 || childColumn instanceof ComplexField ||
                ((childColumn as ScalarField).type.value as ScalarType) !== ScalarType.STRING) {
                throw new Error("Currently only optional string fields are implemented for a struct.");
            }

            const presentStreamMetadata = StreamMetadataDecoder.decode(data, offset);
            const presentStream = decodeBooleanRle(data, presentStreamMetadata.numValues, offset);
            const offsetStreamMetadata = StreamMetadataDecoder.decode(data, offset);
            const offsetStream = IntegerStreamDecoder.decodeIntStream(data, offset, offsetStreamMetadata, false);

            const columnName = column.name + (childField.name === "default" ? "" : `:${childField.name}`);

            stringDictionaryVectors[i++]  = symbolTableBuffer ?
                new StringFsstDictionaryVector(columnName, offsetStream, dictionaryOffsetBuffer, dictionaryBuffer,
                    symbolOffsetBuffer, symbolTableBuffer,
                    new BitVector(presentStream, presentStreamMetadata.numValues)) :
                new StringDictionaryVector(columnName, offsetStream, dictionaryOffsetBuffer, dictionaryBuffer,
                    new BitVector(presentStream, presentStreamMetadata.numValues));
        }

        return stringDictionaryVectors;
    }*/

    /*public static decodeSharedDictionary(
        data: Uint8Array, offset: IntWrapper, column: Column, numFeatures: number
    ): Vector {
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
                        dictionaryOffsetBuffer = IntegerStreamDecoder.decodeLengthStreamToOffsetBuffer(data, offset, streamMetadata);
                    } else {
                        symbolOffsetBuffer = IntegerStreamDecoder.decodeLengthStreamToOffsetBuffer(data, offset, streamMetadata);
                    }
                    break;
                case PhysicalStreamType.DATA:
                    if (DictionaryType.SINGLE === streamMetadata.logicalStreamType.dictionaryType ||
                        DictionaryType.SHARED === streamMetadata.logicalStreamType.dictionaryType) {
                        dictionaryBuffer = data.subarray(offset.get(), offset.get() + streamMetadata.byteLength);
                        dictionaryStreamDecoded = true;
                    } else {
                        symbolTableBuffer = data.subarray(offset.get(), offset.get() + streamMetadata.byteLength);
                    }
                    offset.add(streamMetadata.byteLength);
                    break;
            }
        }


        const childFields = (column.type.value as ComplexColumn).children;
        const fieldVectors = new Array<DictionaryDataVector>(childFields.length);
        let i = 0;
        for (const childField of childFields) {
            const numStreams = decodeVarint(data, offset, 1)[0];

            if (numStreams !== 2 ||  childField.type instanceof ComplexField ||
                ((childField.type.value as ScalarField).type.value as ScalarType) === ScalarType.STRING) {
                throw new Error("Currently only optional string fields are implemented for a struct.");
            }

            const presentStreamMetadata = StreamMetadataDecoder.decode(data, offset);
            const presentStream = decodeBooleanRle(data, presentStreamMetadata.numValues, offset);
            const offsetStreamMetadata = StreamMetadataDecoder.decode(data, offset);
            const isNullable = (offsetStreamMetadata instanceof RleEncodedStreamMetadata
                ? offsetStreamMetadata.numRleValues
                : offsetStreamMetadata.numValues) !== numFeatures;
            const offsetStream = isNullable
                ? IntegerStreamDecoder.decodeNullableIntStream(data, offset, offsetStreamMetadata, false,
                    new BitVector(presentStream, presentStreamMetadata.numValues))
                : IntegerStreamDecoder.decodeIntStream(data, offset, offsetStreamMetadata, false);

            const columnName = column.name + (childField.name === "default" ? "" : `:${childField.name}`);
            const dataVector: DictionaryDataVector = {name: columnName, nullabilityBuffer: new BitVector(presentStream,
                    presentStreamMetadata.numValues), indexBuffer: offsetStream};
            fieldVectors[i++] = dataVector;
        }

        return symbolTableBuffer
            ? new StringSharedFsstDictionaryVector(column.name, dictionaryOffsetBuffer,
                dictionaryBuffer, symbolOffsetBuffer, symbolTableBuffer, fieldVectors)
            : new StringSharedDictionaryVector(column.name, dictionaryOffsetBuffer, dictionaryBuffer, fieldVectors);
    }*/

    /*static decode2(name: string, data: Uint8Array, offset: IntWrapper, numStreams: number, bitVector: BitVector): Vector {
        let dictionaryLengthStream: Int32Array = null;
        let offsetStream: Int32Array = null;
        let dictionaryStream: Uint8Array = null;
        let symbolLengthStream: Int32Array = null;
        let symbolTableStream: Uint8Array = null;

        for (let i = 0; i < numStreams; i++) {
            const streamMetadata = StreamMetadataDecoder.decode(data, offset);
            switch (streamMetadata.physicalStreamType) {
                case PhysicalStreamType.OFFSET:
                    offsetStream = IntegerStreamDecoder.decodeIntStream(data, offset, streamMetadata, false);
                    break;
                case PhysicalStreamType.LENGTH: {
                    const ls = IntegerStreamDecoder.decodeIntStream(data, offset, streamMetadata, false);
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
            return this.decodeFsstDictionary(name, bitVector, offsetStream, dictionaryLengthStream, dictionaryStream,
                symbolLengthStream, symbolTableStream);
        } else if (dictionaryStream) {
            return this.decodeDictionary(name, bitVector, offsetStream, dictionaryLengthStream, dictionaryStream);
        }

        return this.decodePlain(name, bitVector, offsetStream, dictionaryStream);
    }

    static decodeSharedDictionary2(data: Uint8Array, offset: IntWrapper, column: Column): Vector {
        let dictionaryLengthBuffer: Int32Array = null;
        let dictionaryBuffer: Uint8Array = null;
        let symbolLengthBuffer: Int32Array = null;
        let symbolTableBuffer: Uint8Array = null;

        let dictionaryStreamDecoded = false;
        while (!dictionaryStreamDecoded) {
            const streamMetadata = StreamMetadataDecoder.decode(data, offset);
            switch (streamMetadata.physicalStreamType) {
                case "LENGTH":
                    if (LengthType.DICTIONARY === streamMetadata.logicalStreamType.lengthType) {
                        dictionaryLengthBuffer = IntegerStreamDecoder.decodeIntStream(data, offset, streamMetadata, false);
                    } else {
                        symbolLengthBuffer = IntegerStreamDecoder.decodeIntStream(data, offset, streamMetadata, false);
                    }
                    break;
                case "DATA":
                    if (DictionaryType.SINGLE === streamMetadata.logicalStreamType.dictionaryType ||
                        DictionaryType.SHARED === streamMetadata.logicalStreamType.dictionaryType) {
                        dictionaryBuffer = data.subarray(offset.get(), offset.get() + streamMetadata.byteLength);
                        dictionaryStreamDecoded = true;
                    } else {
                        symbolTableBuffer = data.subarray(offset.get(), offset.get() + streamMetadata.byteLength);
                    }
                    offset.add(streamMetadata.byteLength);
                    break;
            }
        }

        const childFields = (column.type.value as ComplexColumn).children
        const fieldVectors = new Array<DictionaryDataVector>(childFields.length);
        let i = 0;
        for (const childField of childFields) {
            const numStreams = decodeVarint(data, offset, 1)[0];
            if (numStreams !== 2 || childField.type.value instanceof ComplexField || childField.type.value.type.value
                !== ScalarType.STRING) {
                throw new Error("Currently only optional string fields are implemented for a struct.");
            }

            const presentStreamMetadata = StreamMetadataDecoder.decode(data, offset);
            const presentStream = decodeBooleanRle(data, presentStreamMetadata.numValues, offset);
            const offsetStreamMetadata = StreamMetadataDecoder.decode(data, offset);
            const offsetStream = IntegerStreamDecoder.decodeIntStream(data, offset, offsetStreamMetadata, false);

            const columnName = column.name + (childField.name === "default" ? "" : `:${childField.name}`);
            const dataVector: DictionaryDataVector = {name: columnName, nullabilityBuffer: new BitVector(presentStream,
                    presentStreamMetadata.numValues), indexBuffer: offsetStream};
            fieldVectors[i++] = dataVector;
        }

        return symbolTableBuffer
            ? new StringSharedFsstDictionaryVector(column.name, dictionaryLengthBuffer, dictionaryBuffer,
                symbolLengthBuffer, symbolTableBuffer, fieldVectors)
            : new StringSharedDictionaryVector(column.name, dictionaryLengthBuffer, dictionaryBuffer, fieldVectors);
    }

    private static decodePlain(
        name: string, nullabilityVector: BitVector, lengthStream: Int32Array, utf8Values: Uint8Array
    ): StringFlatVector {
        return new StringFlatVector(name, lengthStream, utf8Values, nullabilityVector);
    }

    private static decodeDictionary(
        name: string, nullabilityVector: BitVector, dictionaryOffsets: Int32Array, lengthStream: Int32Array, utf8Values: Uint8Array
    ): StringDictionaryVector {
        return new StringDictionaryVector(name, dictionaryOffsets, lengthStream, utf8Values, nullabilityVector);
    }

    private static decodeFsstDictionary(
        name: string, nullabilityVector: BitVector, dictionaryOffsets: Int32Array, lengthStream: Int32Array,
        utf8Values: Uint8Array, symbolLengthStream: Int32Array, symbolTable: Uint8Array
    ): StringFsstDictionaryVector {
        return new StringFsstDictionaryVector(name, dictionaryOffsets, lengthStream, utf8Values,
            symbolLengthStream, symbolTable, nullabilityVector);
    }


    */
}