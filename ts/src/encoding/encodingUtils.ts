import IntWrapper from "../decoding/intWrapper";

export function encodeSingleVarintInt32(value: number, dst: Uint8Array, offset: IntWrapper): void {
    let v = value;
    while (v > 0x7f) {
        dst[offset.get()] = (v & 0x7f) | 0x80;
        offset.increment();
        v >>>= 7;
    }
    dst[offset.get()] = v & 0x7f;
    offset.increment();
}

export function encodeVarintInt32Array(values: Int32Array): Uint8Array {
    const buffer = new Uint8Array(values.length * 5);
    const offset = new IntWrapper(0);

    for (const value of values) {
        encodeSingleVarintInt32(value, buffer, offset);
    }
    return buffer.slice(0, offset.get());
}

export function encodeSingleVarintInt64(value: bigint, dst: Uint8Array, offset: IntWrapper): void {
    let v = value;
    while (v > 0x7fn) {
        dst[offset.get()] = Number(v & 0x7fn) | 0x80;
        offset.increment();
        v >>= 7n;
    }
    dst[offset.get()] = Number(v & 0x7fn);
    offset.increment();
}

export function encodeVarintInt64Array(values: BigInt64Array): Uint8Array {
    const buffer = new Uint8Array(values.length * 10);
    const offset = new IntWrapper(0);

    for (const value of values) {
        encodeSingleVarintInt64(value, buffer, offset);
    }
    return buffer.slice(0, offset.get());
}
export function encodeZigZag32(value: number): number {
    return (value << 1) ^ (value >> 31);
}

export function encodeZigZag64(value: bigint): bigint {
    return (value << 1n) ^ (value >> 63n);
}

//Used for Morton encoding
export function encodeDelta(values: Int32Array): Int32Array {
    if (values.length === 0) return new Int32Array(0);

    const result = new Int32Array(values.length);
    result[0] = values[0];

    for (let i = 1; i < values.length; i++) {
        result[i] = values[i] - values[i - 1];
    }

    return result;
}

export function encodeFloatsLE(values: Float32Array): Uint8Array {
    const buffer = new Uint8Array(values.length * 4);
    const view = new DataView(buffer.buffer);

    for (let i = 0; i < values.length; i++) {
        view.setFloat32(i * 4, values[i], true);
    }

    return buffer;
}

export function encodeDoubleLE(values: Float32Array): Uint8Array {
    const buffer = new Uint8Array(values.length * 8);
    const view = new DataView(buffer.buffer);

    for (let i = 0; i < values.length; i++) {
        view.setFloat64(i * 8, values[i], true);
    }

    return buffer;
}

export function encodeBooleanRle(values: boolean[]): Uint8Array {
    // Pack booleans into bytes (8 booleans per byte)
    const numBytes = Math.ceil(values.length / 8);
    const packed = new Uint8Array(numBytes);

    for (let i = 0; i < values.length; i++) {
        if (values[i]) {
            const byteIndex = Math.floor(i / 8);
            const bitIndex = i % 8;
            packed[byteIndex] |= 1 << bitIndex;
        }
    }

    const result = new Uint8Array(1 + numBytes);
    result[0] = 256 - numBytes;
    result.set(packed, 1);

    return result;
}

export function encodeStrings(strings: string[]): Uint8Array {
    const encoder = new TextEncoder();
    const encoded = strings.map((s) => encoder.encode(s));
    const totalLength = encoded.reduce((sum, arr) => sum + arr.length, 0);
    const result = new Uint8Array(totalLength);
    let offset = 0;
    for (const arr of encoded) {
        result.set(arr, offset);
        offset += arr.length;
    }
    return result;
}

export function createStringLengths(strings: string[]): Int32Array {
    const lengths = new Int32Array(strings.length);
    const encoder = new TextEncoder();
    for (let i = 0; i < strings.length; i++) {
        lengths[i] = encoder.encode(strings[i]).length;
    }
    return lengths;
}
