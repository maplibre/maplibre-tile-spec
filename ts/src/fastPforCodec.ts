import IntWrapper from "./decoding/intWrapper";

type Int32Buf = Int32Array<ArrayBufferLike>;

const MASKS = (() => {
    const masks = new Uint32Array(33);
    masks[0] = 0;
    for (let bitWidth = 1; bitWidth <= 32; bitWidth++) {
        masks[bitWidth] = bitWidth === 32 ? 0xffffffff : 0xffffffff >>> (32 - bitWidth);
    }
    return masks;
})();

const OVERHEAD_OF_EACH_EXCEPT = 8;
const DEFAULT_PAGE_SIZE = 65536;
const BLOCK_SIZE = 256;

interface Int32Codec {
    compress(inValues: Int32Array, inPos: IntWrapper, inLength: number, out: Int32Buf, outPos: IntWrapper): Int32Buf;
    uncompress(inValues: Int32Array, inPos: IntWrapper, inLength: number, out: Int32Array, outPos: IntWrapper): void;
}

function greatestMultiple(value: number, factor: number): number {
    return value - (value % factor);
}

function roundUpToMultipleOf32(value: number): number {
    return greatestMultiple(value + 31, 32);
}

function bits(value: number): number {
    return 32 - Math.clz32(value);
}

function normalizePageSize(pageSize: number): number {
    if (!Number.isFinite(pageSize) || pageSize <= 0) return DEFAULT_PAGE_SIZE;

    const aligned = greatestMultiple(Math.floor(pageSize), BLOCK_SIZE);
    return aligned === 0 ? BLOCK_SIZE : aligned;
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

function ensureUint8Capacity(buffer: Uint8Array, requiredLength: number): Uint8Array {
    if (requiredLength <= buffer.length) return buffer;

    let newLength = buffer.length === 0 ? 1 : buffer.length;
    while (newLength < requiredLength) {
        newLength *= 2;
    }

    const next = new Uint8Array(newLength);
    next.set(buffer);
    return next;
}

function getMask(bitWidth: number): number {
    return MASKS[bitWidth] >>> 0;
}

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

    const mask = getMask(bitWidth);
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
}

function fastUnpack32_10(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    const in0 = inValues[inPos + 0] >>> 0;
    const in1 = inValues[inPos + 1] >>> 0;
    const in2 = inValues[inPos + 2] >>> 0;
    const in3 = inValues[inPos + 3] >>> 0;
    const in4 = inValues[inPos + 4] >>> 0;
    const in5 = inValues[inPos + 5] >>> 0;
    const in6 = inValues[inPos + 6] >>> 0;
    const in7 = inValues[inPos + 7] >>> 0;
    const in8 = inValues[inPos + 8] >>> 0;
    const in9 = inValues[inPos + 9] >>> 0;

    out[outPos + 0] = ((in0 >>> 0) & 0x3ff) | 0;
    out[outPos + 1] = ((in0 >>> 10) & 0x3ff) | 0;
    out[outPos + 2] = ((in0 >>> 20) & 0x3ff) | 0;
    out[outPos + 3] = (((in0 >>> 30) | ((in1 & 0xff) << 2)) & 0x3ff) | 0;
    out[outPos + 4] = ((in1 >>> 8) & 0x3ff) | 0;
    out[outPos + 5] = ((in1 >>> 18) & 0x3ff) | 0;
    out[outPos + 6] = (((in1 >>> 28) | ((in2 & 0x3f) << 4)) & 0x3ff) | 0;
    out[outPos + 7] = ((in2 >>> 6) & 0x3ff) | 0;
    out[outPos + 8] = ((in2 >>> 16) & 0x3ff) | 0;
    out[outPos + 9] = (((in2 >>> 26) | ((in3 & 0xf) << 6)) & 0x3ff) | 0;
    out[outPos + 10] = ((in3 >>> 4) & 0x3ff) | 0;
    out[outPos + 11] = ((in3 >>> 14) & 0x3ff) | 0;
    out[outPos + 12] = (((in3 >>> 24) | ((in4 & 0x3) << 8)) & 0x3ff) | 0;
    out[outPos + 13] = ((in4 >>> 2) & 0x3ff) | 0;
    out[outPos + 14] = ((in4 >>> 12) & 0x3ff) | 0;
    out[outPos + 15] = ((in4 >>> 22) & 0x3ff) | 0;
    out[outPos + 16] = ((in5 >>> 0) & 0x3ff) | 0;
    out[outPos + 17] = ((in5 >>> 10) & 0x3ff) | 0;
    out[outPos + 18] = ((in5 >>> 20) & 0x3ff) | 0;
    out[outPos + 19] = (((in5 >>> 30) | ((in6 & 0xff) << 2)) & 0x3ff) | 0;
    out[outPos + 20] = ((in6 >>> 8) & 0x3ff) | 0;
    out[outPos + 21] = ((in6 >>> 18) & 0x3ff) | 0;
    out[outPos + 22] = (((in6 >>> 28) | ((in7 & 0x3f) << 4)) & 0x3ff) | 0;
    out[outPos + 23] = ((in7 >>> 6) & 0x3ff) | 0;
    out[outPos + 24] = ((in7 >>> 16) & 0x3ff) | 0;
    out[outPos + 25] = (((in7 >>> 26) | ((in8 & 0xf) << 6)) & 0x3ff) | 0;
    out[outPos + 26] = ((in8 >>> 4) & 0x3ff) | 0;
    out[outPos + 27] = ((in8 >>> 14) & 0x3ff) | 0;
    out[outPos + 28] = (((in8 >>> 24) | ((in9 & 0x3) << 8)) & 0x3ff) | 0;
    out[outPos + 29] = ((in9 >>> 2) & 0x3ff) | 0;
    out[outPos + 30] = ((in9 >>> 12) & 0x3ff) | 0;
    out[outPos + 31] = ((in9 >>> 22) & 0x3ff) | 0;
}

