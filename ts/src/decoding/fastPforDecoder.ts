import IntWrapper from "./intWrapper";

/**
 * Decode FastPFOR compressed integer stream.
 *
 * @param data - Byte array containing encoded stream
 * @param numValues - Expected number of decoded values
 * @param byteLength - Length of encoded data in bytes
 * @param offset - Current position in data (updated after decoding)
 * @returns Decoded int32 array
 */
export function decodeFastPfor(
    data: Uint8Array,
    numValues: number,
    byteLength: number,
    offset: IntWrapper,
): Int32Array {
    const startOffset = offset.get();
    const encodedValuesSlice = data.slice(startOffset, startOffset + byteLength);

    // Convert bytes to int32 array (big-endian)
    const dataView = new DataView(
        encodedValuesSlice.buffer,
        encodedValuesSlice.byteOffset,
        encodedValuesSlice.byteLength,
    );
    const numCompleteInts = Math.floor(byteLength / 4);
    const hasTrailingBytes = byteLength % 4 !== 0;
    const numInts = hasTrailingBytes ? numCompleteInts + 1 : numCompleteInts;

    const intValues = new Int32Array(numInts);
    for (let i = 0; i < numCompleteInts; i++) {
        intValues[i] = dataView.getInt32(i * 4, false);
    }

    if (hasTrailingBytes) {
        intValues[numCompleteInts] = 0;
    }

    // Decompress: FastPFOR handles blocks of 256, VariableByte handles remainder
    const decodedValues = new Int32Array(numValues);
    const inputPos = new IntWrapper(0);
    const outputPos = new IntWrapper(0);

    compositionUncompress(intValues, inputPos, intValues.length, decodedValues, outputPos, numValues);

    // Advance offset by padded length (4-byte aligned)
    const paddedByteLength = Math.ceil(byteLength / 4) * 4;
    offset.add(paddedByteLength);
    return decodedValues;
}

/** Composition codec: FastPFOR for blocks of 256, VariableByte for remainder */
function compositionUncompress(
    input: Int32Array,
    inpos: IntWrapper,
    inlength: number,
    output: Int32Array,
    outpos: IntWrapper,
    numValues: number,
): void {
    const init = inpos.get();

    fastPforUncompress(input, inpos, inlength, output, outpos);

    const consumedInt32s = inpos.get() - init;
    const remainingInt32s = inlength - consumedInt32s;
    const remainingValues = numValues - outpos.get();

    if (remainingInt32s > 0 && remainingValues > 0) {
        variableByteUncompress(input, inpos, remainingInt32s, output, outpos, remainingValues);
    }
}

/** Read FastPFOR stream header and decode pages */
function fastPforUncompress(
    input: Int32Array,
    inpos: IntWrapper,
    inlength: number,
    output: Int32Array,
    outpos: IntWrapper,
): void {
    if (inlength === 0) return;

    const outlength = input[inpos.get()];
    inpos.increment();

    if (outlength === 0) return;

    headlessUncompress(input, inpos, inlength - 1, output, outpos, outlength);
}

const BLOCK_SIZE = 256;
const DEFAULT_PAGE_SIZE = 65536;

/** Process FastPFOR pages (up to 65536 values each) */
function headlessUncompress(
    input: Int32Array,
    inpos: IntWrapper,
    inlength: number,
    output: Int32Array,
    outpos: IntWrapper,
    outlength: number,
): void {
    let remaining = outlength;
    let iteration = 0;

    while (remaining > 0) {
        const thissize = Math.min(remaining, DEFAULT_PAGE_SIZE);
        decodePage(input, inpos, output, outpos, thissize);
        remaining -= thissize;
        iteration++;
    }
}

