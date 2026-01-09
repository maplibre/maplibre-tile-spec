import type { Int32Buf, Uint8Buf } from "./fastPforShared";
import {
    MASKS,
    DEFAULT_PAGE_SIZE,
    BLOCK_SIZE,
    greatestMultiple,
    roundUpToMultipleOf32,
    normalizePageSize,
    IS_LE,
} from "./fastPforShared";
import {
    fastUnpack32_2,
    fastUnpack32_3,
    fastUnpack32_4,
    fastUnpack32_5,
    fastUnpack32_6,
    fastUnpack32_7,
    fastUnpack32_8,
    fastUnpack32_9,
    fastUnpack32_10,
    fastUnpack32_11,
    fastUnpack32_12,
    fastUnpack32_16,
    fastUnpack256_1,
    fastUnpack256_2,
    fastUnpack256_3,
    fastUnpack256_4,
    fastUnpack256_5,
    fastUnpack256_6,
    fastUnpack256_7,
    fastUnpack256_8,
    fastUnpack256_16,
    fastUnpack256_Generic,
} from "./fastPforUnpack";

/**
 * Workspace for the FastPFOR decoder.
 */
export type FastPforDecoderWorkspace = {
    dataToBePacked: Array<Int32Array | undefined>;
    dataPointers: Int32Array;
    byteContainer: Uint8Buf;
    byteContainerI32?: Int32Array;
    exceptionSizes: Int32Array;
};

const PAGE_SIZE = normalizePageSize(DEFAULT_PAGE_SIZE);
const BYTE_CONTAINER_SIZE = ((3 * PAGE_SIZE) / BLOCK_SIZE + PAGE_SIZE) | 0;

/**
 * Creates an isolated workspace for decoding.
 * Reusing a workspace across calls avoids repeated allocations.
 */
export function createDecoderWorkspace(): FastPforDecoderWorkspace {
    const byteContainer = new Uint8Array(BYTE_CONTAINER_SIZE) as Uint8Buf;
    return {
        dataToBePacked: new Array(33),
        dataPointers: new Int32Array(33),
        byteContainer,
        byteContainerI32: IS_LE ? new Int32Array(byteContainer.buffer, byteContainer.byteOffset, byteContainer.byteLength >>> 2) : undefined,
        exceptionSizes: new Int32Array(33),
    };
}

function materializeByteContainer(
    inValues: Int32Array,
    byteContainerStart: number,
    byteSize: number,
    ws: FastPforDecoderWorkspace,
): Uint8Buf {
    if (ws.byteContainer.length < byteSize) {
        ws.byteContainer = new Uint8Array(byteSize * 2) as Uint8Buf;
        ws.byteContainerI32 = undefined;
    }
    const byteContainer = ws.byteContainer;
    const numFullInts = byteSize >>> 2;

    if (IS_LE && (byteContainer.byteOffset & 3) === 0) {
        let intView = ws.byteContainerI32;
        if (
            !intView ||
            intView.buffer !== byteContainer.buffer ||
            intView.byteOffset !== byteContainer.byteOffset ||
            intView.length < numFullInts
        ) {
            intView = ws.byteContainerI32 = new Int32Array(
                byteContainer.buffer,
                byteContainer.byteOffset,
                byteContainer.byteLength >>> 2,
            );
        }

        intView.set(inValues.subarray(byteContainerStart, byteContainerStart + numFullInts));
    } else {
        for (let i = 0; i < numFullInts; i = (i + 1) | 0) {
            const val = inValues[(byteContainerStart + i) | 0] | 0;
            const base = i << 2;
            byteContainer[base] = val & 0xff;
            byteContainer[(base + 1) | 0] = (val >>> 8) & 0xff;
            byteContainer[(base + 2) | 0] = (val >>> 16) & 0xff;
            byteContainer[(base + 3) | 0] = (val >>> 24) & 0xff;
        }
    }

    const remainder = byteSize & 3;
    if (remainder > 0) {
        const lastIntIdx = (byteContainerStart + numFullInts) | 0;
        const lastVal = inValues[lastIntIdx] | 0;
        const base = numFullInts << 2;
        for (let r = 0; r < remainder; r = (r + 1) | 0) {
            byteContainer[(base + r) | 0] = (lastVal >>> (r << 3)) & 0xff;
        }
    }

    return byteContainer;
}

