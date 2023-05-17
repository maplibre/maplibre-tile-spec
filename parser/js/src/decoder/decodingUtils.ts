export function decodeStringField(buffer: Uint8Array, offset: number): [value: string, offset: number] {
    const [stringLength, strStartOffset] = decodeVarint(buffer, offset);
    offset = strStartOffset + stringLength;
    const value = decodeString(buffer, strStartOffset, offset);
    return [value, offset];
}

// Source of string decoding: https://github.com/mapbox/pbf/issues/106
const TEXT_DECODER_MIN_LENGTH = 12;
const utf8TextDecoder = typeof TextDecoder === "undefined" ? null : new TextDecoder("utf-8");

export function decodeString(buf: Uint8Array, pos: number, end: number): string {
    if (end - pos >= TEXT_DECODER_MIN_LENGTH && utf8TextDecoder) {
        // longer strings are fast with the built-in browser TextDecoder API
        return utf8TextDecoder.decode(buf.subarray(pos, end));
    }
    // short strings are fast with custom implementation
    return readUtf8(buf, pos, end);
}

function readUtf8(buf, pos, end): string {
    let str = "";
    let i = pos;

    while (i < end) {
        const b0 = buf[i];
        let c = null; // codepoint
        let bytesPerSequence = b0 > 0xef ? 4 : b0 > 0xdf ? 3 : b0 > 0xbf ? 2 : 1;

        if (i + bytesPerSequence > end) break;

        let b1, b2, b3;

        if (bytesPerSequence === 1) {
            if (b0 < 0x80) {
                c = b0;
            }
        } else if (bytesPerSequence === 2) {
            b1 = buf[i + 1];
            if ((b1 & 0xc0) === 0x80) {
                c = ((b0 & 0x1f) << 0x6) | (b1 & 0x3f);
                if (c <= 0x7f) {
                    c = null;
                }
            }
        } else if (bytesPerSequence === 3) {
            b1 = buf[i + 1];
            b2 = buf[i + 2];
            if ((b1 & 0xc0) === 0x80 && (b2 & 0xc0) === 0x80) {
                c = ((b0 & 0xf) << 0xc) | ((b1 & 0x3f) << 0x6) | (b2 & 0x3f);
                if (c <= 0x7ff || (c >= 0xd800 && c <= 0xdfff)) {
                    c = null;
                }
            }
        } else if (bytesPerSequence === 4) {
            b1 = buf[i + 1];
            b2 = buf[i + 2];
            b3 = buf[i + 3];
            if ((b1 & 0xc0) === 0x80 && (b2 & 0xc0) === 0x80 && (b3 & 0xc0) === 0x80) {
                c = ((b0 & 0xf) << 0x12) | ((b1 & 0x3f) << 0xc) | ((b2 & 0x3f) << 0x6) | (b3 & 0x3f);
                if (c <= 0xffff || c >= 0x110000) {
                    c = null;
                }
            }
        }

        if (c === null) {
            c = 0xfffd;
            bytesPerSequence = 1;
        } else if (c > 0xffff) {
            c -= 0x10000;
            str += String.fromCharCode(((c >>> 10) & 0x3ff) | 0xd800);
            c = 0xdc00 | (c & 0x3ff);
        }

        str += String.fromCharCode(c);
        i += bytesPerSequence;
    }

    return str;
}

export function decodeUint64Varints(
    buffer: Uint8Array,
    numValues: number,
    offset = 0,
): [values: BigUint64Array, offset: number] {
    const values = new BigUint64Array(numValues);
    for (let i = 0; i < numValues; i++) {
        const [value, newOffset] = decodeVarint(buffer, offset);
        values[i] = BigInt(value);
        offset = newOffset;
    }

    return [values, offset];
}

export function decodeInt64Varints(
    buffer: Uint8Array,
    numValues: number,
    offset = 0,
): [values: BigInt64Array, offset: number] {
    const values = new BigInt64Array(numValues);
    for (let i = 0; i < numValues; i++) {
        const [value, newOffset] = decodeZigZagVarint(buffer, offset);
        values[i] = BigInt(value);
        offset = newOffset;
    }

    return [values, offset];
}

/*
 * Source of varint decoding: https://github.com/mapbox/pbf/blob/main/index.js
 * Base 128 Varint from Protocol Buffers (data in little endian)
 */
