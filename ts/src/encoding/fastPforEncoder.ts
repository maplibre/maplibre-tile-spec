import IntWrapper from "../decoding/intWrapper";
import type { Int32Buf, Uint8Buf } from "../decoding/fastPforSpec";
import {
    MASKS,
    DEFAULT_PAGE_SIZE,
    BLOCK_SIZE,
    greatestMultiple,
    roundUpToMultipleOf32,
    normalizePageSize,
} from "../decoding/fastPforSpec";

const OVERHEAD_OF_EACH_EXCEPT = 8;

function bits(value: number): number {
    return 32 - Math.clz32(value >>> 0);
}

function ensureInt32Capacity(buffer: Int32Buf, requiredLength: number): Int32Buf {
    if (requiredLength <= buffer.length) return buffer;

    let newLength = buffer.length === 0 ? 1 : buffer.length;
    while (newLength < requiredLength) {
        newLength *= 2;
    }

    const next = new Int32Array(newLength) as Int32Buf;
    next.set(buffer);
    return next;
}

function ensureUint8Capacity(buffer: Uint8Buf, requiredLength: number): Uint8Buf {
    if (requiredLength <= buffer.length) return buffer;

    let newLength = buffer.length === 0 ? 1 : buffer.length;
    while (newLength < requiredLength) {
        newLength *= 2;
    }

    const next = new Uint8Array(newLength) as Uint8Buf;
    next.set(buffer);
    return next;
}

/**
 * Internal workspace for the FastPFOR encoder.
 * Not exported - workspace API is kept simple for internal use.
 */
type FastPforEncoderWorkspace = {
    dataToBePacked: Int32Array[];
    dataPointers: Int32Array;
    byteContainer: Uint8Buf;
    freqs: Int32Array;
    best: Int32Array;
};

/**
 * Generic bit-packing of 32 integers, matching JavaFastPFOR BitPacking.fastpack ordering.
 * Writes exactly `bitWidth` int32 words into `out` starting at `outPos`.
 */
function fastPack32(inValues: Int32Array, inPos: number, out: Int32Buf, outPos: number, bitWidth: number): void {
    if (bitWidth === 0) return;
    if (bitWidth === 32) {
        out.set(inValues.subarray(inPos, inPos + 32), outPos);
        return;
    }

    const mask = MASKS[bitWidth] >>> 0;
    let outputWordIndex = outPos;
    let bitOffset = 0;
    let currentWord = 0;

    for (let i = 0; i < 32; i++) {
        const value = (inValues[inPos + i] >>> 0) & mask;

        if (bitOffset + bitWidth <= 32) {
            currentWord |= value << bitOffset;
            bitOffset += bitWidth;

            if (bitOffset === 32) {
                out[outputWordIndex++] = currentWord | 0;
                bitOffset = 0;
                currentWord = 0;
            }
        } else {
            const lowBits = 32 - bitOffset;
            const lowMask = MASKS[lowBits] >>> 0;
            currentWord |= (value & lowMask) << bitOffset;
            out[outputWordIndex++] = currentWord | 0;
            currentWord = value >>> lowBits;
            bitOffset = bitWidth - lowBits;
        }
    }

    if (bitOffset !== 0) {
        out[outputWordIndex] = currentWord | 0;
    }
}

const pSize = normalizePageSize(DEFAULT_PAGE_SIZE);

const initialPackedSize = (pSize / 32) * 4;
const byteContainerSize = ((3 * pSize) / BLOCK_SIZE + pSize) | 0;

function createEncoderWorkspace(): FastPforEncoderWorkspace {
    const dataToBePacked: Int32Array[] = new Array(33);
    for (let k = 1; k < dataToBePacked.length; k++) {
        dataToBePacked[k] = new Int32Array(initialPackedSize);
    }

    return {
        dataToBePacked,
        dataPointers: new Int32Array(33),
        byteContainer: new Uint8Array(byteContainerSize) as Uint8Buf,
        freqs: new Int32Array(33),
        best: new Int32Array(3),
    };
}

/**
 * Default encoder workspace, allocated lazily on first use.
 * The encoder is used primarily for tests and is not safe for concurrent calls.
 */
let defaultEncoderWorkspace: FastPforEncoderWorkspace | undefined;

function getOrCreateDefaultWorkspace(): FastPforEncoderWorkspace {
    if (!defaultEncoderWorkspace) {
        defaultEncoderWorkspace = createEncoderWorkspace();
    }
    return defaultEncoderWorkspace;
}

