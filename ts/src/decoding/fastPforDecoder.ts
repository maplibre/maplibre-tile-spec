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
 *
 * Contains scratch buffers that are lazily allocated on first use.
 * Reusing a workspace across multiple decode calls avoids repeated allocations.
 */
export type FastPforDecoderWorkspace = {
    dataToBePacked: Array<Int32Array | undefined>;
    dataPointers: Int32Array;
    byteContainer: Uint8Buf;
    exceptionSizes: Int32Array; // Stores declared size for each exception stream [2..32]
};

// Module-level constants
const pSize = normalizePageSize(DEFAULT_PAGE_SIZE);
const byteContainerSize = ((3 * pSize) / BLOCK_SIZE + pSize) | 0;

// Endianness detection: true if host is little-endian (x86/ARM)
const IS_LE = new Uint8Array(new Uint32Array([0x11223344]).buffer)[0] === 0x44;

/**
 * Creates a new independent workspace for the decoder.
 *
 * The workspace uses lazy allocation: buffers are only allocated when needed.
 * This makes it efficient to create a fresh workspace per call (safe default)
 * while still allowing reuse for maximum performance.
 *
 * Note on Isolation:
 * The decoder keeps scratch buffers in a workspace to avoid repeated allocations.
 * decodeFastPforInt32() uses a shared module-level workspace by default (fast-path).
 * If you need strict isolation (e.g. you wrap decoding in a reusable decoder object,
 * or you want to be explicit about scratch ownership), create a workspace and pass it.
 *
 * Note: In browsers, each WebWorker has its own JS realm/module instance, so the shared default
 * workspace is not shared across workers. Passing an explicit workspace still makes ownership explicit.
 */
export function createDecoderWorkspace(): FastPforDecoderWorkspace {
    return {
        dataToBePacked: new Array(33), // lazy: allocate on first use per bitwidth
        dataPointers: new Int32Array(33),
        byteContainer: new Uint8Array(byteContainerSize) as Uint8Buf,
        exceptionSizes: new Int32Array(33), // Tracks declared size for corruption detection
    };
}

/**
 * Shared default workspace for the hot-path when caller doesn't provide one.
 * Avoids allocating ~O(pageSize) buffers per stream decode.
 *
 * IMPORTANT: Not safe for concurrent/overlapping decode calls. JS execution is synchronous by default,
 * but if you wrap decoding in a reusable decoder object or need explicit scratch ownership,
 * create a workspace via createDecoderWorkspace() and pass it to each call.
 *
 * Note: In browsers, each WebWorker has its own JS realm/module instance, so this shared workspace
 * is not shared across workers.
 */
let sharedDefaultWorkspace: FastPforDecoderWorkspace | undefined;

/**
 * FastPFOR page layout (Int32 words):
 *
 *  [0] whereMeta (offset in int32 words from page start to metadata area)
 *  [1..whereMeta-1] packed blocks region (bitpacked payload for 256-value blocks)
 *
 *  Metadata area at (pageStart + whereMeta):
 *   [0] byteSize (bytes)            -> number of bytes for byteContainer (block headers + exception positions)
 *   [1..metaInts] byteContainer     -> stored as int32 words, little-endian byte order
 *   [next] bitmap (int32)           -> which exception streams (bitwidth 2..32) are present
 *   For each k in 2..32 if bitmap bit set:
 *     [size] count of exception values in that stream
 *     [data] bitpacked exception values (rounded up to multiple of 32)
 *
 * Decoding steps:
 *  1) Parse header, locate metadata (whereMeta)
 *  2) Materialize byteContainer bytes from int32 words
 *  3) Expand exception streams into ws.dataToBePacked[k]
 *  4) Decode blocks (bitpacked) into output array
 *  5) Patch exceptions using byteContainer positions + dataToBePacked values
 *
 * Invariants verified:
 *  - whereMeta points within input buffer
 *  - byteSize + metaInts fit in input
 *  - bitWidth b in [0..32], cExcept in [0..BLOCK_SIZE]
 *  - maxBits in [b..32], exception pos in [0..BLOCK_SIZE)
 *  - packed region exactly consumed (tmpInPos === packedEnd)
 */
