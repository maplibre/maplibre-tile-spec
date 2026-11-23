import { IS_LE } from "./binary";

const PAGE_SIZE = 65536;
const BLOCK_SIZE = 128;
const TAIL32 = new Int32Array(32);

function unpack32(w: Int32Array, ip: number, out: Int32Array, op: number, b: number): void {
    const mask = (1 << b) - 1;
    const splitBit = (b - 1) << 5;
    let bitPos = 0;
    let i = 0;
    while (bitPos < splitBit) {
        const wi = ip + (bitPos >>> 5);
        const sh = bitPos & 31;
        out[op + i] = ((w[wi] >>> sh) | ((w[wi + 1] << 1) << (31 - sh))) & mask;
        bitPos += b;
        i++;
    }
    const last = w[ip + b - 1];
    for (; i < 32; i++, bitPos += b) out[op + i] = (last >>> (bitPos & 31)) & mask;
}

export function decodeFastPfor128Into(data: Uint8Array, numValues: number, out: Uint32Array): Uint32Array {
    if (numValues === 0) {
        if (data.length !== 0) throw new Error("fpf empty");
        return out;
    }
    if ((data.length & 3) !== 0) throw new Error("fpf align");
    const wordCount = data.length >>> 2;

    let words: Int32Array;
    if (IS_LE && (data.byteOffset & 3) === 0) {
        words = new Int32Array(data.buffer, data.byteOffset, wordCount);
    } else {
        const copy = new Uint8Array(data.length);
        if (IS_LE) copy.set(data);
        else {
            const dv = new DataView(data.buffer, data.byteOffset, data.length);
            words = new Int32Array(copy.buffer);
            for (let i = 0; i < wordCount; i++) words[i] = dv.getInt32(i * 4, true);
        }
        words = new Int32Array(copy.buffer);
    }

    const outI32 = new Int32Array(out.buffer, out.byteOffset, numValues);
    const exStreams: Array<Int32Array | undefined> = new Array(33);
    const exSizes = new Int32Array(33);
    const exPtrs = new Int32Array(33);
    let inPos = 0;
    let outPos = 0;

    if (wordCount > 0) {
        const alignedLength = words[inPos++] >>> 0;
        if ((alignedLength & (BLOCK_SIZE - 1)) !== 0 || alignedLength > numValues) throw new Error("fpf len");

        while (outPos < alignedLength) {
            const thisSize = Math.min(PAGE_SIZE, alignedLength - outPos);
            if (inPos >= wordCount) throw new Error("fpf page");
            const pageStart = inPos;
            const whereMeta = words[pageStart] >>> 0;
            if (whereMeta === 0 || pageStart + whereMeta >= wordCount) throw new Error("fpf meta");
            const packedStart = pageStart + 1;
            const packedEnd = pageStart + whereMeta;
            const byteSize = words[packedEnd] >>> 0;
            const metaStart = packedEnd + 1;
            const bitmapPos = metaStart + ((byteSize + 3) >>> 2);
            if (bitmapPos >= wordCount) throw new Error("fpf meta eof");

            const metaView = new Uint8Array(words.buffer, words.byteOffset + metaStart * 4, byteSize);
            let inExcept = bitmapPos;
            const bitmap = words[inExcept++];
            exSizes.fill(0);
            exPtrs.fill(0);

            for (let k = 2; bitmap !== 0 && k <= 32; k++) {
                if (((bitmap >>> (k - 1)) & 1) === 0) continue;
                if (inExcept >= wordCount) throw new Error("fpf ex");
                const size = words[inExcept++] >>> 0;
                const wordsNeeded = (size * k + 31) >>> 5;
                if (size > PAGE_SIZE || inExcept + wordsNeeded > wordCount) throw new Error("fpf ex eof");
                const stream = new Int32Array(Math.max(32, (size + 31) & ~31));
                exStreams[k] = stream;
                if (k === 32) {
                    stream.set(words.subarray(inExcept, inExcept + size));
                } else {
                    const fullGroups = size >>> 5;
                    for (let g = 0; g < fullGroups; g++) unpack32(words, inExcept + g * k, stream, g * 32, k);
                    if ((size & 31) !== 0) {
                        TAIL32.fill(0, 0, k);
                        TAIL32.set(words.subarray(inExcept + fullGroups * k, inExcept + wordsNeeded));
                        unpack32(TAIL32, 0, stream, fullGroups * 32, k);
                    }
                }
                inExcept += wordsNeeded;
                exSizes[k] = size;
            }

            let tmpInPos = packedStart;
            let bytePos = 0;
            const blocks = thisSize / BLOCK_SIZE;
            for (let b = 0; b < blocks; b++) {
                if (bytePos + 2 > byteSize) throw new Error("fpf hdr");
                const bitWidth = metaView[bytePos++];
                const exCount = metaView[bytePos++];
                if (bitWidth > 32 || exCount > BLOCK_SIZE) throw new Error("fpf hdr");
                const blockOutPos = outPos + b * BLOCK_SIZE;

                if (bitWidth === 0) {
                    outI32.fill(0, blockOutPos, blockOutPos + BLOCK_SIZE);
                } else if (bitWidth === 32) {
                    if (tmpInPos + BLOCK_SIZE > packedEnd) throw new Error("fpf blk");
                    outI32.set(words.subarray(tmpInPos, tmpInPos + BLOCK_SIZE), blockOutPos);
                    tmpInPos += BLOCK_SIZE;
                } else {
                    const blkWords = 4 * bitWidth;
                    if (tmpInPos + blkWords > packedEnd) throw new Error("fpf blk");
                    for (let g = 0; g < 4; g++)
                        unpack32(words, tmpInPos + g * bitWidth, outI32, blockOutPos + g * 32, bitWidth);
                    tmpInPos += blkWords;
                }

                if (exCount > 0) {
                    if (bytePos + 1 + exCount > byteSize) throw new Error("fpf pos");
                    const maxBits = metaView[bytePos++];
                    if (maxBits <= bitWidth || maxBits > 32) throw new Error("fpf maxBits");
                    const k = maxBits - bitWidth;
                    if (k > 1 && (!exStreams[k] || exPtrs[k] + exCount > exSizes[k])) throw new Error("fpf ex");
                    const stream = exStreams[k] ?? TAIL32;
                    for (let e = 0; e < exCount; e++) {
                        const pos = metaView[bytePos++];
                        if (pos >= BLOCK_SIZE) throw new Error("fpf pos");
                        outI32[blockOutPos + pos] |= (k === 1 ? 1 : stream[exPtrs[k]++]) << bitWidth;
                    }
                }
            }

            if (tmpInPos !== packedEnd) throw new Error("fpf end");
            inPos = inExcept;
            outPos += thisSize;
        }
    }

    if (outPos < numValues) {
        let wordIdx = inPos;
        let bitOffset = 0;
        let acc = 0;
        let shift = 0;
        while (wordIdx < wordCount && outPos < numValues) {
            const byte = (words[wordIdx] >>> bitOffset) & 0xff;
            bitOffset = (bitOffset + 8) & 31;
            if (bitOffset === 0) wordIdx++;
            acc |= (byte & 0x7f) << shift;
            if ((byte & 0x80) !== 0) {
                outI32[outPos++] = acc;
                acc = 0;
                shift = 0;
            } else {
                shift += 7;
                if (shift > 28) throw new Error("fpf vbyte");
            }
        }
        if (outPos !== numValues) throw new Error("fpf tail");
    }
    return out;
}