function getBestBFromData(inValues: Int32Array, pos: number, ws: FastPforEncoderWorkspace): void {
    const freqs = ws.freqs;
    const best = ws.best;
    freqs.fill(0);
    for (let k = pos, kEnd = pos + BLOCK_SIZE; k < kEnd; k++) {
        freqs[bits(inValues[k])]++;
    }

    let maxBits = 32;
    while (freqs[maxBits] === 0) maxBits--;

    let bestB = maxBits;
    let bestCost = maxBits * BLOCK_SIZE;
    let cExcept = 0;
    let bestCExcept = cExcept;

    for (let b = maxBits - 1; b >= 0; b--) {
        cExcept += freqs[b + 1];
        if (cExcept === BLOCK_SIZE) break;

        let thisCost = cExcept * OVERHEAD_OF_EACH_EXCEPT + cExcept * (maxBits - b) + b * BLOCK_SIZE + 8;
        if (maxBits - b === 1) thisCost -= cExcept;

        if (thisCost < bestCost) {
            bestCost = thisCost;
            bestB = b;
            bestCExcept = cExcept;
        }
    }

    best[0] = bestB;
    best[1] = bestCExcept;
    best[2] = maxBits;
}

function encodePage(
    inValues: Int32Array,
    inPos: IntWrapper,
    thisSize: number,
    out: Int32Buf,
    outPos: IntWrapper,
    ws: FastPforEncoderWorkspace,
): Int32Buf {
    const headerPos = outPos.get();
    out = ensureInt32Capacity(out, headerPos + 1);
    outPos.increment();
    let tmpOutPos = outPos.get();

    const dataPointers = ws.dataPointers;
    dataPointers.fill(0);

    let byteContainerPos = 0;

    function byteContainerPut(byteValue: number): void {
        if (byteContainerPos >= ws.byteContainer.length) {
            ws.byteContainer = ensureUint8Capacity(ws.byteContainer, byteContainerPos + 1);
        }
        ws.byteContainer[byteContainerPos++] = byteValue & 0xff;
    }

    let tmpInPos = inPos.get();
    const finalInPos = tmpInPos + thisSize - BLOCK_SIZE;

    const dataToBePacked = ws.dataToBePacked;

    for (; tmpInPos <= finalInPos; tmpInPos += BLOCK_SIZE) {
        getBestBFromData(inValues, tmpInPos, ws);

        const best = ws.best;
        const b = best[0];
        const cExcept = best[1];
        const maxBits = best[2];

        byteContainerPut(b);
        byteContainerPut(cExcept);

        if (cExcept > 0) {
            byteContainerPut(maxBits);
            const index = maxBits - b;

            if (index < 1 || index > 32) {
                throw new Error(`FastPFOR encode: invalid exception index=${index} (b=${b}, maxBits=${maxBits})`);
            }

            if (index !== 1) {
                const needed = dataPointers[index] + cExcept;
                if (needed >= dataToBePacked[index].length) {
                    let newSize = 2 * needed;
                    newSize = roundUpToMultipleOf32(newSize);
                    const next = new Int32Array(newSize);
                    next.set(dataToBePacked[index]);
                    dataToBePacked[index] = next;
                }
            }

            let realExcept = 0;
            for (let k = 0; k < BLOCK_SIZE; k++) {
                const value = inValues[tmpInPos + k] >>> 0;
                if (value >>> b !== 0) {
                    realExcept++;
                    byteContainerPut(k);
                    if (index !== 1) {
                        dataToBePacked[index][dataPointers[index]++] = (value >>> b) | 0;
                    }
                }
            }
            if (realExcept !== cExcept) {
                throw new Error(`FastPFOR encode: exception count mismatch (got ${realExcept}, expected ${cExcept})`);
            }
        }

        for (let k = 0; k < BLOCK_SIZE; k += 32) {
            out = ensureInt32Capacity(out, tmpOutPos + b);
            fastPack32(inValues, tmpInPos + k, out, tmpOutPos, b);
            tmpOutPos += b;
        }
    }

    inPos.set(tmpInPos);
    out[headerPos] = (tmpOutPos - headerPos) | 0;

    const byteSize = byteContainerPos;
    while ((byteContainerPos & 3) !== 0) byteContainerPut(0);

    out = ensureInt32Capacity(out, tmpOutPos + 1);
    out[tmpOutPos++] = byteSize | 0;

    const howManyInts = byteContainerPos / 4;
    out = ensureInt32Capacity(out, tmpOutPos + howManyInts);
    const byteContainer = ws.byteContainer;
    for (let i = 0; i < howManyInts; i++) {
        const base = i * 4;
        const v =
            byteContainer[base] |
            (byteContainer[base + 1] << 8) |
            (byteContainer[base + 2] << 16) |
            (byteContainer[base + 3] << 24) |
            0;
        out[tmpOutPos + i] = v;
    }
    tmpOutPos += howManyInts;

    let bitmap = 0;
    for (let k = 2; k <= 32; k++) {
        if (dataPointers[k] !== 0) {
            bitmap |= (k === 32) ? 0x80000000 : (1 << (k - 1));
        }
    }

    out = ensureInt32Capacity(out, tmpOutPos + 1);
    out[tmpOutPos++] = bitmap;

    for (let k = 2; k <= 32; k++) {
        const size = dataPointers[k];
        if (size !== 0) {
            out = ensureInt32Capacity(out, tmpOutPos + 1);
            out[tmpOutPos++] = size | 0;

            let j = 0;
            for (; j < size; j += 32) {
                out = ensureInt32Capacity(out, tmpOutPos + k);
                fastPack32(dataToBePacked[k], j, out, tmpOutPos, k);
                tmpOutPos += k;
            }

            const overflow = j - size;
            tmpOutPos -= ((overflow * k) >>> 5);
        }
    }

    outPos.set(tmpOutPos);
    return out;
}

