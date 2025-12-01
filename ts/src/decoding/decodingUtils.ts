import type IntWrapper from "./intWrapper";
import { VectorType } from "../vector/vectorType";
import BitVector from "../vector/flat/bitVector";
import { decodeStreamMetadataExtended } from "../metadata/tile/streamMetadataDecoder";

export function skipColumn(numStreams: number, tile: Uint8Array, offset: IntWrapper) {
    //TODO: add size of column in Mlt for fast skipping
    for (let i = 0; i < numStreams; i++) {
        const streamMetadata = decodeStreamMetadataExtended(tile, offset);
        offset.add(streamMetadata.byteLength);
    }
}

export function decodeBooleanRle(buffer: Uint8Array, numBooleans: number, pos: IntWrapper): Uint8Array {
    const numBytes = Math.ceil(numBooleans / 8.0);
    return decodeByteRle(buffer, numBytes, pos);
}

export function decodeNullableBooleanRle(
    buffer: Uint8Array,
    numBooleans: number,
    pos: IntWrapper,
    nullabilityBuffer: BitVector,
): Uint8Array {
    // TODO: refactor quick and dirty solution -> use solution in one pass
    const numBytes = Math.ceil(numBooleans / 8);
    const values = decodeByteRle(buffer, numBytes, pos);
    const bitVector = new BitVector(values, numBooleans);

    const size = nullabilityBuffer.size();
    const nullableBitvector = new BitVector(new Uint8Array(size), size);
    let valueCounter = 0;
    for (let i = 0; i < nullabilityBuffer.size(); i++) {
        const value = nullabilityBuffer.get(i) ? bitVector.get(valueCounter++) : false;
        nullableBitvector.set(i, value);
    }

    return nullableBitvector.getBuffer();
}

export function decodeByteRle(buffer: Uint8Array, numBytes: number, pos: IntWrapper): Uint8Array {
    const values = new Uint8Array(numBytes);

    let valueOffset = 0;
    while (valueOffset < numBytes) {
        const header = buffer[pos.increment()];

        /* Runs */
        if (header <= 0x7f) {
            const numRuns = header + 3;
            const value = buffer[pos.increment()];
            const endValueOffset = valueOffset + numRuns;
            values.fill(value, valueOffset, endValueOffset);
            valueOffset = endValueOffset;
        } else {
            /* Literals */
            const numLiterals = 256 - header;
            for (let i = 0; i < numLiterals; i++) {
                values[valueOffset++] = buffer[pos.increment()];
            }
        }
    }
    return values;
}

export function decodeFloatsLE(encodedValues: Uint8Array, pos: IntWrapper, numValues: number): Float32Array {
    const currentPos = pos.get();
    const newOffset = currentPos + numValues * Float32Array.BYTES_PER_ELEMENT;
    const newBuf = new Uint8Array(encodedValues.subarray(currentPos, newOffset)).buffer;
    const fb = new Float32Array(newBuf);
    pos.set(newOffset);
    return fb;
}

export function decodeDoublesLE(encodedValues: Uint8Array, pos: IntWrapper, numValues: number): Float64Array {
    const currentPos = pos.get();
    const newOffset = currentPos + numValues * Float64Array.BYTES_PER_ELEMENT;
    const newBuf = new Uint8Array(encodedValues.subarray(currentPos, newOffset)).buffer;
    const fb = new Float64Array(newBuf);
    pos.set(newOffset);
    return fb;
}

export function decodeNullableFloatsLE(
    encodedValues: Uint8Array,
    pos: IntWrapper,
    nullabilityBuffer: BitVector,
    numValues: number,
): Float32Array {
    const currentPos = pos.get();
    const newOffset = currentPos + numValues * Float32Array.BYTES_PER_ELEMENT;
    const newBuf = new Uint8Array(encodedValues.subarray(currentPos, newOffset)).buffer;
    const fb = new Float32Array(newBuf);
    pos.set(newOffset);

    const numTotalValues = nullabilityBuffer.size();
    const nullableFloatsBuffer = new Float32Array(numTotalValues);
    let offset = 0;
    for (let i = 0; i < numTotalValues; i++) {
        nullableFloatsBuffer[i] = nullabilityBuffer.get(i) ? fb[offset++] : 0;
    }

    return nullableFloatsBuffer;
}

export function decodeNullableDoublesLE(
    encodedValues: Uint8Array,
    pos: IntWrapper,
    nullabilityBuffer: BitVector,
    numValues: number,
): Float64Array {
    const currentPos = pos.get();
    const newOffset = currentPos + numValues * Float64Array.BYTES_PER_ELEMENT;
    const newBuf = new Uint8Array(encodedValues.subarray(currentPos, newOffset)).buffer;
    const fb = new Float64Array(newBuf);
    pos.set(newOffset);

    const numTotalValues = nullabilityBuffer.size();
    const nullableDoubleBuffer = new Float64Array(numTotalValues);
    let offset = 0;
    for (let i = 0; i < numTotalValues; i++) {
        nullableDoubleBuffer[i] = nullabilityBuffer.get(i) ? fb[offset++] : 0;
    }

    return nullableDoubleBuffer;
}

const TEXT_DECODER_MIN_LENGTH = 12;
const utf8TextDecoder = new TextDecoder();

// Source: https://github.com/mapbox/pbf/issues/106
export function decodeString(buf: Uint8Array, pos: number, end: number): string {
    if (end - pos >= TEXT_DECODER_MIN_LENGTH) {
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

export function getVectorTypeBooleanStream(
    numFeatures: number,
    byteLength: number,
    data: Uint8Array,
    offset: IntWrapper,
): VectorType {
    const valuesPerRun = 0x83;
    // TODO: use VectorType metadata field for to test which VectorType is used
    return Math.ceil(numFeatures / valuesPerRun) * 2 == byteLength &&
        /* Test the first value byte if all bits are set to true */
        (data[offset.get() + 1] & 0xff) === (bitCount(numFeatures) << 2) - 1
        ? VectorType.CONST
        : VectorType.FLAT;
}

function bitCount(number): number {
    //TODO: refactor to get rid of special case handling
    return number === 0 ? 1 : Math.floor(Math.log2(number) + 1);
}