function fastUnpack32_17(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number): void {
    const in0 = inValues[inPos + 0] >>> 0;
    const in1 = inValues[inPos + 1] >>> 0;
    const in2 = inValues[inPos + 2] >>> 0;
    const in3 = inValues[inPos + 3] >>> 0;
    const in4 = inValues[inPos + 4] >>> 0;
    const in5 = inValues[inPos + 5] >>> 0;
    const in6 = inValues[inPos + 6] >>> 0;
    const in7 = inValues[inPos + 7] >>> 0;
    const in8 = inValues[inPos + 8] >>> 0;
    const in9 = inValues[inPos + 9] >>> 0;
    const in10 = inValues[inPos + 10] >>> 0;
    const in11 = inValues[inPos + 11] >>> 0;
    const in12 = inValues[inPos + 12] >>> 0;
    const in13 = inValues[inPos + 13] >>> 0;
    const in14 = inValues[inPos + 14] >>> 0;
    const in15 = inValues[inPos + 15] >>> 0;
    const in16 = inValues[inPos + 16] >>> 0;

    out[outPos + 0] = ((in0 >>> 0) & 0x1ffff) | 0;
    out[outPos + 1] = (((in0 >>> 17) | ((in1 & 0x3) << 15)) & 0x1ffff) | 0;
    out[outPos + 2] = ((in1 >>> 2) & 0x1ffff) | 0;
    out[outPos + 3] = (((in1 >>> 19) | ((in2 & 0xf) << 13)) & 0x1ffff) | 0;
    out[outPos + 4] = ((in2 >>> 4) & 0x1ffff) | 0;
    out[outPos + 5] = (((in2 >>> 21) | ((in3 & 0x3f) << 11)) & 0x1ffff) | 0;
    out[outPos + 6] = ((in3 >>> 6) & 0x1ffff) | 0;
    out[outPos + 7] = (((in3 >>> 23) | ((in4 & 0xff) << 9)) & 0x1ffff) | 0;
    out[outPos + 8] = ((in4 >>> 8) & 0x1ffff) | 0;
    out[outPos + 9] = (((in4 >>> 25) | ((in5 & 0x3ff) << 7)) & 0x1ffff) | 0;
    out[outPos + 10] = ((in5 >>> 10) & 0x1ffff) | 0;
    out[outPos + 11] = (((in5 >>> 27) | ((in6 & 0xfff) << 5)) & 0x1ffff) | 0;
    out[outPos + 12] = ((in6 >>> 12) & 0x1ffff) | 0;
    out[outPos + 13] = (((in6 >>> 29) | ((in7 & 0x3fff) << 3)) & 0x1ffff) | 0;
    out[outPos + 14] = ((in7 >>> 14) & 0x1ffff) | 0;
    out[outPos + 15] = (((in7 >>> 31) | ((in8 & 0xffff) << 1)) & 0x1ffff) | 0;
    out[outPos + 16] = (((in8 >>> 16) | ((in9 & 0x1) << 16)) & 0x1ffff) | 0;
    out[outPos + 17] = ((in9 >>> 1) & 0x1ffff) | 0;
    out[outPos + 18] = (((in9 >>> 18) | ((in10 & 0x7) << 14)) & 0x1ffff) | 0;
    out[outPos + 19] = ((in10 >>> 3) & 0x1ffff) | 0;
    out[outPos + 20] = (((in10 >>> 20) | ((in11 & 0x1f) << 12)) & 0x1ffff) | 0;
    out[outPos + 21] = ((in11 >>> 5) & 0x1ffff) | 0;
    out[outPos + 22] = (((in11 >>> 22) | ((in12 & 0x7f) << 10)) & 0x1ffff) | 0;
    out[outPos + 23] = ((in12 >>> 7) & 0x1ffff) | 0;
    out[outPos + 24] = (((in12 >>> 24) | ((in13 & 0x1ff) << 8)) & 0x1ffff) | 0;
    out[outPos + 25] = ((in13 >>> 9) & 0x1ffff) | 0;
    out[outPos + 26] = (((in13 >>> 26) | ((in14 & 0x7ff) << 6)) & 0x1ffff) | 0;
    out[outPos + 27] = ((in14 >>> 11) & 0x1ffff) | 0;
    out[outPos + 28] = (((in14 >>> 28) | ((in15 & 0x1fff) << 4)) & 0x1ffff) | 0;
    out[outPos + 29] = ((in15 >>> 13) & 0x1ffff) | 0;
    out[outPos + 30] = (((in15 >>> 30) | ((in16 & 0x7fff) << 2)) & 0x1ffff) | 0;
    out[outPos + 31] = ((in16 >>> 15) & 0x1ffff) | 0;
}

