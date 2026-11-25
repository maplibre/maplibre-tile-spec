export function encodeFloatsLE(values: Float32Array): Uint8Array {
    const buffer = new Uint8Array(values.length * 4);
    const view = new DataView(buffer.buffer);

    for (let i = 0; i < values.length; i++) {
        view.setFloat32(i * 4, values[i], true);
    }

    return buffer;
}

export function encodeDoubleLE(values: Float64Array): Uint8Array {
    const buffer = new Uint8Array(values.length * Float64Array.BYTES_PER_ELEMENT);
    const view = new DataView(buffer.buffer);

    for (let i = 0; i < values.length; i++) {
        view.setFloat64(i * Float64Array.BYTES_PER_ELEMENT, values[i], true);
    }

    return buffer;
}

export function encodeUint32sLE(values: Uint32Array): Uint8Array {
    const buffer = new Uint8Array(values.length * Uint32Array.BYTES_PER_ELEMENT);
    const view = new DataView(buffer.buffer);
    for (let i = 0; i < values.length; i++) {
        view.setUint32(i * Uint32Array.BYTES_PER_ELEMENT, values[i], true);
    }
    return buffer;
}

export function encodeUint64sLE(values: BigUint64Array): Uint8Array {
    const buffer = new Uint8Array(values.length * BigUint64Array.BYTES_PER_ELEMENT);
    const view = new DataView(buffer.buffer);
    for (let i = 0; i < values.length; i++) {
        view.setBigUint64(i * BigUint64Array.BYTES_PER_ELEMENT, values[i], true);
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

    return encodeByteRle(packed);
}

export function encodeByteRle(values: Uint8Array): Uint8Array {
    const result: number[] = [];
    let i = 0;

    while (i < values.length) {
        const currentByte = values[i];
        let runLength = 1;

        while (i + runLength < values.length && values[i + runLength] === currentByte && runLength < 130) {
            runLength++;
        }

        if (runLength >= 3) {
            const header = runLength - 3;
            result.push(header);
            result.push(currentByte);
            i += runLength;
        } else {
            const literalStart = i;
            while (i < values.length) {
                let nextRunLength = 1;
                if (i + 1 < values.length) {
                    while (
                        i + nextRunLength < values.length &&
                        values[i + nextRunLength] === values[i] &&
                        nextRunLength < 3
                    ) {
                        nextRunLength++;
                    }
                }

                if (nextRunLength >= 3) {
                    break;
                }
                i++;

                if (i - literalStart >= 128) {
                    break;
                }
            }

            const numLiterals = i - literalStart;
            const header = 256 - numLiterals;
            result.push(header);
            for (let j = literalStart; j < i; j++) {
                result.push(values[j]);
            }
        }
    }

    return new Uint8Array(result);
}

/** The encoder every string of the module goes through. */
const textEncoder = new TextEncoder();

/**
 * The UTF-8 bytes of the strings, one after the other, and the byte length of each, written in one pass. ASCII
 * code units are copied as they are, and a string with any other goes through the `TextEncoder`, so the bytes are
 * those `TextEncoder.encode` gives.
 */
export function encodeUtf8Strings(strings: readonly string[]): { bytes: Uint8Array; lengths: Uint32Array } {
    let capacity = 0;
    for (const string of strings) capacity += string.length * 3;
    const bytes = new Uint8Array(capacity);
    const lengths = new Uint32Array(strings.length);
    let offset = 0;
    for (let i = 0; i < strings.length; i++) {
        const string = strings[i];
        const start = offset;
        let code = 0;
        for (let c = 0; c < string.length && code < 0x80; c++) {
            code = string.charCodeAt(c);
            bytes[offset++] = code;
        }
        if (code >= 0x80) offset = start + textEncoder.encodeInto(string, bytes.subarray(start)).written;
        lengths[i] = offset - start;
    }
    return { bytes: bytes.slice(0, offset), lengths };
}

export function encodeStrings(strings: string[]): Uint8Array {
    return encodeUtf8Strings(strings).bytes;
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
