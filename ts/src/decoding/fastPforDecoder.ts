import {
    MASKS,
    DEFAULT_PAGE_SIZE,
    BLOCK_SIZE,
    greatestMultiple,
    roundUpToMultipleOf32,
    normalizePageSize,
    type Int32Buf,
    type Uint8Buf,
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
 * FastPFOR decoding implementation.
 *
 * @remarks
 * Terminology note: "exceptions" in FastPFOR refer to **outlier values** within a block that do not fit in the
 * chosen base bit-width for that block. These are stored in separate "exception streams" and later applied back
 * to the unpacked base values. This is unrelated to JavaScript/TypeScript runtime exceptions.
 */

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

/**
 * Workspace for decoding the FastPFOR *wire format* (big-endian int32 words).
 *
 * @remarks
 * This workspace owns:
 * - a scratch `encodedWords` buffer to materialize big-endian words
 * - the reusable `FastPforDecoderWorkspace` used by `decodeFastPforInt32`
 *
 * The caller is responsible for creating and reusing this object.
 */
export type FastPforWireDecodeWorkspace = {
    encodedWords: Int32Array;
    decoderWorkspace: FastPforDecoderWorkspace;
};

const MAX_BIT_WIDTH = 32;
const BIT_WIDTH_SLOTS = MAX_BIT_WIDTH + 1;

const PAGE_SIZE = normalizePageSize(DEFAULT_PAGE_SIZE);
const BYTE_CONTAINER_SIZE = ((3 * PAGE_SIZE) / BLOCK_SIZE + PAGE_SIZE) | 0;

/**
 * Creates an isolated workspace for decoding.
 * Reusing a workspace across calls avoids repeated allocations.
 */
export function createDecoderWorkspace(): FastPforDecoderWorkspace {
    const byteContainer = new Uint8Array(BYTE_CONTAINER_SIZE) as Uint8Buf;
    return {
        dataToBePacked: new Array(BIT_WIDTH_SLOTS),
        dataPointers: new Int32Array(BIT_WIDTH_SLOTS),
        byteContainer,
        byteContainerI32: new Int32Array(
            byteContainer.buffer,
            byteContainer.byteOffset,
            byteContainer.byteLength >>> 2,
        ),
        exceptionSizes: new Int32Array(BIT_WIDTH_SLOTS),
    };
}

export function createFastPforWireDecodeWorkspace(initialEncodedWordCapacity: number = 16): FastPforWireDecodeWorkspace {
    if (initialEncodedWordCapacity < 0) {
        throw new RangeError(`initialEncodedWordCapacity must be >= 0, got ${initialEncodedWordCapacity}`);
    }

    const capacity = Math.max(16, initialEncodedWordCapacity | 0);
    return {
        encodedWords: new Int32Array(capacity),
        decoderWorkspace: createDecoderWorkspace(),
    };
}

export function ensureFastPforWireEncodedWordsCapacity(
    workspace: FastPforWireDecodeWorkspace,
    requiredWordCount: number,
): Int32Array {
    if (requiredWordCount <= workspace.encodedWords.length) return workspace.encodedWords;

    const next = new Int32Array(Math.max(16, requiredWordCount * 2));
    workspace.encodedWords = next;
    return next;
}

function materializeByteContainer(
    inValues: Int32Array,
    byteContainerStart: number,
    byteSize: number,
    workspace: FastPforDecoderWorkspace,
): Uint8Buf {
    if (workspace.byteContainer.length < byteSize) {
        workspace.byteContainer = new Uint8Array(byteSize * 2) as Uint8Buf;
        workspace.byteContainerI32 = undefined;
    }
    const byteContainer = workspace.byteContainer;
    const numFullInts = byteSize >>> 2;

    if ((byteContainer.byteOffset & 3) === 0) {
        let intView = workspace.byteContainerI32;
        if (
            !intView ||
            intView.buffer !== byteContainer.buffer ||
            intView.byteOffset !== byteContainer.byteOffset ||
            intView.length < numFullInts
        ) {
            intView = workspace.byteContainerI32 = new Int32Array(
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

/**
 * Unpacks the per-bitWidth "exception streams" described by the page's bitmap.
 *
 * @remarks
 * For each bit-width present in the bitmap, a stream header gives the count of outlier values for that
 * bit-width, followed by packed bits representing those values.
 *
 * @param inValues - Packed input (32-bit words).
 * @param inExcept - Offset (32-bit word index) where the exception bitmap starts.
 * @param workspace - Decoder workspace used to store the unpacked exception streams.
 * @returns The new input offset (32-bit word index) after consuming all exception streams.
 */
function unpackExceptionStreams(inValues: Int32Array, inExcept: number, workspace: FastPforDecoderWorkspace): number {
    const bitmap = inValues[inExcept++] | 0;
    const dataToBePacked = workspace.dataToBePacked;

    for (let bitWidth = 2; bitWidth <= MAX_BIT_WIDTH; bitWidth = (bitWidth + 1) | 0) {
        if (((bitmap >>> (bitWidth - 1)) & 1) === 0) continue;

        if (inExcept >= inValues.length) {
            throw new Error(
                `FastPFOR decode: truncated exception stream header (bitWidth=${bitWidth}, streamWordIndex=${inExcept}, needWords=1, availableWords=${inValues.length - inExcept}, encodedWords=${inValues.length})`,
            );
        }
        const size = inValues[inExcept++] >>> 0;
        const roundedUp = roundUpToMultipleOf32(size);

        const wordsNeeded = (size * bitWidth + 31) >>> 5;
        if (inExcept + wordsNeeded > inValues.length) {
            throw new Error(
                `FastPFOR decode: truncated exception stream (bitWidth=${bitWidth}, size=${size}, streamWordIndex=${inExcept}, needWords=${wordsNeeded}, availableWords=${inValues.length - inExcept}, encodedWords=${inValues.length})`,
            );
        }

        let exceptionStream = dataToBePacked[bitWidth];
        if (!exceptionStream || exceptionStream.length < roundedUp) {
            exceptionStream = dataToBePacked[bitWidth] = new Int32Array(roundedUp);
        }

        let j = 0;
        for (; j < size; j = (j + 32) | 0) {
            fastUnpack32(inValues, inExcept, exceptionStream, j, bitWidth);
            inExcept = (inExcept + bitWidth) | 0;
        }

        const overflow = (j - size) | 0;
        inExcept = (inExcept - ((overflow * bitWidth) >>> 5)) | 0;

        workspace.exceptionSizes[bitWidth] = size;
    }

    return inExcept;
}

/**
 * Unpacks one 256-value block from the packed bitstream using a specialized implementation for common widths.
 *
 * @param inValues - Packed input (32-bit words).
 * @param inPos - Input offset (32-bit word index) where the packed block starts.
 * @param out - Output buffer.
 * @param outPos - Output offset where the 256 values will be written.
 * @param bitWidth - Base bit-width used for this block.
 * @returns The new input offset (32-bit word index) right after the packed block data.
 */
function unpackBlock256(
    inValues: Int32Array,
    inPos: number,
    out: Int32Array,
    outPos: number,
    bitWidth: number,
): number {
    switch (bitWidth) {
        case 1:
            fastUnpack256_1(inValues, inPos, out, outPos);
            break;
        case 2:
            fastUnpack256_2(inValues, inPos, out, outPos);
            break;
        case 3:
            fastUnpack256_3(inValues, inPos, out, outPos);
            break;
        case 4:
            fastUnpack256_4(inValues, inPos, out, outPos);
            break;
        case 5:
            fastUnpack256_5(inValues, inPos, out, outPos);
            break;
        case 6:
            fastUnpack256_6(inValues, inPos, out, outPos);
            break;
        case 7:
            fastUnpack256_7(inValues, inPos, out, outPos);
            break;
        case 8:
            fastUnpack256_8(inValues, inPos, out, outPos);
            break;
        case 16:
            fastUnpack256_16(inValues, inPos, out, outPos);
            break;
        default:
            fastUnpack256_Generic(inValues, inPos, out, outPos, bitWidth);
            break;
    }

    return (inPos + (bitWidth << 3)) | 0;
}

/**
 * Reads and validates the 2-byte block header from the byteContainer.
 *
 * @remarks
 * The header is `[bitWidth, exceptionCount]`, both stored as single bytes.
 *
 * @param byteContainer - Byte metadata buffer for the page.
 * @param byteContainerLen - The valid byte length in `byteContainer` for this page.
 * @param bytePosIn - Current offset in `byteContainer`.
 * @param block - Block index within the page (for error messages).
 * @returns The parsed header and the updated `bytePosIn`.
 */
function readBlockHeader(
    byteContainer: Uint8Array,
    byteContainerLen: number,
    bytePosIn: number,
    block: number,
): { bitWidth: number; exceptionCount: number; bytePosIn: number } {
    if (bytePosIn + 2 > byteContainerLen) {
        throw new Error(
            `FastPFOR decode: byteContainer underflow at block=${block} (need 2 bytes for [bitWidth, exceptionCount], bytePos=${bytePosIn}, byteSize=${byteContainerLen})`,
        );
    }

    const bitWidth = byteContainer[bytePosIn++];
    const exceptionCount = byteContainer[bytePosIn++];

    if (bitWidth > MAX_BIT_WIDTH) {
        throw new Error(
            `FastPFOR decode: invalid bitWidth=${bitWidth} at block=${block} (expected 0..${MAX_BIT_WIDTH}). This likely indicates corrupted or truncated input.`,
        );
    }

    return { bitWidth, exceptionCount, bytePosIn };
}

/**
 * Reads and validates the exception header for a block.
 *
 * @remarks
 * The header contains `maxBits` (1 byte), which defines the width of the outlier values as
 * `exceptionBitWidth = maxBits - bitWidth`.
 *
 * @param byteContainer - Byte metadata buffer for the page.
 * @param byteContainerLen - The valid byte length in `byteContainer` for this page.
 * @param bytePosIn - Current offset in `byteContainer`.
 * @param bitWidth - Base bit-width for the block.
 * @param exceptionCount - Number of exceptions/outliers in this block.
 * @param block - Block index within the page (for error messages).
 * @returns Parsed `maxBits`, `exceptionBitWidth`, and the updated `bytePosIn`.
 */
function readBlockExceptionHeader(
    byteContainer: Uint8Array,
    byteContainerLen: number,
    bytePosIn: number,
    bitWidth: number,
    exceptionCount: number,
    block: number,
): { maxBits: number; exceptionBitWidth: number; bytePosIn: number } {
    if (bytePosIn + 1 > byteContainerLen) {
        throw new Error(
            `FastPFOR decode: exception header underflow at block=${block} (need 1 byte for maxBits, bytePos=${bytePosIn}, byteSize=${byteContainerLen})`,
        );
    }
    const maxBits = byteContainer[bytePosIn++];

    if (maxBits < bitWidth || maxBits > MAX_BIT_WIDTH) {
        throw new Error(
            `FastPFOR decode: invalid maxBits=${maxBits} at block=${block} (bitWidth=${bitWidth}, expected ${bitWidth}..${MAX_BIT_WIDTH})`,
        );
    }
    const exceptionBitWidth = (maxBits - bitWidth) | 0;
    if (exceptionBitWidth < 1 || exceptionBitWidth > MAX_BIT_WIDTH) {
        throw new Error(
            `FastPFOR decode: invalid exceptionBitWidth=${exceptionBitWidth} at block=${block} (bitWidth=${bitWidth}, maxBits=${maxBits})`,
        );
    }

    if (bytePosIn + exceptionCount > byteContainerLen) {
        throw new Error(
            `FastPFOR decode: exception positions underflow at block=${block} (need=${exceptionCount}, have=${byteContainerLen - bytePosIn})`,
        );
    }

    return { maxBits, exceptionBitWidth, bytePosIn };
}

/**
 * Applies (block-local) FastPFOR "exceptions" (outliers) to an already-unpacked base 256-value block.
 *
 * @param out - Output buffer containing the base unpacked values for the block.
 * @param blockOutPos - Offset in `out` where the 256-value block starts.
 * @param bitWidth - Base bit-width for the block.
 * @param exceptionCount - Number of exceptions/outliers in this block.
 * @param byteContainer - Byte metadata buffer for the page.
 * @param byteContainerLen - The valid byte length in `byteContainer` for this page.
 * @param bytePosIn - Current offset in `byteContainer` (right after `[bitWidth, exceptionCount]`).
 * @param workspace - Decoder workspace holding the unpacked exception streams.
 * @param block - Block index within the page (for error messages).
 * @returns The updated `bytePosIn` after consuming the exception metadata bytes.
 *
 * The exception metadata is stored in `byteContainer`:
 * - `maxBits` (1 byte): the maximum bit-width of any value in the block
 * - `exceptionCount` exception positions (1 byte each, 0..255)
 *
 * The exception values themselves are read from the pre-unpacked exception streams stored in `workspace`.
 * Returns the new position in the byteContainer after consuming the exception metadata bytes.
 */
function applyBlockExceptions(
    out: Int32Array,
    blockOutPos: number,
    bitWidth: number,
    exceptionCount: number,
    byteContainer: Uint8Array,
    byteContainerLen: number,
    bytePosIn: number,
    workspace: FastPforDecoderWorkspace,
    block: number,
): number {
    const {
        maxBits,
        exceptionBitWidth,
        bytePosIn: afterHeaderPos,
    } = readBlockExceptionHeader(byteContainer, byteContainerLen, bytePosIn, bitWidth, exceptionCount, block);
    bytePosIn = afterHeaderPos;

    if (exceptionBitWidth === 1) {
        const shift = 1 << bitWidth;
        for (let k = 0; k < exceptionCount; k = (k + 1) | 0) {
            const pos = byteContainer[bytePosIn++];
            out[(pos + blockOutPos) | 0] |= shift;
        }
        return bytePosIn;
    }

    const exceptionValues = workspace.dataToBePacked[exceptionBitWidth];
    if (!exceptionValues) {
        throw new Error(
            `FastPFOR decode: missing exception stream for exceptionBitWidth=${exceptionBitWidth} (bitWidth=${bitWidth}, maxBits=${maxBits}) at block ${block}`,
        );
    }

    const exceptionPointers = workspace.dataPointers;
    let exPtr = exceptionPointers[exceptionBitWidth] | 0;
    const exSize = workspace.exceptionSizes[exceptionBitWidth] | 0;

    if (exPtr + exceptionCount > exSize) {
        throw new Error(
            `FastPFOR decode: exception stream overflow for exceptionBitWidth=${exceptionBitWidth} (ptr=${exPtr}, need ${exceptionCount}, size=${exSize}) at block ${block}`,
        );
    }

    for (let k = 0; k < exceptionCount; k = (k + 1) | 0) {
        const pos = byteContainer[bytePosIn++];
        const val = exceptionValues[exPtr++] | 0;
        out[(pos + blockOutPos) | 0] |= val << bitWidth;
    }
    exceptionPointers[exceptionBitWidth] = exPtr;
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
    workspace: FastPforDecoderWorkspace,
): void {
    let tmpInPos = inPos | 0;
    let bytePosIn = 0;

    for (let run = 0; run < blocks; run = (run + 1) | 0) {
        const header = readBlockHeader(byteContainer, byteContainerLen, bytePosIn, run);
        bytePosIn = header.bytePosIn;
        const bitWidth = header.bitWidth;
        const exceptionCount = header.exceptionCount;

        const blockOutPos = (outPos + run * BLOCK_SIZE) | 0;

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
                workspace,
                run,
            );
        }
    }

    if (tmpInPos !== packedEnd) {
        throw new Error(
            `FastPFOR decode: packed region mismatch (pageStart=${pageStart}, packedStart=${inPos}, consumedPackedEnd=${tmpInPos}, expectedPackedEnd=${packedEnd}, packedWords=${packedEnd - inPos}, encoded.length=${inValues.length})`,
        );
    }

    return;
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
    workspace: FastPforDecoderWorkspace,
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

    const byteSize = inValues[packedEnd] >>> 0;
    const metaInts = (byteSize + 3) >>> 2;
    const byteContainerStart = packedEnd + 1;
    const bitmapPos = byteContainerStart + metaInts;

    if (bitmapPos >= inValues.length) {
        throw new Error(
            `FastPFOR decode: invalid byteSize=${byteSize} (metaInts=${metaInts}, pageStart=${pageStart}, packedEnd=${packedEnd}, byteContainerStart=${byteContainerStart}) causes bitmapPos=${bitmapPos} out of bounds (encoded.length=${inValues.length})`,
        );
    }

    const byteContainer = materializeByteContainer(inValues, byteContainerStart, byteSize, workspace);
    const byteContainerLen = byteSize;

    const inExcept = unpackExceptionStreams(inValues, bitmapPos, workspace);

    const exceptionPointers = workspace.dataPointers;
    exceptionPointers.fill(0);
    const startOutPos = outPos | 0;
    const blocks = (thisSize / BLOCK_SIZE) | 0;
    decodePageBlocks(
        inValues,
        pageStart,
        packedStart,
        packedEnd,
        out,
        startOutPos,
        blocks,
        byteContainer,
        byteContainerLen,
        workspace,
    );

    return inExcept;
}

function decodeAlignedPages(
    inValues: Int32Array,
    out: Int32Array,
    inPos: number,
    outPos: number,
    outLength: number,
    workspace: FastPforDecoderWorkspace,
): number {
    const alignedOutLength = greatestMultiple(outLength, BLOCK_SIZE);
    const finalOut = outPos + alignedOutLength;
    let tmpOutPos = outPos;
    let tmpInPos = inPos;

    while (tmpOutPos !== finalOut) {
        const thisSize = Math.min(PAGE_SIZE, finalOut - tmpOutPos);
        tmpInPos = decodePage(inValues, out, tmpInPos, tmpOutPos, thisSize, workspace);
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

    let bitOffset = 0;
    let wordIndex = inPos;
    const finalWordIndex = inPos + inLength;
    const outPos0 = outPos;
    let tmpOutPos = outPos;
    const targetOut = outPos + expectedCount;

    let accumulator = 0;
    let accumulatorShift = 0;

    while (wordIndex < finalWordIndex && tmpOutPos < targetOut) {
        const word = inValues[wordIndex];
        const byte = (word >>> bitOffset) & 0xff;
        bitOffset += 8;
        wordIndex += bitOffset >>> 5;
        bitOffset &= 31;

        accumulator |= (byte & 0x7f) << accumulatorShift;
        if ((byte & 0x80) !== 0) {
            out[tmpOutPos++] = accumulator | 0;
            accumulator = 0;
            accumulatorShift = 0;
        } else {
            accumulatorShift += 7;
            if (accumulatorShift > 28) {
                throw new Error(
                    `FastPFOR VByte: unterminated value (expected MSB=1 terminator within 5 bytes; shift=${accumulatorShift}, partial=${accumulator}, decoded=${tmpOutPos - outPos0}/${expectedCount}, inPos=${wordIndex}, inEnd=${finalWordIndex})`,
                );
            }
        }
    }

    if (tmpOutPos !== targetOut) {
        throw new Error(
            `FastPFOR VByte: truncated stream (decoded=${tmpOutPos - outPos0}, expected=${expectedCount}, consumedWords=${wordIndex - inPos}/${inLength}, vbyteStart=${inPos}, vbyteEnd=${finalWordIndex})`,
        );
    }

    return wordIndex;
}

/**
 * Decodes a sequence of FastPFOR-encoded integers.
 *
 * @param encoded The input buffer containing FastPFOR encoded data.
 * @param numValues The number of integers expected to be decoded.
 * @param workspace Optional workspace for reuse across calls. If omitted, a new workspace is created per call.
 */
export function decodeFastPforInt32(
    encoded: Int32Buf,
    numValues: number,
    workspace?: FastPforDecoderWorkspace,
): Int32Array {
    let inPos = 0;
    let outPos = 0;
    const decoded = new Int32Array(numValues);

    const decoderWorkspace = workspace ?? createDecoderWorkspace();

    if (encoded.length > 0) {
        const alignedLength = encoded[inPos] | 0;
        inPos = (inPos + 1) | 0;

        if (alignedLength < 0 || (alignedLength & (BLOCK_SIZE - 1)) !== 0) {
            throw new Error(
                `FastPFOR decode: invalid alignedLength=${alignedLength} (expected >= 0 and multiple of ${BLOCK_SIZE})`,
            );
        }

        if (outPos + alignedLength > decoded.length) {
            throw new Error(
                `FastPFOR decode: output buffer too small (outPos=${outPos}, alignedLength=${alignedLength}, out.length=${decoded.length})`,
            );
        }

        inPos = decodeAlignedPages(encoded, decoded, inPos, outPos, alignedLength, decoderWorkspace);
        outPos = (outPos + alignedLength) | 0;
    }

    const remainingLength = (encoded.length - inPos) | 0;
    const expectedTail = (numValues - outPos) | 0;
    inPos = decodeVByte(encoded, inPos, remainingLength, decoded, outPos, expectedTail);
    outPos = (outPos + expectedTail) | 0;

    return decoded;
}

function fastUnpack32(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number, bitWidth: number): void {
    switch (bitWidth) {
        case 2:
            fastUnpack32_2(inValues, inPos, out, outPos);
            return;
        case 3:
            fastUnpack32_3(inValues, inPos, out, outPos);
            return;
        case 4:
            fastUnpack32_4(inValues, inPos, out, outPos);
            return;
        case 5:
            fastUnpack32_5(inValues, inPos, out, outPos);
            return;
        case 6:
            fastUnpack32_6(inValues, inPos, out, outPos);
            return;
        case 7:
            fastUnpack32_7(inValues, inPos, out, outPos);
            return;
        case 8:
            fastUnpack32_8(inValues, inPos, out, outPos);
            return;
        case 9:
            fastUnpack32_9(inValues, inPos, out, outPos);
            return;
        case 10:
            fastUnpack32_10(inValues, inPos, out, outPos);
            return;
        case 11:
            fastUnpack32_11(inValues, inPos, out, outPos);
            return;
        case 12:
            fastUnpack32_12(inValues, inPos, out, outPos);
            return;
        case 16:
            fastUnpack32_16(inValues, inPos, out, outPos);
            return;
        case 32:
            for (let i = 0; i < 32; i = (i + 1) | 0) {
                out[(outPos + i) | 0] = inValues[(inPos + i) | 0] | 0;
            }
            return;
        default:
            break;
    }

    const valueMask = MASKS[bitWidth] >>> 0;
    let inputWordIndex = inPos;
    let bitOffset = 0;
    let currentWord = inValues[inputWordIndex] >>> 0;

    for (let i = 0; i < 32; i++) {
        if (bitOffset + bitWidth <= 32) {
            const value = (currentWord >>> bitOffset) & valueMask;
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

            const value = (low | (high << lowBits)) & valueMask;
            out[outPos + i] = value | 0;
            bitOffset = bitWidth - lowBits;
        }
    }
}
