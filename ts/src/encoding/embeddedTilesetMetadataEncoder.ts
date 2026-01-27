import IntWrapper from "../decoding/intWrapper";
import { encodeVarintInt32Value } from "./integerEncodingUtils";
import { concatenateBuffers } from "../decoding/decodingTestUtils";

/**
 * Encodes a single typeCode as a varint.
 */
export function encodeTypeCode(typeCode: number): Uint8Array {
    const buffer = new Uint8Array(5);
    const offset = new IntWrapper(0);
    encodeVarintInt32Value(typeCode, buffer, offset);
    return buffer.slice(0, offset.get());
}

/**
 * Encodes a field name as a length-prefixed UTF-8 string.
 */
export function encodeFieldName(name: string): Uint8Array {
    const textEncoder = new TextEncoder();
    const nameBytes = textEncoder.encode(name);
    const lengthBuf = new Uint8Array(5);
    const offset = new IntWrapper(0);
    encodeVarintInt32Value(nameBytes.length, lengthBuf, offset);
    const lengthSlice = lengthBuf.slice(0, offset.get());
    return concatenateBuffers(lengthSlice, nameBytes);
}

/**
 * Encodes a child count as a varint.
 */
export function encodeChildCount(count: number): Uint8Array {
    const buffer = new Uint8Array(5);
    const offset = new IntWrapper(0);
    encodeVarintInt32Value(count, buffer, offset);
    return buffer.slice(0, offset.get());
}

/**
 * Computes typeCode for a scalar field.
 */
export function scalarTypeCode(scalarType: number, nullable: boolean): number {
    return 10 + scalarType * 2 + (nullable ? 1 : 0);
}