//TODO: fix -> currently only handles up to 53 bits
export function decodeVarint(buffer: Uint8Array, offset = 0): [value: number, offset: number] {
    let value, b;

    b = buffer[offset++];
    value = b & 0x7f;
    if (b < 0x80) return [value, offset];
    b = buffer[offset++];
    value |= (b & 0x7f) << 7;
    if (b < 0x80) return [value, offset];
    b = buffer[offset++];
    value |= (b & 0x7f) << 14;
    if (b < 0x80) return [value, offset];
    b = buffer[offset++];
    value |= (b & 0x7f) << 21;
    if (b < 0x80) return [value, offset];
    b = buffer[offset];
    value |= (b & 0x0f) << 28;

    return decodeVarintRemainder(buffer, offset, value);
}

function decodeVarintRemainder(buffer: Uint8Array, offset: number, low: number): [value: number, offset: number] {
    let b, high;

    b = buffer[offset++];
    high = (b & 0x70) >> 4;
    if (b < 0x80) return [toNum(low, high), offset];
    b = buffer[offset++];
    high |= (b & 0x7f) << 3;
    if (b < 0x80) return [toNum(low, high), offset];
    b = buffer[offset++];
    high |= (b & 0x7f) << 10;
    if (b < 0x80) return [toNum(low, high), offset];
    b = buffer[offset++];
    high |= (b & 0x7f) << 17;
    if (b < 0x80) return [toNum(low, high), offset];
    b = buffer[offset++];
    high |= (b & 0x7f) << 24;
    if (b < 0x80) return [toNum(low, high), 9];
    b = buffer[offset++];
    high |= (b & 0x01) << 31;
    if (b < 0x80) return [toNum(low, high), 10];

    throw new Error("Expected varint not more than 10 bytes");
}

function toNum(low, high) {
    return (high >>> 0) * 0x100000000 + (low >>> 0);
}

export function decodeDeltaVarints(
    buffer: Uint8Array,
    numValues: number,
    offset = 0,
): [values: Uint32Array, offset: number] {
    const values = new Uint32Array(numValues);
    let previousValue = 0;
    for (let i = 0; i < numValues; i++) {
        const [delta, newOffset] = decodeZigZagVarint(buffer, offset);
        const value = previousValue + delta;
        values[i] = value;

        previousValue = value;
        offset = newOffset;
    }

    return [values, offset];
}

export function decodeDeltaUint64Varints(
    buffer: Uint8Array,
    numValues: number,
    offset = 0,
): [values: BigUint64Array, offset: number] {
    const values = new BigUint64Array(numValues);
    let previousValue = 0;
    for (let i = 0; i < numValues; i++) {
        const [delta, newOffset] = decodeZigZagVarint(buffer, offset);
        const value = previousValue + delta;
        values[i] = BigInt(value);

        previousValue = value;
        offset = newOffset;
    }

    return [values, offset];
}

export function decodeDeltaNumberVarints(
    buffer: Uint8Array,
    numValues: number,
    offset = 0,
): [values: number[], offset: number] {
    const values = new Array(numValues);
    let previousValue = 0;
    for (let i = 0; i < numValues; i++) {
        const [delta, newOffset] = decodeZigZagVarint(buffer, offset);
        const value = previousValue + delta;
        values[i] = value;

        previousValue = value;
        offset = newOffset;
    }

    return [values, offset];
}

export function decodeZigZagVarint(buffer: Uint8Array, offset = 0): [value: number, offset: number] {
    const [zigZagValue, newOffset] = decodeVarint(buffer, offset);
    return [(zigZagValue >> 1) ^ -(zigZagValue & 1), newOffset];
}

export function decodeUint32Rle(
    buffer: Uint8Array,
    numValues: number,
    offset = 0,
): [values: Uint32Array, offset: number] {
    //TODO: get rid of that DataView construction
    const dataView = new DataView(buffer.buffer);
    const values = new Uint32Array(numValues);

    let valuesCounter = 0;
    while (valuesCounter < numValues) {
        const header = buffer[offset++];

        /* Runs */
        if (header <= 0x7f) {
            const numRuns = header + 3;
            const delta = dataView.getInt8(offset++);
            const [firstValue, newOffset] = decodeVarint(buffer, offset);
            offset = newOffset;

            for (let i = 0; i < numRuns; i++) {
                values[valuesCounter++] = firstValue + i * delta;
            }
        } else {
            /* Literals */
            const numLiterals = 256 - header;
            for (let i = 0; i < numLiterals; i++) {
                const [value, newOffset] = decodeVarint(buffer, offset);
                values[valuesCounter++] = value;
                offset = newOffset;
            }
        }
    }

    return [values, offset];
}