function headlessEncode(
    inValues: Int32Array,
    inPos: IntWrapper,
    inLength: number,
    out: Int32Buf,
    outPos: IntWrapper,
    ws: FastPforEncoderWorkspace,
): Int32Buf {
    const alignedLength = greatestMultiple(inLength, BLOCK_SIZE);
    const finalInPos = inPos.get() + alignedLength;

    while (inPos.get() !== finalInPos) {
        const thisSize = Math.min(pSize, finalInPos - inPos.get());
        out = encodePage(inValues, inPos, thisSize, out, outPos, ws);
    }

    return out;
}

function encode(
    inValues: Int32Array,
    inPos: IntWrapper,
    inLength: number,
    out: Int32Buf,
    outPos: IntWrapper,
    ws: FastPforEncoderWorkspace,
): Int32Buf {
    const alignedLength = greatestMultiple(inLength, BLOCK_SIZE);
    out = ensureInt32Capacity(out, outPos.get() + 1);
    out[outPos.get()] = alignedLength;
    outPos.increment();

    if (alignedLength === 0) return out;
    return headlessEncode(inValues, inPos, alignedLength, out, outPos, ws);
}

/**
 * VByte encoding for FastPFOR tail values (MSB=1 terminator).
 * Note: Inverts standard Protobuf Varint (MSB=0 terminator), so we cannot reuse generic methods.
 */
function encodeVByte(
    inValues: Int32Array,
    inPos: IntWrapper,
    inLength: number,
    out: Int32Buf,
    outPos: IntWrapper,
): Int32Buf {
    if (inLength === 0) return out;

    if (inLength > 255) {
        throw new Error(`encodeVByte: inLength=${inLength} exceeds expected max of 255`);
    }
    const bytes: number[] = [];

    const start = inPos.get();
    for (let k = start; k < start + inLength; k++) {
        let v = inValues[k] >>> 0;
        while (v >= 0x80) {
            bytes.push(v & 0x7f);
            v >>>= 7;
        }
        bytes.push(v | 0x80);
    }

    while (bytes.length % 4 !== 0) bytes.push(0);

    const intsToWrite = bytes.length / 4;
    out = ensureInt32Capacity(out, outPos.get() + intsToWrite);

    let outIdx = outPos.get();
    for (let i = 0; i < bytes.length; i += 4) {
        const v = bytes[i] | (bytes[i + 1] << 8) | (bytes[i + 2] << 16) | (bytes[i + 3] << 24) | 0;
        out[outIdx++] = v;
    }

    outPos.set(outIdx);
    inPos.add(inLength);
    return out;
}

/**
 * Encodes an int32 stream using the FastPFOR wire format (pages + VByte tail).
 * Intended for tests and reference output.
 */
export function encodeFastPforInt32(values: Int32Array): Int32Buf {
    const inPos = new IntWrapper(0);
    const outPos = new IntWrapper(0);
    let out = new Int32Array(values.length + 1024) as Int32Buf;

    out = encode(values, inPos, values.length, out, outPos, getOrCreateDefaultWorkspace());

    const remaining = values.length - inPos.get();
    out = encodeVByte(values, inPos, remaining, out, outPos);

    return out.subarray(0, outPos.get());
}