function unpackExceptionStreams(
    inValues: Int32Array,
    inExcept: number,
    ws: FastPforDecoderWorkspace,
): number {
    const bitmap = inValues[inExcept++] | 0;
    const dtp = ws.dataToBePacked;

    for (let k = 2; k <= 32; k = (k + 1) | 0) {
        if (((bitmap >>> (k - 1)) & 1) !== 0) {
            const size = inValues[inExcept++] >>> 0;
            const roundedUp = roundUpToMultipleOf32(size);

            const wordsNeeded = ((size * k) + 31) >>> 5;
            if (inExcept + wordsNeeded > inValues.length) {
                throw new Error(
                    `FastPFOR decode: truncated exception stream (bitWidth=${k}, size=${size}, needWords=${wordsNeeded}, availableWords=${inValues.length - inExcept})`,
                );
            }

            let buf = dtp[k];
            if (!buf || buf.length < roundedUp) {
                buf = dtp[k] = new Int32Array(roundedUp);
            }

            const dtpk = buf;
            let j = 0;
            for (; j < size; j = (j + 32) | 0) {
                fastUnpack32(inValues, inExcept, dtpk, j, k);
                inExcept = (inExcept + k) | 0;
            }

            const overflow = (j - size) | 0;
            inExcept = (inExcept - ((overflow * k) >>> 5)) | 0;

            ws.exceptionSizes[k] = size;
        }
    }

    return inExcept;
}

function unpackBlock256(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number, bitWidth: number): number {
    switch (bitWidth) {
        case 1:
            fastUnpack256_1(inValues, inPos, out, outPos);
            return (inPos + 8) | 0;
        case 2:
            fastUnpack256_2(inValues, inPos, out, outPos);
            return (inPos + 16) | 0;
        case 3:
            fastUnpack256_3(inValues, inPos, out, outPos);
            return (inPos + 24) | 0;
        case 4:
            fastUnpack256_4(inValues, inPos, out, outPos);
            return (inPos + 32) | 0;
        case 5:
            fastUnpack256_5(inValues, inPos, out, outPos);
            return (inPos + 40) | 0;
        case 6:
            fastUnpack256_6(inValues, inPos, out, outPos);
            return (inPos + 48) | 0;
        case 7:
            fastUnpack256_7(inValues, inPos, out, outPos);
            return (inPos + 56) | 0;
        case 8:
            fastUnpack256_8(inValues, inPos, out, outPos);
            return (inPos + 64) | 0;
        case 9:
            fastUnpack256_Generic(inValues, inPos, out, outPos, 9);
            return (inPos + 72) | 0;
        case 10:
            fastUnpack256_Generic(inValues, inPos, out, outPos, 10);
            return (inPos + 80) | 0;
        case 11:
            fastUnpack256_Generic(inValues, inPos, out, outPos, 11);
            return (inPos + 88) | 0;
        case 12:
            fastUnpack256_Generic(inValues, inPos, out, outPos, 12);
            return (inPos + 96) | 0;
        case 13:
            fastUnpack256_Generic(inValues, inPos, out, outPos, 13);
            return (inPos + (13 * 8)) | 0;
        case 14:
            fastUnpack256_Generic(inValues, inPos, out, outPos, 14);
            return (inPos + (14 * 8)) | 0;
        case 15:
            fastUnpack256_Generic(inValues, inPos, out, outPos, 15);
            return (inPos + (15 * 8)) | 0;
        case 16:
            fastUnpack256_16(inValues, inPos, out, outPos);
            return (inPos + 128) | 0;
        default:
            fastUnpack256_Generic(inValues, inPos, out, outPos, bitWidth);
            return (inPos + (bitWidth * 8)) | 0;
    }
}