function decodePage(
    inValues: Int32Array,
    inPos: IntWrapper,
    out: Int32Array,
    outPos: IntWrapper,
    thisSize: number,
    ws: FastPforDecoderWorkspace,
): void {
    // Cache IntWrapper values as local primitives for JIT optimization
    let pIn = inPos.get() | 0;
    const initPos = pIn;
    const whereMeta = inValues[pIn++] | 0;

    // whereMeta is an offset from initPos to the exception metadata area.
    // Must be positive and point within the input buffer.
    if (whereMeta <= 0 || initPos + whereMeta > inValues.length - 1) {
        throw new Error(`FastPFOR: invalid whereMeta ${whereMeta} at position ${initPos}`);
    }

    let inExcept = initPos + whereMeta;

    // Validate inExcept bounds before reading byteSize
    if (inExcept >= inValues.length) {
        throw new Error(`FastPFOR decode: inExcept ${inExcept} out of bounds (length=${inValues.length})`);
    }
    const byteSize = inValues[inExcept++] >>> 0;
    const metaInts = (byteSize + 3) >>> 2;

    // Validate metaInts + bitmap bounds before reading byteContainer
    const bitmapPos = inExcept + metaInts;
    if (bitmapPos >= inValues.length) {
        throw new Error(
            `FastPFOR decode: metaInts overflow (inExcept=${inExcept}, metaInts=${metaInts}, length=${inValues.length})`,
        );
    }

    // Reuse pre-allocated buffer instead of allocating new Uint8Array per page
    // Note: if the buffer is too small, we reallocate it. This updates the reference
    // in the passed 'ws' object, so the caller's workspace is updated for future calls.
    if (ws.byteContainer.length < byteSize) {
        ws.byteContainer = new Uint8Array(byteSize * 2) as Uint8Buf;
    }
    const byteContainer = ws.byteContainer;
    const byteEnd = byteSize; // Keep as unsigned (no |0 to avoid signed overflow)

    // Bulk byte copy: process 4 bytes at a time (V8 optimization)
    // Using Int32Array view is significantly faster than byte-by-byte loop
    const numFullInts = byteSize >>> 2;

    // Fast path: aligned buffer AND little-endian host (x86/ARM)
    if (IS_LE && (byteContainer.byteOffset & 3) === 0) {
        // Direct Int32Array view copy (fastest)
        const intView = new Int32Array(byteContainer.buffer, byteContainer.byteOffset, numFullInts);
        intView.set(inValues.subarray(inExcept, inExcept + numFullInts));
    } else {
        // Fallback: unaligned buffer (safer but slower)
        for (let i = 0; i < numFullInts; i = (i + 1) | 0) {
            const val = inValues[(inExcept + i) | 0] | 0;
            const base = i << 2;
            byteContainer[base] = val & 0xff;
            byteContainer[(base + 1) | 0] = (val >>> 8) & 0xff;
            byteContainer[(base + 2) | 0] = (val >>> 16) & 0xff;
            byteContainer[(base + 3) | 0] = (val >>> 24) & 0xff;
        }
    }

    // Handle remainder bytes (byteSize % 4)
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

    // Cache workspace arrays for faster access in loops
    const dtp = ws.dataToBePacked;
    const ptrs = ws.dataPointers;

    for (let k = 2; k <= 32; k = (k + 1) | 0) {
        // Use >>> to avoid 1 << 31 becoming negative (more review-friendly)
        if (((bitmap >>> (k - 1)) & 1) !== 0) {
            const size = inValues[inExcept++] >>> 0; // Keep unsigned
            const roundedUp = roundUpToMultipleOf32(size);

            // Guard: Verify enough words to contain the bit-packed exceptions.
            // Uses ceil(size * k / 32) — the mathematical minimum — to avoid
            // false positives on tightly-packed buffers where encoder rewind-padding
            // isn't physically present.
            const wordsNeeded = ((size * k) + 31) >>> 5;
            if (inExcept + wordsNeeded > inValues.length) {
                throw new Error(`FastPFOR decode: truncated exception stream for bitwidth ${k} (need ${wordsNeeded} words)`);
            }

            // Lazy allocation: allocate buffer only when needed
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

            // Store the declared size for strict corruption detection during patching
            ws.exceptionSizes[k] = size;
        }
    }

    ptrs.fill(0);
    let tmpOutPos = outPos.get() | 0;
    let tmpInPos = pIn;

    let bytePosIn = 0;
    const blocks = (thisSize / BLOCK_SIZE) | 0;

    // Optimization: manual loop unrolling for 256 values
    // Saves function call overhead (8 calls -> 1 call per block)

    for (let run = 0; run < blocks; run++, tmpOutPos += BLOCK_SIZE) {
        // Validate byteContainer bounds before reading block header
        if (bytePosIn + 2 > byteEnd) {
            throw new Error(`FastPFOR decode: byteContainer underflow at block ${run}`);
        }
        const b = byteContainer[bytePosIn++];
        const cExcept = byteContainer[bytePosIn++];

        // Validate bitWidth (anti-corruption guard)
        if (b > 32) {
            throw new Error(`FastPFOR decode: invalid bitWidth ${b} at block ${run}`);
        }

        // Validate cExcept (anti-corruption guard)
        if (cExcept > BLOCK_SIZE) {
            throw new Error(`FastPFOR decode: invalid cExcept ${cExcept} at block ${run} (max=${BLOCK_SIZE})`);
        }

        // Hot path: each block (256 values).
        // Using fastUnpack256_X saves ~10-15% overhead vs calling fastUnpack32_X 8 times.
        switch (b) {
            case 0:
                out.fill(0, tmpOutPos, tmpOutPos + BLOCK_SIZE);
                break;

            case 32:
                // Direct memory copy is fastest for bitwidth 32
                out.set(inValues.subarray(tmpInPos, tmpInPos + BLOCK_SIZE), tmpOutPos);
                tmpInPos += BLOCK_SIZE;
                // Note: subarray creates a view, but V8 optimizing compilers handle this well.
                // Alternative: manual loop copy if subarray proves slow in microbenchmarks.
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
                // Generic fallback for bitwidths 17-31
                fastUnpack256_Generic(inValues, tmpInPos, out, tmpOutPos, b);
                tmpInPos += b * 8;
                break;
        }

        if (cExcept > 0) {
            // Validate byteContainer bounds for exception header
            if (bytePosIn + 1 > byteEnd) {
                throw new Error(`FastPFOR decode: exception header underflow at block ${run}`);
            }
            const maxBits = byteContainer[bytePosIn++];

            // Guard: maxBits must be within [b..32] (anti-corruption)
            if (maxBits < b || maxBits > 32) {
                throw new Error(`FastPFOR decode: invalid maxBits=${maxBits} (b=${b}) at block ${run}`);
            }
            const index = maxBits - b;



            // Validate byteContainer bounds for exception positions
            if (bytePosIn + cExcept > byteEnd) {
                throw new Error(`FastPFOR decode: exception payload underflow (need ${cExcept}, have ${byteEnd - bytePosIn})`);
            }

            // Optimized exception loops
            if (index === 1) {
                const shift = 1 << b; // Hoist shift calculation
                for (let k = 0; k < cExcept; k = (k + 1) | 0) {
                    const pos = byteContainer[bytePosIn++];
                    // Validate pos within block (anti-corruption guard)
                    if (pos >= BLOCK_SIZE) {
                        throw new Error(`FastPFOR decode: exception pos ${pos} >= BLOCK_SIZE at block ${run}`);
                    }
                    out[(pos + tmpOutPos) | 0] |= shift;
                }
            } else {
                const exArr = dtp[index];
                // Guard: exception stream must exist (anti-corruption)
                if (!exArr) {
                    throw new Error(
                        `FastPFOR decode: missing exception stream for index=${index} (b=${b}, maxBits=${maxBits}) at block ${run}`,
                    );
                }

                let exPtr = ptrs[index] | 0;
                const exSize = ws.exceptionSizes[index] | 0;

                // Guard: exception stream must have enough data (anti-corruption).
                // Verify against declared size (exact), not rounded-up buffer length.
                // This catches corruption where size field is malformed/truncated.
                if (exPtr + cExcept > exSize) {
                    throw new Error(
                        `FastPFOR decode: exception stream overflow for index=${index} (ptr=${exPtr}, need ${cExcept}, size=${exSize}) at block ${run}`,
                    );
                }

                for (let k = 0; k < cExcept; k = (k + 1) | 0) {
                    const pos = byteContainer[bytePosIn++];
                    // Validate pos within block (anti-corruption guard)
                    if (pos >= BLOCK_SIZE) {
                        throw new Error(`FastPFOR decode: exception pos ${pos} >= BLOCK_SIZE at block ${run}`);
                    }
                    const val = exArr[exPtr++] | 0;
                    out[(pos + tmpOutPos) | 0] |= val << b;
                }
                ptrs[index] = exPtr;
            }
        }
    }

    // Verify packed region was fully consumed (corruption detection)
    const packedEnd = initPos + whereMeta;
    if (tmpInPos !== packedEnd) {
        throw new Error(`FastPFOR: packed region mismatch (tmpInPos=${tmpInPos}, expected=${packedEnd})`);
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
        // Ensure thisSize aligns with BLOCK_SIZE unless it's the very end of stream logic handled elsewhere
        // But fastpfor pages are always multiples of BLOCK_SIZE (256).
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
    if (inLength < 1) throw new Error("FastPFOR: buffer too small");

    const outLength = inValues[inPos.get()];
    inPos.increment();

    if (outLength < 0 || (outLength & (BLOCK_SIZE - 1)) !== 0) {
        throw new Error(`FastPFOR: invalid outLength=${outLength}`);
    }

    // Performance note: check capacity once
    const currentOut = outPos.get();
    if (currentOut + outLength > out.length) {
        throw new Error(`FastPFOR: outLength=${outLength} exceeds output capacity`);
    }

    decodeAlignedPages(inValues, inPos, out, outPos, outLength, ws);
}



// Note: This implements VByte decoding (MSB=1 is terminator), which is the inverse of standard
// Protobuf Varint (MSB=0 is terminator). That is why we cannot reuse the generic decodeVarint methods.
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

    // Stop as soon as we decoded expectedCount values to avoid consuming any trailing/padding bytes.
    while (p < finalP && tmpOutPos < targetOut) {
        const val = inValues[p];
        const c = (val >>> s) & 0xff;
        s += 8;
        p += s >>> 5;
        s &= 31;

        v |= (c & 0x7f) << shift;
        if ((c & 0x80) !== 0) { // MSB=1 is the terminator in VByte
            out[tmpOutPos++] = v | 0;
            v = 0;
            shift = 0;
        } else {
            shift += 7;
            // Guard: 5 bytes max for 32-bit varint (shift max after 4 continuations = 28)
            if (shift > 28) {
                throw new Error("FastPFOR VByte: varint too long / corrupted stream");
            }
        }
    }

    // Truncation check: did we decode all expected values?
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
 * @param ws Optional workspace for reuse across calls. If omitted, a shared module-level workspace is used
 *           to avoid per-call allocations in hot loops. For explicit reuse (and for concurrency safety),
 *           create a workspace once with `createDecoderWorkspace()` and pass it to each call.
 */
export function decodeFastPforInt32(encoded: Int32Buf, numValues: number, ws?: FastPforDecoderWorkspace): Int32Array {
    const inPos = new IntWrapper(0);
    const outPos = new IntWrapper(0);
    const decoded = new Int32Array(numValues);

    // 1. FastPFOR decode
    const init = inPos.get();
    const workspace = ws ?? (sharedDefaultWorkspace ??= createDecoderWorkspace());
    decodeFastPforPages(encoded, inPos, encoded.length, decoded, outPos, workspace);

    // 2. VariableByte decode for remaining values
    const remainingLength = encoded.length - (inPos.get() - init);
    const expectedTail = numValues - outPos.get();
    decodeVByte(encoded, inPos, remainingLength, decoded, outPos, expectedTail);

    // Final invariant check: decoded count must match expected (should always pass after decodeVByte)
    if (outPos.get() !== numValues) {
        throw new Error(`FastPFOR: decoded ${outPos.get()} values, expected ${numValues}`);
    }

    return decoded;
}



/**
 * Generic bit-unpacking of 32 integers, matching JavaFastPFOR BitPacking.fastunpack ordering.
 * Reads exactly `bitWidth` int32 words from `inValues` starting at `inPos`.
 * Uses explicit switch for specialized widths 1-12 and 16, generic fallback for 13-15 and 17-31.
 */
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
            // Generic fallback for bitwidths 13-15 and 17-31
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