//TODO: implement next method so that not all rle values have to be duplicated
export function decodeInt64Rle(
    buffer: Uint8Array,
    numValues: number,
    offset = 0,
): [values: BigInt64Array, offset: number] {
    const dataView = new DataView(buffer.buffer);
    const values = new BigInt64Array(numValues);

    let valuesCounter = 0;
    while (valuesCounter < numValues) {
        const header = buffer[offset++];

        /* Runs */
        if (header <= 0x7f) {
            const numRuns = header + 3;
            const delta = dataView.getInt8(offset++);
            const [firstValue, newOffset] = decodeZigZagVarint(buffer, offset);
            offset = newOffset;

            for (let i = 0; i < numRuns; i++) {
                values[valuesCounter++] = BigInt(firstValue + i * delta);
            }
        } else {
            /* Literals */
            const numLiterals = 256 - header;
            for (let i = 0; i < numLiterals; i++) {
                const [value, newOffset] = decodeZigZagVarint(buffer, offset);
                values[valuesCounter++] = BigInt(value);
                offset = newOffset;
            }
        }
    }

    return [values, offset];
}

export function decodeUInt64Rle(
    buffer: Uint8Array,
    numValues: number,
    offset = 0,
): [values: BigUint64Array, offset: number] {
    const dataView = new DataView(buffer.buffer);
    const values = new BigUint64Array(numValues);

    let valuesCounter = 0;
    while (valuesCounter < numValues) {
        const header = buffer[offset++];

        /* Runs */
        if (header <= 0x7f) {
            const numRuns = header + 3;
            const delta = dataView.getInt8(offset++);
            const [firstValue, newOffset] = decodeVarint(buffer, offset);
            offset = newOffset;

            for (let i = 0; i < numRuns; i++) {
                values[valuesCounter++] = BigInt(firstValue + i * delta);
            }
        } else {
            /* Literals */
            const numLiterals = 256 - header;
            for (let i = 0; i < numLiterals; i++) {
                const [value, newOffset] = decodeVarint(buffer, offset);
                values[valuesCounter++] = BigInt(value);
                offset = newOffset;
            }
        }
    }

    return [values, offset];
}

export function decodeNumberRle(buffer: Uint8Array, numValues: number, offset = 0): [values: number[], offset: number] {
    const dataView = new DataView(buffer.buffer);
    const values = new Array(numValues);

    let valuesCounter = 0;
    while (valuesCounter < numValues) {
        const header = buffer[offset++];

        /* Runs */
        if (header <= 0x7f) {
            const numRuns = header + 3;
            const delta = dataView.getInt8(offset++);
            const [firstValue, newOffset] = decodeVarint(buffer, offset);
            offset = newOffset;

            for (let i = 0; i < numRuns; i++) {
                values[valuesCounter++] = firstValue + i * delta;
            }
        } else {
            /* Literals */
            const numLiterals = 256 - header;
            for (let i = 0; i < numLiterals; i++) {
                const [value, newOffset] = decodeVarint(buffer, offset);
                values[valuesCounter++] = value;
                offset = newOffset;
            }
        }
    }

    return [values, offset];
}

//TODO: implement next method so that not all rle values have to be duplicated
export function decodeByteRle(buffer: Uint8Array, numBytes: number, offset = 0): [values: Uint8Array, offset: number] {
    const values = new Uint8Array(numBytes);

    let valueOffset = 0;
    while (valueOffset < numBytes) {
        const header = buffer[offset++];

        /* Runs */
        if (header <= 0x7f) {
            const numRuns = header + 3;
            const value = buffer[offset++];
            const endValueOffset = valueOffset + numRuns;
            values.fill(value, valueOffset, endValueOffset);
            valueOffset = endValueOffset;
        } else {
            /* Literals */
            const numLiterals = 256 - header;
            for (let i = 0; i < numLiterals; i++) {
                values[valueOffset++] = buffer[offset++];
            }
        }
    }

    return [values, offset];
}

//TODO: optimize
export function isBitSet(buffer: Uint8Array, index: number): boolean {
    const byteIndex = Math.floor(index / 8);
    const bitIndex = index - byteIndex * 8;
    const byteValue = buffer[byteIndex];
    return (byteValue & (2 ** bitIndex)) > 0;
}
