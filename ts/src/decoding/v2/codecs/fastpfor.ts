import { IS_LE } from "./binary";

const PAGE_SIZE = 65536;
const exStreams: Array<Int32Array | undefined> = new Array(33);
const exSizes = new Int32Array(33);
const exPtrs = new Int32Array(33);
const TAIL32 = new Int32Array(32);
let fpfBytes = new Uint8Array(16384);

function unpack32(w: Int32Array, ip: number, out: Int32Array, op: number, b: number): void {
    switch (b) {
        case 1: {
            const a0 = w[ip];
            for (let i = 0; i < 32; i += 4) {
                out[op + i] = (a0 >>> i) & 1;
                out[op + i + 1] = (a0 >>> (i + 1)) & 1;
                out[op + i + 2] = (a0 >>> (i + 2)) & 1;
                out[op + i + 3] = (a0 >>> (i + 3)) & 1;
            }
            return;
        }
        case 2: {
            const a0 = w[ip];
            const a1 = w[ip + 1];
            for (let i = 0; i < 16; i++) out[op + i] = (a0 >>> (i << 1)) & 3;
            for (let i = 0; i < 16; i++) out[op + 16 + i] = (a1 >>> (i << 1)) & 3;
            return;
        }
        case 4: {
            for (let k = 0; k < 4; k++) {
                const v = w[ip + k];
                const o = op + (k << 3);
                out[o] = v & 15;
                out[o + 1] = (v >>> 4) & 15;
                out[o + 2] = (v >>> 8) & 15;
                out[o + 3] = (v >>> 12) & 15;
                out[o + 4] = (v >>> 16) & 15;
                out[o + 5] = (v >>> 20) & 15;
                out[o + 6] = (v >>> 24) & 15;
                out[o + 7] = v >>> 28;
            }
            return;
        }
        case 8: {
            for (let k = 0; k < 8; k++) {
                const v = w[ip + k];
                const o = op + (k << 2);
                out[o] = v & 255;
                out[o + 1] = (v >>> 8) & 255;
                out[o + 2] = (v >>> 16) & 255;
                out[o + 3] = v >>> 24;
            }
            return;
        }
        case 16: {
            for (let k = 0; k < 16; k++) {
                const v = w[ip + k];
                const o = op + (k << 1);
                out[o] = v & 0xffff;
                out[o + 1] = v >>> 16;
            }
            return;
        }
    }

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
    for (; i < 32; i++, bitPos += b) {
        out[op + i] = (last >>> (bitPos & 31)) & mask;
    }
}

