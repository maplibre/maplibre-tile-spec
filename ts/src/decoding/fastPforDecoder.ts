import IntWrapper from "./intWrapper";
import type { Int32Buf, Uint8Buf } from "./fastPforSpec";
import {
    MASKS,
    DEFAULT_PAGE_SIZE,
    BLOCK_SIZE,
    greatestMultiple,
    roundUpToMultipleOf32,
    normalizePageSize,
} from "./fastPforSpec";
import {
    fastUnpack32_1,
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
    exceptionSizes: Int32Array;
};

const pSize = normalizePageSize(DEFAULT_PAGE_SIZE);
const byteContainerSize = ((3 * pSize) / BLOCK_SIZE + pSize) | 0;

const IS_LE = new Uint8Array(new Uint32Array([0x11223344]).buffer)[0] === 0x44;

/**
 * Creates an isolated workspace for decoding.
 * Reusing a workspace across calls avoids repeated allocations.
 */
export function createDecoderWorkspace(): FastPforDecoderWorkspace {
    return {
        dataToBePacked: new Array(33),
        dataPointers: new Int32Array(33),
        byteContainer: new Uint8Array(byteContainerSize) as Uint8Buf,
        exceptionSizes: new Int32Array(33),
    };
}

/**
 * Shared workspace used when the caller doesn't provide one.
 * Not safe for overlapping decode calls.
 */
let sharedDefaultWorkspace: FastPforDecoderWorkspace | undefined;

/**
 * Decodes one FastPFOR page (aligned to 256-value blocks).
 */
