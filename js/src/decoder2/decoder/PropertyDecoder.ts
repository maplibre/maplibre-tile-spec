import { StreamMetadata } from '../metadata/stream/StreamMetadata';
import { StreamMetadataDecoder } from '../metadata/stream/StreamMetadataDecoder';
import { Column, ScalarType, ScalarColumn, ComplexColumn } from "../../../src/decoder/mlt_tileset_metadata_pb";
import { IntWrapper } from './IntWrapper';
import { DecodingUtils } from './DecodingUtils';
import { IntegerDecoder } from './IntegerDecoder';
import { FloatDecoder } from './FloatDecoder';
import { StringDecoder } from './StringDecoder';

class PropertyDecoder {

    public static decodePropertyColumn(data: Uint8Array, offset: IntWrapper, column: Column, numStreams: number) {
        let presentStreamMetadata: StreamMetadata | null = null;

        // https://github.com/bufbuild/protobuf-es/blob/main/docs/runtime_api.md#accessing-oneof-groups
        const scalarColumn = column.type.case;
        if (scalarColumn !== undefined) {
            let presentStream = null;
            let numValues = 0;
            if (numStreams > 1) {
                presentStreamMetadata = StreamMetadataDecoder.decode(data, offset);
                numValues = presentStreamMetadata.numValues();
                presentStream = DecodingUtils.decodeBooleanRle(data, presentStreamMetadata.numValues(), presentStreamMetadata.byteLength(), offset);
            }
            const physicalType = column.type.value.type.value;
            switch (physicalType) {
                case ScalarType.BOOLEAN: {
                    const dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
                    const dataStream = DecodingUtils.decodeBooleanRle(data, dataStreamMetadata.numValues(), dataStreamMetadata.byteLength(), offset);
                    const booleanValues: (boolean | null)[] = new Array(presentStreamMetadata.numValues());
                    let counter = 0;
                    for (let i = 0; i < presentStreamMetadata.numValues(); i++) {
                        const value = presentStream[i] ? dataStream[counter++] : null;
                        booleanValues[i] = value !== null ? Boolean(value) : null;
                    }
                    return booleanValues;
                }
                case ScalarType.UINT_32: {
                    const dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
                    const dataStream = IntegerDecoder.decodeIntStream(data, offset, dataStreamMetadata, false);
                    const values: (number | null)[] = new Array(presentStreamMetadata.numValues());
                    let counter = 0;
                    for (let i = 0; i < presentStreamMetadata.numValues(); i++) {
                        const value = presentStream[i] ? dataStream[counter++] : null;
                        values[i] = value;
                    }
                    return values;
                }
                case ScalarType.INT_32: {
                    const dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
                    const dataStream = IntegerDecoder.decodeIntStream(data, offset, dataStreamMetadata, true);
                    const values: (number | null)[] = new Array(presentStreamMetadata.numValues());
                    let counter = 0;
                    for (let i = 0; i < presentStreamMetadata.numValues(); i++) {
                        const value = presentStream[i] ? dataStream[counter++] : null;
                        values[i] = value;
                    }
                    return values;
                }
                case ScalarType.FLOAT: {
                    const dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
                    const dataStream = FloatDecoder.decodeFloatStream(data, offset, dataStreamMetadata);
                    const values: (number | null)[] = new Array(presentStreamMetadata.numValues());
                    let counter = 0;
                    for (let i = 0; i < presentStreamMetadata.numValues(); i++) {
                        const value = presentStream[i] ? dataStream[counter++] : null;
                        values[i] = value;
                    }
                    return values;
                }
                case ScalarType.UINT_64: {
                    const dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
                    const dataStream = IntegerDecoder.decodeLongStream(data, offset, dataStreamMetadata, false);
                    const values: (bigint | null)[] = new Array(presentStreamMetadata.numValues());
                    let counter = 0;
                    for (let i = 0; i < presentStreamMetadata.numValues(); i++) {
                        const value = presentStream[i] ? dataStream[counter++] : null;
                        values[i] = value;
                    }
                    return values;
                }
                case ScalarType.INT_64: {
                    const dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
                    const dataStream = IntegerDecoder.decodeLongStream(data, offset, dataStreamMetadata, true);
                    const values: (bigint | null)[] = new Array(presentStreamMetadata.numValues());
                    let counter = 0;
                    for (let i = 0; i < presentStreamMetadata.numValues(); i++) {
                        const value = presentStream[i] ? dataStream[counter++] : null;
                        values[i] = value;
                    }
                    return values;
                }
                case ScalarType.STRING: {
                    return StringDecoder.decode(data, offset, numStreams - 1, presentStream, numValues);
                }
                default:
                    throw new Error("The specified data type for the field is currently not supported " + physicalType);
            }
        }

        if (numStreams === 1) {
            throw new Error("Present stream currently not supported for Structs.");
        } else {
            // TODO
            throw new Error("Strings are not supported yet for Structs.");
            //const result = StringDecoder.decodeSharedDictionary(data, offset, column);
            //return result.getRight();
        }
    }
}

export { PropertyDecoder };