/**
 * Generic bit-unpacking of 32 integers, matching JavaFastPFOR BitPacking.fastunpack ordering.
 * Reads exactly `bitWidth` int32 words from `inValues` starting at `inPos`.
 */
function fastUnpack32(inValues: Int32Array, inPos: number, out: Int32Array, outPos: number, bitWidth: number): void {
    if (bitWidth === 0) {
        out.fill(0, outPos, outPos + 32);
        return;
    }
    if (bitWidth === 32) {
        out.set(inValues.subarray(inPos, inPos + 32), outPos);
        return;
    }

    if (bitWidth === 10) {
        fastUnpack32_10(inValues, inPos, out, outPos);
        return;
    }
    if (bitWidth === 17) {
        fastUnpack32_17(inValues, inPos, out, outPos);
        return;
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

class FastPfor implements Int32Codec {
    private readonly pageSize: number;
    private dataToBePacked: Int32Array[] = new Array(33);
    private byteContainer: Uint8Array;
    private byteContainerPos = 0;
    private readonly dataPointers = new Int32Array(33);
    private readonly freqs = new Int32Array(33);
    private readonly best = new Int32Array(3);

    constructor(pageSize = DEFAULT_PAGE_SIZE) {
        this.pageSize = normalizePageSize(pageSize);

        const byteContainerSize = (3 * this.pageSize) / BLOCK_SIZE + this.pageSize;
        this.byteContainer = new Uint8Array(byteContainerSize);

        const initialPackedSize = (this.pageSize / 32) * 4;
        for (let k = 1; k < this.dataToBePacked.length; k++) {
            this.dataToBePacked[k] = new Int32Array(initialPackedSize);
        }
    }

    public compress(inValues: Int32Array, inPos: IntWrapper, inLength: number, out: Int32Buf, outPos: IntWrapper): Int32Buf {
        const alignedLength = greatestMultiple(inLength, BLOCK_SIZE);
        if (alignedLength === 0) return out;

        out = ensureInt32Capacity(out, outPos.get() + 1);
        out[outPos.get()] = alignedLength;
        outPos.increment();

        return this.headlessCompress(inValues, inPos, alignedLength, out, outPos);
    }

    private headlessCompress(
        inValues: Int32Array,
        inPos: IntWrapper,
        inLength: number,
        out: Int32Buf,
        outPos: IntWrapper,
    ): Int32Buf {
        const alignedLength = greatestMultiple(inLength, BLOCK_SIZE);
        const finalInPos = inPos.get() + alignedLength;

        while (inPos.get() !== finalInPos) {
            const thisSize = Math.min(this.pageSize, finalInPos - inPos.get());
            out = this.encodePage(inValues, inPos, thisSize, out, outPos);
        }

        return out;
    }

    private getBestBFromData(inValues: Int32Array, pos: number): void {
        this.freqs.fill(0);
        for (let k = pos, kEnd = pos + BLOCK_SIZE; k < kEnd; k++) {
            this.freqs[bits(inValues[k])]++;
        }

        let maxBits = 32;
        while (this.freqs[maxBits] === 0) maxBits--;

        let bestB = maxBits;
        let bestCost = maxBits * BLOCK_SIZE;
        let cExcept = 0;
        let bestCExcept = cExcept;

        for (let b = maxBits - 1; b >= 0; b--) {
            cExcept += this.freqs[b + 1];
            if (cExcept === BLOCK_SIZE) break;

            let thisCost =
                cExcept * OVERHEAD_OF_EACH_EXCEPT + cExcept * (maxBits - b) + b * BLOCK_SIZE + 8;
            if (maxBits - b === 1) thisCost -= cExcept;

            if (thisCost < bestCost) {
                bestCost = thisCost;
                bestB = b;
                bestCExcept = cExcept;
            }
        }

        this.best[0] = bestB;
        this.best[1] = bestCExcept;
        this.best[2] = maxBits;
    }

    private byteContainerClear(): void {
        this.byteContainerPos = 0;
    }

    private byteContainerPut(byteValue: number): void {
        if (this.byteContainerPos >= this.byteContainer.length) {
            this.byteContainer = ensureUint8Capacity(this.byteContainer, this.byteContainerPos + 1);
        }
        this.byteContainer[this.byteContainerPos++] = byteValue & 0xff;
    }

    private encodePage(
        inValues: Int32Array,
        inPos: IntWrapper,
        thisSize: number,
        out: Int32Buf,
        outPos: IntWrapper,
    ): Int32Buf {
        const headerPos = outPos.get();
        out = ensureInt32Capacity(out, headerPos + 1);
        outPos.increment();
        let tmpOutPos = outPos.get();

        this.dataPointers.fill(0);
        this.byteContainerClear();

        let tmpInPos = inPos.get();
        const finalInPos = tmpInPos + thisSize - BLOCK_SIZE;

        for (; tmpInPos <= finalInPos; tmpInPos += BLOCK_SIZE) {
            this.getBestBFromData(inValues, tmpInPos);

            const bestB = this.best[0];
            const cExcept = this.best[1];
            const maxBits = this.best[2];

            this.byteContainerPut(bestB);
            this.byteContainerPut(cExcept);

            if (cExcept > 0) {
                this.byteContainerPut(maxBits);
                const index = maxBits - bestB;

                if (index >= 0 && index < this.dataToBePacked.length) {
                    const needed = this.dataPointers[index] + cExcept;
                    if (needed >= this.dataToBePacked[index].length) {
                        let newSize = 2 * needed;
                        newSize = roundUpToMultipleOf32(newSize);
                        const next = new Int32Array(newSize);
                        next.set(this.dataToBePacked[index]);
                        this.dataToBePacked[index] = next;
                    }
                }

                for (let k = 0; k < BLOCK_SIZE; k++) {
                    const value = inValues[tmpInPos + k] >>> 0;
                    if ((value >>> bestB) !== 0) {
                        this.byteContainerPut(k);
                        if (index !== 1) {
                            this.dataToBePacked[index][this.dataPointers[index]++] = (value >>> bestB) | 0;
                        }
                    }
                }
            }

            for (let k = 0; k < BLOCK_SIZE; k += 32) {
                out = ensureInt32Capacity(out, tmpOutPos + bestB);
                fastPack32(inValues, tmpInPos + k, out, tmpOutPos, bestB);
                tmpOutPos += bestB;
            }
        }

        inPos.set(tmpInPos);
        out[headerPos] = (tmpOutPos - headerPos) | 0;

        const byteSize = this.byteContainerPos;
        while ((this.byteContainerPos & 3) !== 0) this.byteContainerPut(0);

        out = ensureInt32Capacity(out, tmpOutPos + 1);
        out[tmpOutPos++] = byteSize | 0;

        const howManyInts = this.byteContainerPos / 4;
        out = ensureInt32Capacity(out, tmpOutPos + howManyInts);
        for (let i = 0; i < howManyInts; i++) {
            const base = i * 4;
            // byteContainer is serialized in little-endian inside int32 words (matching JavaFastPFOR),
            // independent of how the overall Int32 stream is later converted to bytes.
            const v =
                (this.byteContainer[base] |
                    (this.byteContainer[base + 1] << 8) |
                    (this.byteContainer[base + 2] << 16) |
                    (this.byteContainer[base + 3] << 24)) |
                0;
            out[tmpOutPos + i] = v;
        }
        tmpOutPos += howManyInts;

        let bitmap = 0;
        for (let k = 2; k <= 32; k++) {
            if (this.dataPointers[k] !== 0) bitmap |= 1 << (k - 1);
        }

        out = ensureInt32Capacity(out, tmpOutPos + 1);
        out[tmpOutPos++] = bitmap | 0;

        for (let k = 2; k <= 32; k++) {
            const size = this.dataPointers[k];
            if (size !== 0) {
                out = ensureInt32Capacity(out, tmpOutPos + 1);
                out[tmpOutPos++] = size | 0;

                let j = 0;
                for (; j < size; j += 32) {
                    out = ensureInt32Capacity(out, tmpOutPos + k);
                    fastPack32(this.dataToBePacked[k], j, out, tmpOutPos, k);
                    tmpOutPos += k;
                }

                const overflow = j - size;
                tmpOutPos -= Math.floor((overflow * k) / 32);
            }
        }

        outPos.set(tmpOutPos);
        return out;
    }

    public uncompress(inValues: Int32Array, inPos: IntWrapper, inLength: number, out: Int32Array, outPos: IntWrapper): void {
        if (inLength === 0) return;
        const outLength = inValues[inPos.get()];
        inPos.increment();
        this.headlessUncompress(inValues, inPos, inLength, out, outPos, outLength);
    }

    private headlessUncompress(
        inValues: Int32Array,
        inPos: IntWrapper,
        _inLength: number,
        out: Int32Array,
        outPos: IntWrapper,
        outLength: number,
    ): void {
        const alignedOutLength = greatestMultiple(outLength, BLOCK_SIZE);
        const finalOut = outPos.get() + alignedOutLength;
        while (outPos.get() !== finalOut) {
            const thisSize = Math.min(this.pageSize, finalOut - outPos.get());
            this.decodePage(inValues, inPos, out, outPos, thisSize);
        }
    }

    private decodePage(inValues: Int32Array, inPos: IntWrapper, out: Int32Array, outPos: IntWrapper, thisSize: number): void {
        const initPos = inPos.get();
        const whereMeta = inValues[inPos.get()];
        inPos.increment();

        let inExcept = initPos + whereMeta;
        const byteSize = inValues[inExcept++] >>> 0;

        const byteContainer = new Uint8Array(byteSize);
        for (let i = 0; i < byteSize; i++) {
            const intIdx = inExcept + (i >> 2);
            const byteInInt = i & 3;
            byteContainer[i] = (inValues[intIdx] >>> (byteInInt * 8)) & 0xff;
        }

        inExcept += Math.ceil(byteSize / 4);

        const bitmap = inValues[inExcept++];

        for (let k = 2; k <= 32; k++) {
            if ((bitmap & (1 << (k - 1))) !== 0) {
                const size = inValues[inExcept++];
                const roundedUp = roundUpToMultipleOf32(size);

                if (this.dataToBePacked[k].length < roundedUp) {
                    this.dataToBePacked[k] = new Int32Array(roundedUp);
                }

                let j = 0;
                for (; j < size; j += 32) {
                    fastUnpack32(inValues, inExcept, this.dataToBePacked[k], j, k);
                    inExcept += k;
                }

                const overflow = j - size;
                inExcept -= Math.floor((overflow * k) / 32);
            }
        }

        this.dataPointers.fill(0);
        let tmpOutPos = outPos.get();
        let tmpInPos = inPos.get();

        let bytePosIn = 0;
        const blocks = thisSize / BLOCK_SIZE;

        for (let run = 0; run < blocks; run++, tmpOutPos += BLOCK_SIZE) {
            const b = byteContainer[bytePosIn++];
            const cExcept = byteContainer[bytePosIn++];

            switch (b) {
                case 0:
                    out.fill(0, tmpOutPos, tmpOutPos + BLOCK_SIZE);
                    break;
                case 10:
                    for (let k = 0; k < BLOCK_SIZE; k += 32) {
                        fastUnpack32_10(inValues, tmpInPos, out, tmpOutPos + k);
                        tmpInPos += 10;
                    }
                    break;
                case 17:
                    for (let k = 0; k < BLOCK_SIZE; k += 32) {
                        fastUnpack32_17(inValues, tmpInPos, out, tmpOutPos + k);
                        tmpInPos += 17;
                    }
                    break;
                case 32:
                    out.set(inValues.subarray(tmpInPos, tmpInPos + BLOCK_SIZE), tmpOutPos);
                    tmpInPos += BLOCK_SIZE;
                    break;
                default:
                    for (let k = 0; k < BLOCK_SIZE; k += 32) {
                        fastUnpack32(inValues, tmpInPos, out, tmpOutPos + k, b);
                        tmpInPos += b;
                    }
                    break;
            }

            if (cExcept > 0) {
                const maxBits = byteContainer[bytePosIn++];
                const index = maxBits - b;

                if (index === 1) {
                    for (let k = 0; k < cExcept; k++) {
                        const pos = byteContainer[bytePosIn++];
                        out[pos + tmpOutPos] |= 1 << b;
                    }
                } else {
                    for (let k = 0; k < cExcept; k++) {
                        const pos = byteContainer[bytePosIn++];
                        const exceptValue = this.dataToBePacked[index][this.dataPointers[index]++];
                        out[pos + tmpOutPos] |= exceptValue << b;
                    }
                }
            }
        }

        outPos.set(tmpOutPos);
        inPos.set(inExcept);
    }
}

class VariableByte implements Int32Codec {
    public compress(inValues: Int32Array, inPos: IntWrapper, inLength: number, out: Int32Buf, outPos: IntWrapper): Int32Buf {
        if (inLength === 0) return out;

        const bytes: number[] = [];
        bytes.length = 0;

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
            const v = (bytes[i] | (bytes[i + 1] << 8) | (bytes[i + 2] << 16) | (bytes[i + 3] << 24)) | 0;
            out[outIdx++] = v;
        }

        outPos.set(outIdx);
        inPos.add(inLength);
        return out;
    }

    public uncompress(inValues: Int32Array, inPos: IntWrapper, inLength: number, out: Int32Array, outPos: IntWrapper): void {
        let s = 0;
        let p = inPos.get();
        const finalP = inPos.get() + inLength;
        let tmpOutPos = outPos.get();

        let v = 0;
        let shift = 0;

        while (p < finalP) {
            const val = inValues[p];
            const c = (val >>> s) & 0xff;
            s += 8;
            p += s >>> 5;
            s &= 31;

            v += (c & 127) << shift;
            if ((c & 128) === 128) {
                out[tmpOutPos++] = v;
                v = 0;
                shift = 0;
            } else {
                shift += 7;
            }
        }

        outPos.set(tmpOutPos);
        inPos.add(inLength);
    }
}

class Composition {
    constructor(
        private readonly first: Int32Codec,
        private readonly second: Int32Codec,
    ) {}

    public compress(inValues: Int32Array, inPos: IntWrapper, inLength: number, out: Int32Buf, outPos: IntWrapper): Int32Buf {
        if (inLength === 0) return out;

        const inPosInit = inPos.get();
        const outPosInit = outPos.get();

        out = this.first.compress(inValues, inPos, inLength, out, outPos);
        if (outPos.get() === outPosInit) {
            out = ensureInt32Capacity(out, outPosInit + 1);
            out[outPosInit] = 0;
            outPos.increment();
        }

        const remaining = inLength - (inPos.get() - inPosInit);
        out = this.second.compress(inValues, inPos, remaining, out, outPos);
        return out;
    }

    public uncompress(inValues: Int32Array, inPos: IntWrapper, inLength: number, out: Int32Array, outPos: IntWrapper): void {
        if (inLength === 0) return;
        const init = inPos.get();
        this.first.uncompress(inValues, inPos, inLength, out, outPos);
        const remainingLength = inLength - (inPos.get() - init);
        this.second.uncompress(inValues, inPos, remainingLength, out, outPos);
    }
}

const fastPforCodec = new Composition(new FastPfor(), new VariableByte());

export function compressFastPforInt32(values: Int32Array): Int32Buf {
    const inPos = new IntWrapper(0);
    const outPos = new IntWrapper(0);
    let out = new Int32Array(values.length + 1024) as Int32Buf;
    out = fastPforCodec.compress(values, inPos, values.length, out, outPos);
    return out.subarray(0, outPos.get());
}

export function uncompressFastPforInt32(encoded: Int32Buf, numValues: number): Int32Array {
    const inPos = new IntWrapper(0);
    const outPos = new IntWrapper(0);
    const decoded = new Int32Array(numValues);
    fastPforCodec.uncompress(encoded, inPos, encoded.length, decoded, outPos);
    return decoded;
}

export function int32sToBigEndianBytes(values: Int32Buf): Uint8Array {
    // Note: the FastPFOR codec operates on an Int32 stream. When converted to bytes for the tile format,
    // we serialize those int32 words using big-endian order (consistent with existing MLT TS code paths).
    const bytes = new Uint8Array(values.length * 4);
    for (let i = 0; i < values.length; i++) {
        const v = values[i];
        const base = i * 4;
        bytes[base] = (v >>> 24) & 0xff;
        bytes[base + 1] = (v >>> 16) & 0xff;
        bytes[base + 2] = (v >>> 8) & 0xff;
        bytes[base + 3] = v & 0xff;
    }
    return bytes;
}

export function bigEndianBytesToInt32s(bytes: Uint8Array, offset: number, byteLength: number): Int32Buf {
    const numCompleteInts = Math.floor(byteLength / 4);
    const hasTrailingBytes = byteLength % 4 !== 0;
    const numInts = hasTrailingBytes ? numCompleteInts + 1 : numCompleteInts;

    const ints = new Int32Array(numInts) as Int32Buf;
    if (numCompleteInts > 0) {
        const absoluteOffset = bytes.byteOffset + offset;
        if ((absoluteOffset & 3) === 0) {
            const u32 = new Uint32Array(bytes.buffer, absoluteOffset, numCompleteInts);
            for (let i = 0; i < numCompleteInts; i++) {
                ints[i] = bswap32(u32[i]) | 0;
            }
        } else {
            for (let i = 0; i < numCompleteInts; i++) {
                const base = offset + i * 4;
                ints[i] =
                    ((bytes[base] << 24) | (bytes[base + 1] << 16) | (bytes[base + 2] << 8) | bytes[base + 3]) |
                    0;
            }
        }
    }

    if (hasTrailingBytes) {
        const base = offset + numCompleteInts * 4;
        const remaining = byteLength - numCompleteInts * 4;
        let v = 0;
        for (let i = 0; i < remaining; i++) {
            v |= bytes[base + i] << (24 - i * 8);
        }
        ints[numCompleteInts] = v | 0;
    }
    return ints;
}

function bswap32(value: number): number {
    const x = value >>> 0;
    return (
        (((x & 0xff) << 24) |
            ((x & 0xff00) << 8) |
            ((x >>> 8) & 0xff00) |
            ((x >>> 24) & 0xff)) >>>
        0
    );
}