/** Decode a single FastPFOR page: read metadata, unpack blocks, apply exceptions */
function decodePage(
    input: Int32Array,
    inpos: IntWrapper,
    output: Int32Array,
    outpos: IntWrapper,
    thissize: number,
): void {
    const initpos = inpos.get();
    const wheremeta = input[inpos.get()];
    inpos.increment();

    let inexcept = initpos + wheremeta;

    const bytesize = input[inexcept++] >>> 0;

    const byteContainer = new Uint8Array(bytesize);
    const bytesNeeded = Math.ceil(bytesize / 4) * 4;
    const byteOffset = input.byteOffset + inexcept * 4;

    if (byteOffset + bytesNeeded <= input.buffer.byteLength) {
        const srcBytes = new Uint8Array(input.buffer, byteOffset, bytesNeeded);
        byteContainer.set(srcBytes.subarray(0, bytesize));
    } else {
        for (let i = 0; i < bytesize; i++) {
            const intIdx = inexcept + Math.floor(i / 4);
            const byteInInt = i % 4;
            byteContainer[i] = (input[intIdx] >>> (byteInInt * 8)) & 0xff;
        }
    }

    inexcept += Math.ceil(bytesize / 4);

    const bitmap = input[inexcept++];

    // Unpack exception arrays for each bit width
    const dataTobePacked: Int32Array[] = new Array(33);
    const dataPointers: number[] = new Array(33).fill(0);

    for (let k = 2; k <= 32; k++) {
        if ((bitmap & (1 << (k - 1))) !== 0) {
            const size = input[inexcept++];

            if (size < 0 || size > 1000000) {
                throw new Error(`Invalid exception size ${size} for k=${k} at position ${inexcept - 1}`);
            }

            dataTobePacked[k] = new Int32Array(size);

            const tmpInpos = new IntWrapper(inexcept);
            fastunpack(input, tmpInpos, dataTobePacked[k], 0, k);
            inexcept = tmpInpos.get();
        }
    }

    let bytePosIn = 0;
    const numBlocks = thissize / BLOCK_SIZE;

    for (let run = 0; run < numBlocks; run++) {
        const b = byteContainer[bytePosIn++] & 0xff;
        const cexcept = byteContainer[bytePosIn++] & 0xff;
        const tmpoutpos = outpos.get() + run * BLOCK_SIZE;

        if (b > 0) {
            fastunpack(input, inpos, output, tmpoutpos, b, BLOCK_SIZE);
        } else {
            output.fill(0, tmpoutpos, tmpoutpos + BLOCK_SIZE);
        }

        if (cexcept > 0) {
            const maxbits = byteContainer[bytePosIn++] & 0xff;
            const index = maxbits - b;

            if (index === 1) {
                for (let k = 0; k < cexcept; k++) {
                    const pos = byteContainer[bytePosIn++] & 0xff;
                    output[pos + tmpoutpos] |= 1 << b;
                }
            } else {
                for (let k = 0; k < cexcept; k++) {
                    const pos = byteContainer[bytePosIn++] & 0xff;
                    const exceptvalue = dataTobePacked[index][dataPointers[index]++];
                    output[pos + tmpoutpos] |= exceptvalue << b;
                }
            }
        }
    }

    inpos.set(inexcept);
    outpos.add(thissize);
}

/**
 * Unpack bit-packed integers from stream.
 * Uses dual 32-bit buffer to avoid overflow when shifting >= 32 bits.
 */
function fastunpack(
    input: Int32Array,
    inpos: IntWrapper,
    output: Int32Array,
    outOffset: number,
    bit: number,
    numValues?: number,
): void {
    if (bit === 0) return;

    const valuesToUnpack = numValues !== undefined ? numValues : output.length - outOffset;
    let inIdx = inpos.get();

    let bufferLow = 0;
    let bufferHigh = 0;
    let bitsInBuffer = 0;
    const mask = (1 << bit) - 1;

    for (let i = 0; i < valuesToUnpack; i++) {
        while (bitsInBuffer < bit) {
            if (inIdx >= input.length) break;

            const val = input[inIdx++] >>> 0;

            if (bitsInBuffer === 0) {
                bufferLow = val;
                bitsInBuffer = 32;
            } else {
                const shift = bitsInBuffer;
                bufferHigh = val >>> (32 - shift);
                bufferLow = (bufferLow | ((val << shift) >>> 0)) >>> 0;
                bitsInBuffer += 32;
            }
        }

        if (bitsInBuffer >= bit) {
            output[outOffset + i] = bufferLow & mask;

            if (bit < 32) {
                bufferLow = ((bufferLow >>> bit) | (bufferHigh << (32 - bit))) >>> 0;
                bufferHigh = bufferHigh >>> bit;
            } else {
                bufferLow = bufferHigh >>> (bit - 32);
                bufferHigh = 0;
            }
            bitsInBuffer -= bit;
        } else {
            output[outOffset + i] = bufferLow & mask;
            bufferLow = 0;
            bufferHigh = 0;
            bitsInBuffer = 0;
        }
    }
    inpos.set(inIdx);
}

/**
 * VariableByte decoder.
 * Bytes packed in int32 array (little-endian), 7 bits per byte, 0x80 termination bit.
 */
function variableByteUncompress(
    input: Int32Array,
    inpos: IntWrapper,
    inlength: number,
    output: Int32Array,
    outpos: IntWrapper,
    numValues: number,
): void {
    const intIdx = inpos.get();
    let outIdx = outpos.get();
    let decodedCount = 0;
    const maxIntIdx = inpos.get() + inlength;

    let s = 0;
    let p = intIdx;
    let v = 0;
    let shift = 0;

    while (decodedCount < numValues && p < maxIntIdx) {
        const val = input[p];
        const c = (val >>> s) & 0xff;

        s += 8;
        p += s >> 5;
        s = s & 31;

        v += (c & 0x7f) << shift;

        if ((c & 0x80) === 0x80) {
            output[outIdx++] = v;
            v = 0;
            shift = 0;
            decodedCount++;
        } else {
            shift += 7;
        }
    }

    while (decodedCount < numValues) {
        output[outIdx++] = 0;
        decodedCount++;
    }

    inpos.add(inlength);
    outpos.set(outIdx);
}