function applyBlockExceptions(
    out: Int32Array,
    blockOutPos: number,
    bitWidth: number,
    exceptionCount: number,
    byteContainer: Uint8Array,
    byteContainerLen: number,
    bytePosIn: number,
    ws: FastPforDecoderWorkspace,
    block: number,
): number {
    if (bytePosIn + 1 > byteContainerLen) {
        throw new Error(
            `FastPFOR decode: exception header underflow at block=${block} (need 1 byte for maxBits, bytePos=${bytePosIn}, byteSize=${byteContainerLen})`,
        );
    }
    const maxBits = byteContainer[bytePosIn++];

    if (maxBits < bitWidth || maxBits > 32) {
        throw new Error(
            `FastPFOR decode: invalid maxBits=${maxBits} at block=${block} (bitWidth=${bitWidth}, expected ${bitWidth}..32)`,
        );
    }
    const exceptionBitWidth = (maxBits - bitWidth) | 0;
    if (exceptionBitWidth < 1 || exceptionBitWidth > 32) {
        throw new Error(
            `FastPFOR decode: invalid exceptionBitWidth=${exceptionBitWidth} at block=${block} (bitWidth=${bitWidth}, maxBits=${maxBits})`,
        );
    }

    if (bytePosIn + exceptionCount > byteContainerLen) {
        throw new Error(
            `FastPFOR decode: exception positions underflow at block=${block} (need=${exceptionCount}, have=${byteContainerLen - bytePosIn})`,
        );
    }

    if (exceptionBitWidth === 1) {
        const shift = 1 << bitWidth;
        for (let k = 0; k < exceptionCount; k = (k + 1) | 0) {
            const pos = byteContainer[bytePosIn++];
            if (pos >= BLOCK_SIZE) {
                throw new Error(`FastPFOR decode: invalid exception pos=${pos} at block=${block} (expected 0..${BLOCK_SIZE - 1})`);
            }
            out[(pos + blockOutPos) | 0] |= shift;
        }
        return bytePosIn;
    }

    const exArr = ws.dataToBePacked[exceptionBitWidth];
    if (!exArr) {
        throw new Error(
            `FastPFOR decode: missing exception stream for exceptionBitWidth=${exceptionBitWidth} (bitWidth=${bitWidth}, maxBits=${maxBits}) at block ${block}`,
        );
    }

    const ptrs = ws.dataPointers;
    let exPtr = ptrs[exceptionBitWidth] | 0;
    const exSize = ws.exceptionSizes[exceptionBitWidth] | 0;

    if (exPtr + exceptionCount > exSize) {
        throw new Error(
            `FastPFOR decode: exception stream overflow for exceptionBitWidth=${exceptionBitWidth} (ptr=${exPtr}, need ${exceptionCount}, size=${exSize}) at block ${block}`,
        );
    }

    for (let k = 0; k < exceptionCount; k = (k + 1) | 0) {
        const pos = byteContainer[bytePosIn++];
        if (pos >= BLOCK_SIZE) {
            throw new Error(`FastPFOR decode: invalid exception pos=${pos} at block=${block} (expected 0..${BLOCK_SIZE - 1})`);
        }
        const val = exArr[exPtr++] | 0;
        out[(pos + blockOutPos) | 0] |= val << bitWidth;
    }
    ptrs[exceptionBitWidth] = exPtr;
    return bytePosIn;
}

function decodePageBlocks(
    inValues: Int32Array,
    pageStart: number,
    inPos: number,
    packedEnd: number,
    out: Int32Array,
    outPos: number,
    blocks: number,
    byteContainer: Uint8Array,
    byteContainerLen: number,
    ws: FastPforDecoderWorkspace,
): number {
    let tmpInPos = inPos | 0;
    let bytePosIn = 0;

    for (let run = 0; run < blocks; run = (run + 1) | 0) {
        if (bytePosIn + 2 > byteContainerLen) {
            throw new Error(
                `FastPFOR decode: byteContainer underflow at block=${run} (need 2 bytes for [bitWidth, exceptionCount], bytePos=${bytePosIn}, byteSize=${byteContainerLen})`,
            );
        }

        const bitWidth = byteContainer[bytePosIn++];
        const exceptionCount = byteContainer[bytePosIn++];

        if (bitWidth > 32) {
            throw new Error(
                `FastPFOR decode: invalid bitWidth=${bitWidth} at block=${run} (expected 0..32). This likely indicates corrupted or truncated input.`,
            );
        }

        if (exceptionCount > BLOCK_SIZE) {
            throw new Error(
                `FastPFOR decode: invalid exceptionCount=${exceptionCount} at block=${run} (expected 0..${BLOCK_SIZE})`,
            );
        }

        const blockOutPos = (outPos + (run * BLOCK_SIZE)) | 0;

        switch (bitWidth) {
            case 0:
                out.fill(0, blockOutPos, blockOutPos + BLOCK_SIZE);
                break;

            case 32:
                for (let i = 0; i < BLOCK_SIZE; i = (i + 1) | 0) {
                    out[(blockOutPos + i) | 0] = inValues[(tmpInPos + i) | 0] | 0;
                }
                tmpInPos = (tmpInPos + BLOCK_SIZE) | 0;
                break;

            default:
                tmpInPos = unpackBlock256(inValues, tmpInPos, out, blockOutPos, bitWidth);
                break;
        }

        if (exceptionCount > 0) {
            bytePosIn = applyBlockExceptions(
                out,
                blockOutPos,
                bitWidth,
                exceptionCount,
                byteContainer,
                byteContainerLen,
                bytePosIn,
                ws,
                run,
            );
        }
    }

    if (tmpInPos !== packedEnd) {
        throw new Error(
            `FastPFOR decode: packed region mismatch (pageStart=${pageStart}, tmpInPos=${tmpInPos}, expectedPackedEnd=${packedEnd}, encoded.length=${inValues.length})`,
        );
    }

    return tmpInPos;
}

