import { IntWrapper } from './IntWrapper';
import { StreamMetadata } from '../metadata/stream/StreamMetadata';
import { DecodingUtils } from './DecodingUtils';

class FloatDecoder {
    private constructor() {}

    public static decodeFloatStream(data: Uint8Array, offset: IntWrapper, streamMetadata: StreamMetadata): Float32Array {
        const values = DecodingUtils.decodeFloatsLE(data, offset, streamMetadata.numValues());
        const valuesList: Float32Array = new Float32Array(values.length);
        for (let i = 0; i < values.length; i++) {
            valuesList[i] = values[i];
        }
        return valuesList;
    }
}

export { FloatDecoder };