function decodePage(
    inValues: Int32Array,
    inPos: IntWrapper,
    out: Int32Array,
    outPos: IntWrapper,
    thisSize: number,
    ws: FastPforDecoderWorkspace,
): void {
    let pIn = inPos.get() | 0;
    const initPos = pIn;
    const whereMeta = inValues[pIn++] | 0;

    if (whereMeta <= 0 || initPos + whereMeta > inValues.length - 1) {
        throw new Error(
            `FastPFOR decode: invalid whereMeta=${whereMeta} at pageStart=${initPos} (expected > 0 and pageStart+whereMeta < encoded.length=${inValues.length})`,
        );
    }

    let inExcept = initPos + whereMeta;

    if (inExcept >= inValues.length) {
        throw new Error(
            `FastPFOR decode: metadata offset out of bounds (pageStart=${initPos}, whereMeta=${whereMeta}, inExcept=${inExcept}, encoded.length=${inValues.length})`,
        );
    }
    const byteSize = inValues[inExcept++] >>> 0;
    const metaInts = (byteSize + 3) >>> 2;

    const bitmapPos = inExcept + metaInts;
    if (bitmapPos >= inValues.length) {
        throw new Error(
            `FastPFOR decode: invalid byteSize=${byteSize} (metaInts=${metaInts}) causes bitmapPos=${bitmapPos} out of bounds (encoded.length=${inValues.length})`,
        );
    }

    if (ws.byteContainer.length < byteSize) {
        ws.byteContainer = new Uint8Array(byteSize * 2) as Uint8Buf;
    }
    const byteContainer = ws.byteContainer;
    const byteEnd = byteSize;

    const numFullInts = byteSize >>> 2;

    if (IS_LE && (byteContainer.byteOffset & 3) === 0) {
        const intView = new Int32Array(byteContainer.buffer, byteContainer.byteOffset, numFullInts);
        intView.set(inValues.subarray(inExcept, inExcept + numFullInts));
    } else {
        for (let i = 0; i < numFullInts; i = (i + 1) | 0) {
            const val = inValues[(inExcept + i) | 0] | 0;
            const base = i << 2;
            byteContainer[base] = val & 0xff;
            byteContainer[(base + 1) | 0] = (val >>> 8) & 0xff;
            byteContainer[(base + 2) | 0] = (val >>> 16) & 0xff;
            byteContainer[(base + 3) | 0] = (val >>> 24) & 0xff;
        }
    }

    const remainder = byteSize & 3;
    if (remainder > 0) {
        const lastIntIdx = (inExcept + numFullInts) | 0;
        const lastVal = inValues[lastIntIdx] | 0;
        const base = numFullInts << 2;
        for (let r = 0; r < remainder; r = (r + 1) | 0) {
            byteContainer[(base + r) | 0] = (lastVal >>> (r << 3)) & 0xff;
        }
    }

    inExcept = (inExcept + metaInts) | 0;

    const bitmap = inValues[inExcept++] | 0;

    const dtp = ws.dataToBePacked;
    const ptrs = ws.dataPointers;

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

    ptrs.fill(0);
    let tmpOutPos = outPos.get() | 0;
    let tmpInPos = pIn;

    let bytePosIn = 0;
    const blocks = (thisSize / BLOCK_SIZE) | 0;

    for (let run = 0; run < blocks; run++, tmpOutPos += BLOCK_SIZE) {
        if (bytePosIn + 2 > byteEnd) {
            throw new Error(
                `FastPFOR decode: byteContainer underflow at block=${run} (need 2 bytes for [b,cExcept], bytePos=${bytePosIn}, byteSize=${byteEnd})`,
            );
        }
        const b = byteContainer[bytePosIn++];
        const cExcept = byteContainer[bytePosIn++];

        if (b > 32) {
            throw new Error(`FastPFOR decode: invalid bitWidth=${b} at block=${run} (expected 0..32)`);
        }

        if (cExcept > BLOCK_SIZE) {
            throw new Error(`FastPFOR decode: invalid cExcept=${cExcept} at block=${run} (expected 0..${BLOCK_SIZE})`);
        }

        switch (b) {
            case 0:
                out.fill(0, tmpOutPos, tmpOutPos + BLOCK_SIZE);
                break;

            case 32:
                out.set(inValues.subarray(tmpInPos, tmpInPos + BLOCK_SIZE), tmpOutPos);
                tmpInPos += BLOCK_SIZE;
                break;

            case 1:
                fastUnpack256_1(inValues, tmpInPos, out, tmpOutPos);
                tmpInPos += 8;
                break;

            case 2:
                fastUnpack256_2(inValues, tmpInPos, out, tmpOutPos);
                tmpInPos += 16;
                break;

            case 3:
                fastUnpack256_3(inValues, tmpInPos, out, tmpOutPos);
                tmpInPos += 24;
                break;

            case 4:
                fastUnpack256_4(inValues, tmpInPos, out, tmpOutPos);
                tmpInPos += 32;
                break;

            case 5:
                fastUnpack256_5(inValues, tmpInPos, out, tmpOutPos);
                tmpInPos += 40;
                break;

            case 6:
                fastUnpack256_6(inValues, tmpInPos, out, tmpOutPos);
                tmpInPos += 48;
                break;

            case 7:
                fastUnpack256_7(inValues, tmpInPos, out, tmpOutPos);
                tmpInPos += 56;
                break;

            case 8:
                fastUnpack256_8(inValues, tmpInPos, out, tmpOutPos);
                tmpInPos += 64;
                break;

            case 9:
                fastUnpack256_Generic(inValues, tmpInPos, out, tmpOutPos, 9);
                tmpInPos += 72;
                break;

            case 10:
                fastUnpack256_Generic(inValues, tmpInPos, out, tmpOutPos, 10);
                tmpInPos += 80;
                break;

            case 11:
                fastUnpack256_Generic(inValues, tmpInPos, out, tmpOutPos, 11);
                tmpInPos += 88;
                break;

            case 12:
                fastUnpack256_Generic(inValues, tmpInPos, out, tmpOutPos, 12);
                tmpInPos += 96;
                break;

            case 13:
                fastUnpack256_Generic(inValues, tmpInPos, out, tmpOutPos, 13);
                tmpInPos += 13 * 8;
                break;
            case 14:
                fastUnpack256_Generic(inValues, tmpInPos, out, tmpOutPos, 14);
                tmpInPos += 14 * 8;
                break;
            case 15:
                fastUnpack256_Generic(inValues, tmpInPos, out, tmpOutPos, 15);
                tmpInPos += 15 * 8;
                break;

            case 16:
                fastUnpack256_16(inValues, tmpInPos, out, tmpOutPos);
                tmpInPos += 128;
                break;

            default:
                fastUnpack256_Generic(inValues, tmpInPos, out, tmpOutPos, b);
                tmpInPos += b * 8;
                break;
        }

        if (cExcept > 0) {
            if (bytePosIn + 1 > byteEnd) {
                throw new Error(
                    `FastPFOR decode: exception header underflow at block=${run} (need 1 byte for maxBits, bytePos=${bytePosIn}, byteSize=${byteEnd})`,
                );
            }
            const maxBits = byteContainer[bytePosIn++];

            if (maxBits < b || maxBits > 32) {
                throw new Error(`FastPFOR decode: invalid maxBits=${maxBits} at block=${run} (b=${b}, expected ${b}..32)`);
            }
            const index = maxBits - b;

            if (bytePosIn + cExcept > byteEnd) {
                throw new Error(
                    `FastPFOR decode: exception positions underflow at block=${run} (need=${cExcept}, have=${byteEnd - bytePosIn})`,
                );
            }

            if (index === 1) {
                const shift = 1 << b;
                for (let k = 0; k < cExcept; k = (k + 1) | 0) {
                    const pos = byteContainer[bytePosIn++];
                    if (pos >= BLOCK_SIZE) {
                        throw new Error(`FastPFOR decode: invalid exception pos=${pos} at block=${run} (expected 0..${BLOCK_SIZE - 1})`);
                    }
                    out[(pos + tmpOutPos) | 0] |= shift;
                }
            } else {
                const exArr = dtp[index];
                if (!exArr) {
                    throw new Error(
                        `FastPFOR decode: missing exception stream for index=${index} (b=${b}, maxBits=${maxBits}) at block ${run}`,
                    );
                }

                let exPtr = ptrs[index] | 0;
                const exSize = ws.exceptionSizes[index] | 0;

                if (exPtr + cExcept > exSize) {
                    throw new Error(
                        `FastPFOR decode: exception stream overflow for index=${index} (ptr=${exPtr}, need ${cExcept}, size=${exSize}) at block ${run}`,
                    );
                }

                for (let k = 0; k < cExcept; k = (k + 1) | 0) {
                    const pos = byteContainer[bytePosIn++];
                    if (pos >= BLOCK_SIZE) {
                        throw new Error(`FastPFOR decode: invalid exception pos=${pos} at block=${run} (expected 0..${BLOCK_SIZE - 1})`);
                    }
                    const val = exArr[exPtr++] | 0;
                    out[(pos + tmpOutPos) | 0] |= val << b;
                }
                ptrs[index] = exPtr;
            }
        }
    }

    const packedEnd = initPos + whereMeta;
    if (tmpInPos !== packedEnd) {
        throw new Error(
            `FastPFOR decode: packed region mismatch (pageStart=${initPos}, whereMeta=${whereMeta}, tmpInPos=${tmpInPos}, expectedPackedEnd=${packedEnd})`,
        );
    }

    outPos.set(tmpOutPos);
    inPos.set(inExcept);
}