/**
 * Decodes one FastPFOR page (aligned to 256-value blocks).
 */
function decodePage(
    inValues: Int32Array,
    out: Int32Array,
    inPos: number,
    outPos: number,
    thisSize: number,
    ws: FastPforDecoderWorkspace,
): number {
    const pageStart = inPos | 0;
    const whereMeta = inValues[pageStart] | 0;

    if (whereMeta <= 0 || pageStart + whereMeta > inValues.length - 1) {
        throw new Error(
            `FastPFOR decode: invalid whereMeta=${whereMeta} at pageStart=${pageStart} (expected > 0 and pageStart+whereMeta < encoded.length=${inValues.length})`,
        );
    }

    const packedStart = (pageStart + 1) | 0;
    const packedEnd = (pageStart + whereMeta) | 0;

    if (packedEnd >= inValues.length) {
        throw new Error(
            `FastPFOR decode: metadata offset out of bounds (pageStart=${pageStart}, whereMeta=${whereMeta}, metaStart=${packedEnd}, encoded.length=${inValues.length})`,
        );
    }

    const byteSize = inValues[packedEnd] >>> 0;
    const metaInts = (byteSize + 3) >>> 2;
    const byteContainerStart = packedEnd + 1;
    const bitmapPos = byteContainerStart + metaInts;

    if (bitmapPos >= inValues.length) {
        throw new Error(
            `FastPFOR decode: invalid byteSize=${byteSize} (metaInts=${metaInts}) causes bitmapPos=${bitmapPos} out of bounds (encoded.length=${inValues.length})`,
        );
    }

    const byteContainer = materializeByteContainer(inValues, byteContainerStart, byteSize, ws);
    const byteContainerLen = byteSize;

    const inExcept = unpackExceptionStreams(inValues, bitmapPos, ws);

    const ptrs = ws.dataPointers;
    ptrs.fill(0);
    const startOutPos = outPos | 0;
    const blocks = (thisSize / BLOCK_SIZE) | 0;
    decodePageBlocks(inValues, pageStart, packedStart, packedEnd, out, startOutPos, blocks, byteContainer, byteContainerLen, ws);

    return inExcept;
}

function decodeAlignedPages(
    inValues: Int32Array,
    out: Int32Array,
    inPos: number,
    outPos: number,
    outLength: number,
    ws: FastPforDecoderWorkspace,
): number {
    const alignedOutLength = greatestMultiple(outLength, BLOCK_SIZE);
    const finalOut = outPos + alignedOutLength;
    let tmpOutPos = outPos;
    let tmpInPos = inPos;

    while (tmpOutPos !== finalOut) {
        const thisSize = Math.min(PAGE_SIZE, finalOut - tmpOutPos);
        tmpInPos = decodePage(inValues, out, tmpInPos, tmpOutPos, thisSize, ws);
        tmpOutPos = (tmpOutPos + thisSize) | 0;
    }

    return tmpInPos;
}

/**
 * Decodes the VariableByte tail (MSB=1 terminator, opposite of Protobuf Varint).
 */
function decodeVByte(
    inValues: Int32Array,
    inPos: number,
    inLength: number,
    out: Int32Array,
    outPos: number,
    expectedCount: number,
): number {
    if (expectedCount === 0) return inPos;

    let s = 0;
    let p = inPos;
    const finalP = inPos + inLength;
    const outPos0 = outPos;
    let tmpOutPos = outPos;
    const targetOut = outPos + expectedCount;

    let v = 0;
    let shift = 0;

    while (p < finalP && tmpOutPos < targetOut) {
        const val = inValues[p];
        const c = (val >>> s) & 0xff;
        s += 8;
        p += s >>> 5;
        s &= 31;

        v |= (c & 0x7f) << shift;
        if ((c & 0x80) !== 0) {
            out[tmpOutPos++] = v | 0;
            v = 0;
            shift = 0;
        } else {
            shift += 7;
            if (shift > 28) {
                throw new Error(
                    `FastPFOR VByte: unterminated value (expected MSB=1 terminator within 5 bytes; shift=${shift}, partial=${v}, decoded=${tmpOutPos - outPos0}/${expectedCount}, inPos=${p}, inEnd=${finalP})`,
                );
            }
        }
    }

    if (tmpOutPos !== targetOut) {
        throw new Error(`FastPFOR VByte: truncated stream (decoded ${tmpOutPos - outPos0}, expected ${expectedCount})`);
    }

    return p;
}