export function decodeFastPforInto(data: Uint8Array, numValues: number, is128: boolean, out: Uint32Array): Uint32Array {
    if (numValues === 0) {
        if (data.length !== 0) throw new Error("fpf empty");
        return out;
    }
    const byteLen = data.length;
    if ((byteLen & 3) !== 0) throw new Error("fpf align");

    const blockSize = is128 ? 128 : 256;
    const groupsPerBlock = is128 ? 4 : 8;
    const wordCount = byteLen >>> 2;

    let words: Int32Array;
    if (IS_LE && is128 && (data.byteOffset & 3) === 0) {
        words = new Int32Array(data.buffer, data.byteOffset, wordCount);
    } else {
        if (fpfBytes.length < byteLen) {
            fpfBytes = new Uint8Array(Math.max(byteLen, fpfBytes.length * 2));
        }
        if (IS_LE) {
            fpfBytes.set(data);
            words = new Int32Array(fpfBytes.buffer, 0, wordCount);
            if (!is128) {
                for (let i = 0; i < wordCount; i++) {
                    const x = words[i];
                    words[i] = ((x & 0xff) << 24) | ((x & 0xff00) << 8) | ((x >>> 8) & 0xff00) | (x >>> 24);
                }
            }
        } else {
            words = new Int32Array(fpfBytes.buffer, 0, wordCount);
            const dv = new DataView(data.buffer, data.byteOffset, byteLen);
            for (let i = 0; i < wordCount; i++) words[i] = dv.getInt32(i * 4, is128);
        }
    }

    const outI32 = new Int32Array(out.buffer, out.byteOffset, numValues);
    let inPos = 0;
    let outPos = 0;

    if (wordCount > 0) {
        const alignedLength = words[inPos++] >>> 0;
        if ((alignedLength & (blockSize - 1)) !== 0 || alignedLength > numValues) {
            throw new Error("fpf len");
        }

        while (outPos < alignedLength) {
            const thisSize = Math.min(PAGE_SIZE, alignedLength - outPos);
            if (inPos >= wordCount) throw new Error("fpf page");
            const pageStart = inPos;
            const whereMeta = words[pageStart] >>> 0;
            if (whereMeta === 0 || pageStart + whereMeta >= wordCount) {
                throw new Error("fpf meta");
            }
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

            if (bitmap !== 0) {
                for (let k = 2; k <= 32; k++) {
                    if (((bitmap >>> (k - 1)) & 1) === 0) continue;
                    if (inExcept >= wordCount) throw new Error("fpf ex");
                    const size = words[inExcept++] >>> 0;
                    if (size > PAGE_SIZE) throw new Error("fpf ex size");
                    const allocLen = (size + 31) & ~31;
                    let stream = exStreams[k];
                    if (!stream || stream.length < allocLen) {
                        stream = new Int32Array(Math.max(32, allocLen));
                        exStreams[k] = stream;
                    }
                    const wordsNeeded = (size * k + 31) >>> 5;
                    if (inExcept + wordsNeeded > wordCount) throw new Error("fpf ex eof");
                    if (k === 32) {
                        stream.set(words.subarray(inExcept, inExcept + size));
                    } else {
                        const fullGroups = size >>> 5;
                        for (let g = 0; g < fullGroups; g++) {
                            unpack32(words, inExcept + g * k, stream, g * 32, k);
                        }
                        if ((size & 31) !== 0) {
                            const usedWords = fullGroups * k;
                            TAIL32.fill(0, 0, k);
                            TAIL32.set(words.subarray(inExcept + usedWords, inExcept + wordsNeeded));
                            unpack32(TAIL32, 0, stream, fullGroups * 32, k);
                        }
                    }
                    inExcept += wordsNeeded;
                    exSizes[k] = size;
                }
            }

            let tmpInPos = packedStart;
            let bytePos = 0;
            const blocks = thisSize / blockSize;
            for (let b = 0; b < blocks; b++) {
                if (bytePos + 2 > byteSize) throw new Error("fpf hdr");
                const bitWidth = metaView[bytePos++];
                const exCount = metaView[bytePos++];
                if (bitWidth > 32 || exCount > blockSize) throw new Error("fpf hdr");
                const blockOutPos = outPos + b * blockSize;

                if (bitWidth === 0) {
                    outI32.fill(0, blockOutPos, blockOutPos + blockSize);
                } else if (bitWidth === 32) {
                    if (tmpInPos + blockSize > packedEnd) throw new Error("fpf blk");
                    outI32.set(words.subarray(tmpInPos, tmpInPos + blockSize), blockOutPos);
                    tmpInPos += blockSize;
                } else {
                    const blkWords = groupsPerBlock * bitWidth;
                    if (tmpInPos + blkWords > packedEnd) throw new Error("fpf blk");
                    for (let g = 0; g < groupsPerBlock; g++) {
                        unpack32(words, tmpInPos + g * bitWidth, outI32, blockOutPos + g * 32, bitWidth);
                    }
                    tmpInPos += blkWords;
                }

                if (exCount > 0) {
                    if (bytePos + 1 + exCount > byteSize) throw new Error("fpf pos");
                    const maxBits = metaView[bytePos++];
                    if (maxBits <= bitWidth || maxBits > 32) throw new Error("fpf maxBits");
                    const k = maxBits - bitWidth;
                    if (k === 1) {
                        const patch = 1 << bitWidth;
                        for (let e = 0; e < exCount; e++) {
                            const pos = metaView[bytePos++];
                            if (pos >= blockSize) throw new Error("fpf pos");
                            outI32[blockOutPos + pos] |= patch;
                        }
                    } else {
                        const stream = exStreams[k];
                        let ptr = exPtrs[k];
                        if (!stream || ptr + exCount > exSizes[k]) throw new Error("fpf ex");
                        for (let e = 0; e < exCount; e++) {
                            const pos = metaView[bytePos++];
                            if (pos >= blockSize) throw new Error("fpf pos");
                            outI32[blockOutPos + pos] |= stream[ptr++] << bitWidth;
                        }
                        exPtrs[k] = ptr;
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
            bitOffset += 8;
            if (bitOffset === 32) {
                bitOffset = 0;
                wordIdx++;
            }
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

    if (fpfBytes.length > 262144) fpfBytes = new Uint8Array(16384);
    return out;
}
