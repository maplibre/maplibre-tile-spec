import type { Int32Buf, Uint8Buf } from "../decoding/fastPforShared";
import {
    MASKS,
    DEFAULT_PAGE_SIZE,
    BLOCK_SIZE,
    greatestMultiple,
    roundUpToMultipleOf32,
    normalizePageSize,
} from "../decoding/fastPforShared";

const EXCEPTION_OVERHEAD_BITS = 8;
const MAX_BIT_WIDTH = 32;
const BIT_WIDTH_SLOTS = MAX_BIT_WIDTH + 1;
const PAGE_SIZE = normalizePageSize(DEFAULT_PAGE_SIZE);
const INITIAL_PACKED_BUFFER_SIZE_WORDS = (PAGE_SIZE / 32) * 4;
const BYTE_CONTAINER_SIZE = ((3 * PAGE_SIZE) / BLOCK_SIZE + PAGE_SIZE) | 0;

function requiredBits(value: number): number {
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
 * Exposed so callers can avoid allocations.
 * Use one workspace per concurrent encode call.
 */
export type FastPforEncoderWorkspace = {
    dataToBePacked: Array<Int32Array | undefined>;
    dataPointers: Int32Array;
    byteContainer: Uint8Buf;
    bitWidthFrequencies: Int32Array;
    bestBitWidthPlan: Int32Array;
};

export function fastPack32(inValues: Int32Array, inPos: number, out: Int32Buf, outPos: number, bitWidth: number): void {
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

export function createFastPforEncoderWorkspace(): FastPforEncoderWorkspace {
    const dataToBePacked: Array<Int32Array | undefined> = new Array(BIT_WIDTH_SLOTS);
    for (let k = 1; k < BIT_WIDTH_SLOTS; k++) {
        dataToBePacked[k] = new Int32Array(INITIAL_PACKED_BUFFER_SIZE_WORDS);
    }

    return {
        dataToBePacked,
        dataPointers: new Int32Array(BIT_WIDTH_SLOTS),
        byteContainer: new Uint8Array(BYTE_CONTAINER_SIZE) as Uint8Buf,
        bitWidthFrequencies: new Int32Array(BIT_WIDTH_SLOTS),
        bestBitWidthPlan: new Int32Array(3),
    };
}

function computeBestBitWidthPlan(inValues: Int32Array, pos: number, workspace: FastPforEncoderWorkspace): void {
    const bitWidthFrequencies = workspace.bitWidthFrequencies;
    const bestBitWidthPlan = workspace.bestBitWidthPlan;
    bitWidthFrequencies.fill(0);
    for (let k = pos, kEnd = pos + BLOCK_SIZE; k < kEnd; k++) {
        bitWidthFrequencies[requiredBits(inValues[k])]++;
    }

    let maxBitWidth = MAX_BIT_WIDTH;
    while (bitWidthFrequencies[maxBitWidth] === 0) maxBitWidth--;

    let bestBitWidth = maxBitWidth;
    let bestCost = maxBitWidth * BLOCK_SIZE;
    let exceptionCount = 0;
    let bestExceptionCount = exceptionCount;

    for (let candidateBitWidth = maxBitWidth - 1; candidateBitWidth >= 0; candidateBitWidth--) {
        exceptionCount += bitWidthFrequencies[candidateBitWidth + 1];
        if (exceptionCount === BLOCK_SIZE) break;

        let candidateCost =
            exceptionCount * EXCEPTION_OVERHEAD_BITS +
            exceptionCount * (maxBitWidth - candidateBitWidth) +
            candidateBitWidth * BLOCK_SIZE +
            8;
        if (maxBitWidth - candidateBitWidth === 1) candidateCost -= exceptionCount;

        if (candidateCost < bestCost) {
            bestCost = candidateCost;
            bestBitWidth = candidateBitWidth;
            bestExceptionCount = exceptionCount;
        }
    }

    bestBitWidthPlan[0] = bestBitWidth;
    bestBitWidthPlan[1] = bestExceptionCount;
    bestBitWidthPlan[2] = maxBitWidth;
}

function writeByte(workspace: FastPforEncoderWorkspace, byteContainerPos: number, byteValue: number): number {
    if (byteContainerPos >= workspace.byteContainer.length) {
        workspace.byteContainer = ensureUint8Capacity(workspace.byteContainer, byteContainerPos + 1);
    }
    workspace.byteContainer[byteContainerPos] = byteValue & 0xff;
    return byteContainerPos + 1;
}

function ensureExceptionValuesCapacity(
    dataToBePacked: Array<Int32Array | undefined>,
    dataPointers: Int32Array,
    exceptionBitWidth: number,
    exceptionCount: number,
): void {
    if (exceptionBitWidth === 1) return;

    const needed = dataPointers[exceptionBitWidth] + exceptionCount;
    const currentExceptionValues = dataToBePacked[exceptionBitWidth];
    if (!currentExceptionValues || needed >= currentExceptionValues.length) {
        let newSize = 2 * needed;
        newSize = roundUpToMultipleOf32(newSize);
        const next = new Int32Array(newSize);
        if (currentExceptionValues) next.set(currentExceptionValues);
        dataToBePacked[exceptionBitWidth] = next;
    }
}

function writeBlockHeader(
    workspace: FastPforEncoderWorkspace,
    byteContainerPos: number,
    bitWidth: number,
    exceptionCount: number,
    maxBitWidth: number,
): number {
    byteContainerPos = writeByte(workspace, byteContainerPos, bitWidth);
    byteContainerPos = writeByte(workspace, byteContainerPos, exceptionCount);
    if (exceptionCount > 0) {
        byteContainerPos = writeByte(workspace, byteContainerPos, maxBitWidth);
    }
    return byteContainerPos;
}

function recordBlockExceptions(
    workspace: FastPforEncoderWorkspace,
    inValues: Int32Array,
    blockPos: number,
    bitWidth: number,
    exceptionCount: number,
    exceptionBitWidth: number,
    byteContainerPos: number,
): number {
    if (exceptionCount === 0) return byteContainerPos;

    const dataToBePacked = workspace.dataToBePacked;
    const dataPointers = workspace.dataPointers;

    ensureExceptionValuesCapacity(dataToBePacked, dataPointers, exceptionBitWidth, exceptionCount);

    let actualExceptionCount = 0;
    for (let k = 0; k < BLOCK_SIZE; k++) {
        const value = inValues[blockPos + k] >>> 0;
        if (value >>> bitWidth !== 0) {
            actualExceptionCount++;
            byteContainerPos = writeByte(workspace, byteContainerPos, k);
            if (exceptionBitWidth !== 1) {
                const exceptionValues = dataToBePacked[exceptionBitWidth];
                if (!exceptionValues) {
                    throw new Error(`FastPFOR encode: missing exception buffer for bitWidth=${exceptionBitWidth}`);
                }
                exceptionValues[dataPointers[exceptionBitWidth]++] = (value >>> bitWidth) | 0;
            }
        }
    }

    if (actualExceptionCount !== exceptionCount) {
        throw new Error(
            `FastPFOR encode: exception count mismatch (got ${actualExceptionCount}, expected ${exceptionCount})`,
        );
    }

    return byteContainerPos;
}

type EncodeState = { inPos: number; out: Int32Buf; outPos: number };

function packBlock(
    inValues: Int32Array,
    blockPos: number,
    bitWidth: number,
    state: EncodeState,
): void {
    for (let k = 0; k < BLOCK_SIZE; k += 32) {
        state.out = ensureInt32Capacity(state.out, state.outPos + bitWidth);
        fastPack32(inValues, blockPos + k, state.out, state.outPos, bitWidth);
        state.outPos += bitWidth;
    }
}

function padByteContainerToInt32(workspace: FastPforEncoderWorkspace, byteContainerPos: number): number {
    while ((byteContainerPos & 3) !== 0) {
        byteContainerPos = writeByte(workspace, byteContainerPos, 0);
    }
    return byteContainerPos;
}

function writeByteContainerInts(
    workspace: FastPforEncoderWorkspace,
    state: EncodeState,
    byteContainerPos: number,
): void {
    const howManyInts = byteContainerPos / 4;
    state.out = ensureInt32Capacity(state.out, state.outPos + howManyInts);

    const byteContainer = workspace.byteContainer;
    for (let i = 0; i < howManyInts; i++) {
        const base = i * 4;
        const packedWord =
            byteContainer[base] |
            (byteContainer[base + 1] << 8) |
            (byteContainer[base + 2] << 16) |
            (byteContainer[base + 3] << 24) |
            0;
        state.out[state.outPos + i] = packedWord;
    }

    state.outPos += howManyInts;
}

function computeExceptionBitmap(dataPointers: Int32Array): number {
    let bitmap = 0;
    for (let k = 2; k <= MAX_BIT_WIDTH; k++) {
        if (dataPointers[k] !== 0) {
            bitmap |= (k === MAX_BIT_WIDTH) ? 0x80000000 : (1 << (k - 1));
        }
    }
    return bitmap;
}

function writeExceptionStreams(
    workspace: FastPforEncoderWorkspace,
    state: EncodeState,
): void {
    const dataPointers = workspace.dataPointers;
    const dataToBePacked = workspace.dataToBePacked;

    const bitmap = computeExceptionBitmap(dataPointers);
    state.out = ensureInt32Capacity(state.out, state.outPos + 1);
    state.out[state.outPos++] = bitmap;

    for (let k = 2; k <= MAX_BIT_WIDTH; k++) {
        const size = dataPointers[k];
        if (size !== 0) {
            state.out = ensureInt32Capacity(state.out, state.outPos + 1);
            state.out[state.outPos++] = size | 0;

            let j = 0;
            for (; j < size; j += 32) {
                const exceptionValues = dataToBePacked[k];
                if (!exceptionValues) {
                    throw new Error(`FastPFOR encode: missing exception stream for bitWidth=${k}`);
                }
                state.out = ensureInt32Capacity(state.out, state.outPos + k);
                fastPack32(exceptionValues, j, state.out, state.outPos, k);
                state.outPos += k;
            }

            const overflow = j - size;
            state.outPos -= ((overflow * k) >>> 5);
        }
    }
}

function encodePage(
    inValues: Int32Array,
    thisSize: number,
    state: EncodeState,
    workspace: FastPforEncoderWorkspace,
): void {
    const headerPos = state.outPos;
    state.out = ensureInt32Capacity(state.out, headerPos + 1);
    state.outPos = (state.outPos + 1) | 0;

    const dataPointers = workspace.dataPointers;
    dataPointers.fill(0);

    let byteContainerPos = 0;

    let tmpInPos = state.inPos;
    const finalInPos = tmpInPos + thisSize - BLOCK_SIZE;

    for (; tmpInPos <= finalInPos; tmpInPos += BLOCK_SIZE) {
        computeBestBitWidthPlan(inValues, tmpInPos, workspace);

        const bestBitWidthPlan = workspace.bestBitWidthPlan;
        const bitWidth = bestBitWidthPlan[0];
        const exceptionCount = bestBitWidthPlan[1];
        const maxBitWidth = bestBitWidthPlan[2];

        const exceptionBitWidth = exceptionCount > 0 ? (maxBitWidth - bitWidth) : 0;
        if (exceptionCount > 0 && (exceptionBitWidth < 1 || exceptionBitWidth > MAX_BIT_WIDTH)) {
            throw new Error(
                `FastPFOR encode: invalid exceptionBitWidth=${exceptionBitWidth} (bitWidth=${bitWidth}, maxBitWidth=${maxBitWidth})`,
            );
        }

        byteContainerPos = writeBlockHeader(workspace, byteContainerPos, bitWidth, exceptionCount, maxBitWidth);
        byteContainerPos = recordBlockExceptions(
            workspace,
            inValues,
            tmpInPos,
            bitWidth,
            exceptionCount,
            exceptionBitWidth,
            byteContainerPos,
        );

        packBlock(inValues, tmpInPos, bitWidth, state);
    }

    const pageEndOutPos = state.outPos;
    state.inPos = tmpInPos;
    state.out[headerPos] = (pageEndOutPos - headerPos) | 0;

    const byteSize = byteContainerPos;
    byteContainerPos = padByteContainerToInt32(workspace, byteContainerPos);

    state.out = ensureInt32Capacity(state.out, state.outPos + 1);
    state.out[state.outPos++] = byteSize | 0;

    writeByteContainerInts(workspace, state, byteContainerPos);

    writeExceptionStreams(workspace, state);
}

function encodeAlignedPages(
    inValues: Int32Array,
    inLength: number,
    state: EncodeState,
    workspace: FastPforEncoderWorkspace,
): void {
    const alignedLength = greatestMultiple(inLength, BLOCK_SIZE);
    const finalInPos = state.inPos + alignedLength;

    while (state.inPos !== finalInPos) {
        const thisSize = Math.min(PAGE_SIZE, finalInPos - state.inPos);
        encodePage(inValues, thisSize, state, workspace);
    }
}

function encode(
    inValues: Int32Array,
    inLength: number,
    state: EncodeState,
    workspace: FastPforEncoderWorkspace,
): void {
    const alignedLength = greatestMultiple(inLength, BLOCK_SIZE);
    state.out = ensureInt32Capacity(state.out, state.outPos + 1);
    state.out[state.outPos++] = alignedLength;

    if (alignedLength === 0) return;
    encodeAlignedPages(inValues, alignedLength, state, workspace);
}

/**
 * VByte encoding for FastPFOR tail values (MSB=1 terminator).
 * Note: Inverts standard Protobuf Varint (MSB=0 terminator), so we cannot reuse generic methods.
 */
function encodeVByte(
    inValues: Int32Array,
    inLength: number,
    state: EncodeState,
    workspace: FastPforEncoderWorkspace,
): void {
    if (inLength === 0) return;

    if (inLength > 255) {
        throw new Error(`encodeVByte: inLength=${inLength} exceeds expected max of 255`);
    }

    const requiredBytes = inLength * 5 + 3;
    workspace.byteContainer = ensureUint8Capacity(workspace.byteContainer, requiredBytes);

    const start = state.inPos;
    let bytePos = 0;
    for (let k = start; k < start + inLength; k++) {
        let value = inValues[k] >>> 0;
        while (value >= 0x80) {
            workspace.byteContainer[bytePos++] = value & 0x7f;
            value >>>= 7;
        }
        workspace.byteContainer[bytePos++] = (value | 0x80) & 0xff;
    }

    while ((bytePos & 3) !== 0) workspace.byteContainer[bytePos++] = 0;

    const intsToWrite = bytePos / 4;
    state.out = ensureInt32Capacity(state.out, state.outPos + intsToWrite);

    let outIdx = state.outPos;
    for (let i = 0; i < bytePos; i += 4) {
        const packedWord =
            workspace.byteContainer[i] |
            (workspace.byteContainer[i + 1] << 8) |
            (workspace.byteContainer[i + 2] << 16) |
            (workspace.byteContainer[i + 3] << 24) |
            0;
        state.out[outIdx++] = packedWord;
    }

    state.outPos = outIdx;
    state.inPos = (state.inPos + inLength) | 0;
}

/**
 * Encodes an int32 stream using the FastPFOR wire format (pages + VByte tail).
 */
export function encodeFastPforInt32WithWorkspace(
    values: Int32Array,
    workspace: FastPforEncoderWorkspace,
): Int32Buf {
    const state: EncodeState = { inPos: 0, outPos: 0, out: new Int32Array(values.length + 1024) as Int32Buf };

    encode(values, values.length, state, workspace);

    const remaining = values.length - state.inPos;
    encodeVByte(values, remaining, state, workspace);

    return state.out.subarray(0, state.outPos);
}