/**
 * Decodes a sequence of FastPFOR-encoded integers.
 *
 * @param encoded The input buffer containing FastPFOR encoded data.
 * @param numValues The number of integers expected to be decoded.
 * @param ws Optional workspace for reuse across calls. If omitted, a new workspace is created per call.
 */
export function decodeFastPforInt32(encoded: Int32Buf, numValues: number, ws?: FastPforDecoderWorkspace): Int32Array {
    let inPos = 0;
    let outPos = 0;
    const decoded = new Int32Array(numValues);

    const workspace = ws ?? createDecoderWorkspace();

    if (encoded.length > 0) {
        const alignedLength = encoded[inPos] | 0;
        inPos = (inPos + 1) | 0;

        if (alignedLength < 0 || (alignedLength & (BLOCK_SIZE - 1)) !== 0) {
            throw new Error(`FastPFOR decode: invalid alignedLength=${alignedLength} (expected >= 0 and multiple of ${BLOCK_SIZE})`);
        }

        if (outPos + alignedLength > decoded.length) {
            throw new Error(
                `FastPFOR decode: output buffer too small (outPos=${outPos}, alignedLength=${alignedLength}, out.length=${decoded.length})`,
            );
        }

        inPos = decodeAlignedPages(encoded, decoded, inPos, outPos, alignedLength, workspace);
        outPos = (outPos + alignedLength) | 0;
    }

    const remainingLength = (encoded.length - inPos) | 0;
    const expectedTail = (numValues - outPos) | 0;
    inPos = decodeVByte(encoded, inPos, remainingLength, decoded, outPos, expectedTail);
    outPos = (outPos + expectedTail) | 0;

    if (outPos !== numValues) {
        throw new Error(`FastPFOR decode: decoded ${outPos} values, expected ${numValues}`);
    }

    return decoded;
}

function fastUnpack32(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number, bitWidth: number): void {
    switch (bitWidth) {
        case 2: fastUnpack32_2(inValues, inPos, out, outPos); return;
        case 3: fastUnpack32_3(inValues, inPos, out, outPos); return;
        case 4: fastUnpack32_4(inValues, inPos, out, outPos); return;
        case 5: fastUnpack32_5(inValues, inPos, out, outPos); return;
        case 6: fastUnpack32_6(inValues, inPos, out, outPos); return;
        case 7: fastUnpack32_7(inValues, inPos, out, outPos); return;
        case 8: fastUnpack32_8(inValues, inPos, out, outPos); return;
        case 9: fastUnpack32_9(inValues, inPos, out, outPos); return;
        case 10: fastUnpack32_10(inValues, inPos, out, outPos); return;
        case 11: fastUnpack32_11(inValues, inPos, out, outPos); return;
        case 12: fastUnpack32_12(inValues, inPos, out, outPos); return;
        case 16: fastUnpack32_16(inValues, inPos, out, outPos); return;
        case 32:
            for (let i = 0; i < 32; i = (i + 1) | 0) {
                out[(outPos + i) | 0] = inValues[(inPos + i) | 0] | 0;
            }
            return;
        default:
            break;
    }

    const mask = MASKS[bitWidth] >>> 0;
    let inputWordIndex = inPos;
    let bitOffset = 0;
    let currentWord = inValues[inputWordIndex] >>> 0;

    for (let i = 0; i < 32; i++) {
        if (bitOffset + bitWidth <= 32) {
            const value = (currentWord >>> bitOffset) & mask;
            out[outPos + i] = value | 0;
            bitOffset += bitWidth;

            if (bitOffset === 32) {
                bitOffset = 0;
                inputWordIndex++;
                if (i !== 31) currentWord = inValues[inputWordIndex] >>> 0;
            }
        } else {
            const lowBits = 32 - bitOffset;
            const low = currentWord >>> bitOffset;

            inputWordIndex++;
            currentWord = inValues[inputWordIndex] >>> 0;
            const highMask = MASKS[bitWidth - lowBits] >>> 0;
            const high = currentWord & highMask;

            const value = (low | (high << lowBits)) & mask;
            out[outPos + i] = value | 0;
            bitOffset = bitWidth - lowBits;
        }
    }
}