function decodeAlignedPages(
    inValues: Int32Array,
    inPos: IntWrapper,
    out: Int32Array,
    outPos: IntWrapper,
    outLength: number,
    ws: FastPforDecoderWorkspace,
): void {
    const alignedOutLength = greatestMultiple(outLength, BLOCK_SIZE);
    const finalOut = outPos.get() + alignedOutLength;
    while (outPos.get() !== finalOut) {
        const thisSize = Math.min(pSize, finalOut - outPos.get());
        decodePage(inValues, inPos, out, outPos, thisSize, ws);
    }
}

function decodeFastPforPages(
    inValues: Int32Array,
    inPos: IntWrapper,
    inLength: number,
    out: Int32Array,
    outPos: IntWrapper,
    ws: FastPforDecoderWorkspace
): void {
    if (inLength === 0) return;
    if (inLength < 1) {
        throw new Error(`FastPFOR decode: truncated input (need at least 1 int32 word for alignedLength header, inLength=${inLength})`);
    }

    const alignedLength = inValues[inPos.get()];
    inPos.increment();

    if (alignedLength < 0 || (alignedLength & (BLOCK_SIZE - 1)) !== 0) {
        throw new Error(`FastPFOR decode: invalid alignedLength=${alignedLength} (expected >= 0 and multiple of ${BLOCK_SIZE})`);
    }

    const currentOut = outPos.get();
    if (currentOut + alignedLength > out.length) {
        throw new Error(
            `FastPFOR decode: output buffer too small (currentOut=${currentOut}, alignedLength=${alignedLength}, out.length=${out.length})`,
        );
    }

    decodeAlignedPages(inValues, inPos, out, outPos, alignedLength, ws);
}

/**
 * Decodes the VariableByte tail (MSB=1 terminator, opposite of Protobuf Varint).
 */
function decodeVByte(
    inValues: Int32Array,
    inPos: IntWrapper,
    inLength: number,
    out: Int32Array,
    outPos: IntWrapper,
    expectedCount: number,
): void {
    if (expectedCount === 0) return;

    let s = 0;
    let p = inPos.get();
    const finalP = inPos.get() + inLength;
    const outPos0 = outPos.get();
    let tmpOutPos = outPos0;
    const targetOut = outPos0 + expectedCount;

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

    outPos.set(tmpOutPos);
    inPos.set(p);
}

/**
 * Decodes a sequence of FastPFOR-encoded integers.
 *
 * @param encoded The input buffer containing FastPFOR encoded data.
 * @param numValues The number of integers expected to be decoded.
 * @param ws Optional workspace for reuse across calls. If omitted, a shared workspace is used.
 */
export function decodeFastPforInt32(encoded: Int32Buf, numValues: number, ws?: FastPforDecoderWorkspace): Int32Array {
    const inPos = new IntWrapper(0);
    const outPos = new IntWrapper(0);
    const decoded = new Int32Array(numValues);

    const init = inPos.get();
    const workspace = ws ?? (sharedDefaultWorkspace ??= createDecoderWorkspace());
    decodeFastPforPages(encoded, inPos, encoded.length, decoded, outPos, workspace);

    const remainingLength = encoded.length - (inPos.get() - init);
    const expectedTail = numValues - outPos.get();
    decodeVByte(encoded, inPos, remainingLength, decoded, outPos, expectedTail);

    if (outPos.get() !== numValues) {
        throw new Error(`FastPFOR decode: decoded ${outPos.get()} values, expected ${numValues}`);
    }

    return decoded;
}

function fastUnpack32(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number, bitWidth: number): void {
    switch (bitWidth) {
        case 1: fastUnpack32_1(inValues, inPos, out, outPos); return;
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
            out.set(inValues.subarray(inPos, inPos + 32), outPos);
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
