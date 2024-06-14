import { IntWrapper } from './IntWrapper';
import { StreamMetadata } from '../metadata/stream/StreamMetadata';
import { DecodingUtils } from './DecodingUtils';

export class DoubleDecoder {
    public static decodeDoubleStream(data: Uint8Array, offset: IntWrapper, streamMetadata: StreamMetadata): Float64Array {
        return DecodingUtils.decodeDoublesLE(data, offset, streamMetadata.numValues());
    }
}
