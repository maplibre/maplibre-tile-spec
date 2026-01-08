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

export function concatenateBuffers(...buffers: Uint8Array[]): Uint8Array {
    const totalLength = buffers.reduce((sum, buf) => sum + buf.length, 0);
    const result = new Uint8Array(totalLength);
    let offset = 0;

    for (const buffer of buffers) {
        result.set(buffer, offset);
        offset += buffer.length;
    }

    return result;
}
