import type IntWrapper from './intWrapper';
import { type StreamMetadata } from '../metadata/tile/streamMetadata';
import {decodeDoublesLE, decodeFloatsLE} from "./decodingUtils";

export function decodeFloatStream(data: Uint8Array, offset: IntWrapper, streamMetadata: StreamMetadata): Float32Array {
        const values = decodeFloatsLE(data, offset, streamMetadata.numValues);
        const valuesList: Float32Array = new Float32Array(values.length);
        for (let i = 0; i < values.length; i++) {
            valuesList[i] = values[i];
        }
        return valuesList;
}

export function decodeDoubleStream(data: Uint8Array, offset: IntWrapper, streamMetadata: StreamMetadata): Float32Array {
    const values = decodeDoublesLE(data, offset, streamMetadata.numValues);
    const valuesList: Float32Array = new Float32Array(values.length);
    for (let i = 0; i < values.length; i++) {
        valuesList[i] = values[i];
    }
    return valuesList;
}
