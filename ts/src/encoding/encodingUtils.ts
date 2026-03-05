export function encodeFloatsLE(values: Float32Array): Uint8Array {
    const buffer = new Uint8Array(values.length * 4);
    const view = new DataView(buffer.buffer);

    for (let i = 0; i < values.length; i++) {
        view.setFloat32(i * 4, values[i], true);
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

export function encodeByteRle(values: Uint8Array): Uint8Array {
    const result: number[] = [];
    let i = 0;

    while (i < values.length) {
        const currentByte = values[i];
        let runLength = 1;

        while (i + runLength < values.length && values[i + runLength] === currentByte && runLength < 131) {
            runLength++;
        }

        if (runLength >= 3) {
            const header = runLength - 3;
            result.push(Math.min(header, 0x7f));
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
