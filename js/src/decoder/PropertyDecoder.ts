import { StreamMetadata } from '../metadata/stream/StreamMetadata';
import { StreamMetadataDecoder } from '../metadata/stream/StreamMetadataDecoder';
import { IntWrapper } from './IntWrapper';
import { DecodingUtils } from './DecodingUtils';
import { IntegerDecoder } from './IntegerDecoder';
import { FloatDecoder } from './FloatDecoder';
import { DoubleDecoder } from './DoubleDecoder';
import { StringDecoder } from './StringDecoder';

enum ScalarType {
    BOOLEAN = 0,
    INT_8 = 1,
    UINT_8 = 2,
    INT_32 = 3,
    UINT_32 = 4,
    INT_64 = 5,
    UINT_64 = 6,
    FLOAT = 7,
    DOUBLE = 8,
    STRING = 9
}

class PropertyDecoder {

    public static decodePropertyColumn(data: Uint8Array, offset: IntWrapper, physicalType: number, numStreams: number) {
        let presentStreamMetadata: StreamMetadata | null = null;

        if (physicalType !== undefined) {
            let presentStream = null;
            let numValues = 0;
            if (numStreams > 1) {
                presentStreamMetadata = StreamMetadataDecoder.decode(data, offset);
                numValues = presentStreamMetadata.numValues();
                presentStream = DecodingUtils.decodeBooleanRle(data, presentStreamMetadata.numValues(), presentStreamMetadata.byteLength(), offset);
            }
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
                case ScalarType.DOUBLE: {
                    const dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
                    const dataStream = DoubleDecoder.decodeDoubleStream(data, offset, dataStreamMetadata);
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
