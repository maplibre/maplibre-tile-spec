export function decodeZigZagVarint(buffer: Uint8Array, offset = 0): [value: number, offset: number] {
    const [zigZagValue, newOffset] = decodeVarint(buffer, offset);
    return [(zigZagValue >> 1) ^ -(zigZagValue & 1), newOffset];
}

/*
 * Source: https://github.com/mapbox/pbf/blob/main/index.js
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

/* Currently only works up to 53 bit  */
export function decodeLongVarints(
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

/* Currently only works up to 53 bit  */
export function decodeZigZagLongVarints(
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

/* Currently only works up to 53 bit  */
export function decodeDeltaLongVarints(
    buffer: Uint8Array,
    numValues: number,
    offset = 0,
): [values: BigInt64Array, offset: number] {
    const values = new BigInt64Array(numValues);
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

//TODO: implement next method so that not all rle values have to be duplicated
//TODO: get rid of the BigInt conversion
export function decodeRle(
    buffer: Uint8Array,
    numValues: number,
    signed = true,
    offset = 0,
): [values: BigInt64Array, offset: number] {
    const values = new BigInt64Array(numValues);

    let valuesCounter = 0;
    while (valuesCounter < numValues) {
        const header = buffer[offset++];

        /* Runs */
        if (header <= 0x7f) {
            const numRuns = header + 3;
            //TODO: get rid of that DataView construction
            const delta = new DataView(buffer.buffer).getInt8(offset++);
            //TODO: get rid of that branch
            const [firstValue, newOffset] = signed ? decodeZigZagVarint(buffer, offset) : decodeVarint(buffer, offset);
            offset = newOffset;

            for (let i = 0; i < numRuns; i++) {
                values[valuesCounter++] = BigInt(firstValue + i * delta);
            }
        } else {
            /* Literals */
            const numLiterals = 256 - header;
            for (let i = 0; i < numLiterals; i++) {
                const [value, newOffset] = signed ? decodeZigZagVarint(buffer, offset) : decodeVarint(buffer, offset);
                values[valuesCounter++] = BigInt(value);
                offset = newOffset;
            }
        }
    }

    return [values, offset];
}

export function decodeUnsignedRle(
    buffer: Uint8Array,
    numValues: number,
    offset = 0,
): [values: Uint32Array, offset: number] {
    const values = new Uint32Array(numValues);
    //TODO: get rid of that DataView construction
    const dataView = new DataView(buffer.buffer);

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

/*export function decodeRleSlow(buffer: Uint8Array, numValues: number, offset = 0): [values: number[], offset: number] {
    const values = new Array(numValues);

    let valuesCounter = 0;
    while (valuesCounter < numValues) {
        const header = buffer[offset++];

        /!* Runs *!/
        if (header <= 0x7f) {
            const numRuns = header + 3;
            const [delta, firstValueOffset] = decodeZigZagVarint(buffer, offset);
            const [firstValue, newOffset] = decodeZigZagVarint(buffer, firstValueOffset);
            offset = newOffset;

            for (let i = 0; i < numRuns; i++) {
                values[valuesCounter++] = firstValue + i * delta;
            }
        } else {
            /!* Literals *!/
            const numLiterals = 256 - header;
            for (let i = 0; i < numLiterals; i++) {
                const [value, newOffset] = decodeZigZagVarint(buffer, offset);
                values[valuesCounter++] = value;
                offset = newOffset;
            }
        }
    }

    return [values, offset];
}*/

//TODO: implement next method so that not all rle values have to be duplicated
export function decodeByteRle(buffer: Uint8Array, numBytes: number, offset = 0): [values: Uint8Array, offset: number] {
    const values = new Uint8Array(numBytes);

    let valuesCounter = 0;
    while (valuesCounter < numBytes) {
        const header = buffer[offset++];

        /* Runs */
        if (header <= 0x7f) {
            const numRuns = header + 3;
            const value = buffer[offset++];

            for (let i = 0; i < numRuns; i++) {
                values[valuesCounter++] = value;
            }
        } else {
            /* Literals */
            const numLiterals = 256 - header;
            for (let i = 0; i < numLiterals; i++) {
                values[valuesCounter++] = buffer[offset++];
            }
        }
    }

    return [values, offset];
}

export function decodeRleNext(
    buffer: Uint8Array,
    numValues: number,
    offset = 0,
): [values: BigInt64Array, offset: number] {
    const values = new BigInt64Array(numValues);

    const header = buffer[offset++];

    /* Runs */
    if (header <= 0x7f) {
        const numRuns = header + 3;
        const deltaRes = decodeZigZagVarint(buffer, offset);
        const delta = deltaRes[0];
        offset = deltaRes[1];
        const firstValueRes = decodeZigZagVarint(buffer, offset);
        const firstValue = firstValueRes[0];
        offset = firstValueRes[1];

        for (let i = 0; i < numRuns; i++) {
            values[i] = BigInt(firstValue + i * delta);
        }
    } else {
        /* Literals */
        const numLiterals = 256 - header;
        for (let i = 0; i < numLiterals; i++) {
            const valueRes = decodeZigZagVarint(buffer, offset);
            values[i] = BigInt(valueRes[0]);
            offset = valueRes[1];
        }
    }

    return [values, offset];
}

export function decodeBooleanRle(buffer: Uint8Array, numValues: number, offset = 0) {
    const [bytes, newOffset] = decodeByteRle(buffer, numValues, offset);
}

export function isBitSet(buffer: Uint8Array, index: number): boolean {
    const byteIndex = Math.floor(index / 8);
    const bitIndex = index - byteIndex * 8;
    const byteValue = buffer[byteIndex];
    return (byteValue & (2 ** bitIndex)) > 0;
}

//TODO: use a combination of TextDecoder and fromCharCode to improve performance see https://github.com/mapbox/pbf/blob/bfaaa5daea7e2a57e5e0b22cca75037113000e33/index.js#L119 and
// https://github.com/mapbox/pbf/issues/106
let decoder;
export function decodeString(buffer: Uint8Array, offset: number): [value: string, offset: number] {
    const [stringLength, newOffset] = decodeVarint(buffer, offset);
    offset = newOffset + stringLength;
    decoder ??= new TextDecoder();
    const stringSlice = buffer.subarray(newOffset, offset);
    const value = decoder.decode(stringSlice);
    return [value, offset];
}

decoder ??= new TextDecoder();
export function decodeStringDictionary(
    buffer: Uint8Array,
    offset: number,
    lengths: Uint32Array,
): [values: string[], offset: number] {
    const values = [];
    for (let i = 0; i < lengths.length; i++) {
        const length = lengths[i];
        const stringSlice = buffer.subarray(offset, offset + length);
        //TODO: replace with faster String.fromCharCode(stringSlice);
        const value = decoder.decode(stringSlice);
        values.push(value);
        offset += length;
    }

    return [values, offset];
}
/*export function decodeStringDictionary(
    buffer: Uint8Array,
    offset: number,
    lengths: BigInt64Array,
): [values: string[], offset: number] {
    const values = [];
    for (let i = 0; i < lengths.length; i++) {
        const length = lengths[i];
        const stringSlice = buffer.subarray(offset, offset + length);
        //TODO: replace with faster String.fromCharCode(stringSlice);
        const value = decoder.decode(stringSlice);
        values.push(value);
        offset += Number(length);
    }

    return [values, offset];
}*/

export function decodeDeltaVarintCoordinates(
    buffer: Uint8Array,
    numCoordinates: number,
    offset = 0,
): [vertices: Int32Array, offset: number] {
    const vertices = new Int32Array(numCoordinates * 2);

    let x = 0;
    let y = 0;
    let coordIndex = 0;
    for (let i = 0; i < numCoordinates; i++) {
        const [deltaX, nextYOffset] = decodeZigZagVarint(buffer, offset);
        const [deltaY, nextXOffset] = decodeZigZagVarint(buffer, nextYOffset);

        x += deltaX;
        y += deltaY;
        vertices[coordIndex++] = x;
        vertices[coordIndex++] = y;

        offset = nextXOffset;
    }

    return [vertices, offset];
}

export function decodeAndToVertexBuffer(
    buffer: Uint8Array,
    bufferOffset,
    numCoordinates: number,
    vertexBuffer: Uint32Array,
    vertexBufferOffset,
): [offset: number, vertexBufferOffset: number] {
    let x = 0;
    let y = 0;
    for (let i = 0; i < numCoordinates; i++) {
        const [deltaX, nextYOffset] = decodeZigZagVarint(buffer, bufferOffset);
        const [deltaY, nextXOffset] = decodeZigZagVarint(buffer, nextYOffset);
        x += deltaX;
        y += deltaY;
        vertexBuffer[vertexBufferOffset++] = x;
        vertexBuffer[vertexBufferOffset++] = y;
        bufferOffset = nextXOffset;
    }

    return [bufferOffset, vertexBufferOffset];
}

export function decodeDeltaVarintCoordinates2(
    buffer: Uint8Array,
    numCoordinates: number,
    offset = 0,
): [vertices: { x: number; y: number }[], offset: number] {
    const vertices = [];

    let x = 0;
    let y = 0;
    for (let i = 0; i < numCoordinates; i++) {
        const [deltaX, nextYOffset] = decodeZigZagVarint(buffer, offset);
        const [deltaY, nextXOffset] = decodeZigZagVarint(buffer, nextYOffset);

        x += deltaX;
        y += deltaY;
        vertices.push({ x, y });

        offset = nextXOffset;
    }

    return [vertices, offset];
}
