import { allocU8 } from "./binary";

export function decodeRleSplitU32Into(
    data: Uint32Array,
    runs: number,
    numRleValues: number,
    out: Uint32Array,
): Uint32Array {
    if (data.length !== runs * 2) throw new Error("rle len");
    let pos = 0;
    for (let i = 0; i < runs; i++) {
        const len = data[i];
        if (len > numRleValues - pos) throw new Error("rle sum");
        out.fill(data[runs + i], pos, pos + len);
        pos += len;
    }
    if (pos !== numRleValues) throw new Error("rle sum");
    return out;
}

export function decodeRleSplitU64Into(
    data: BigUint64Array,
    runs: number,
    numRleValues: number,
    out: BigUint64Array,
): BigUint64Array {
    if (data.length !== runs * 2) throw new Error("rle len");
    let pos = 0;
    const maxRem = BigInt(numRleValues);
    for (let i = 0; i < runs; i++) {
        const lenBig = data[i];
        if (lenBig > maxRem - BigInt(pos)) throw new Error("rle sum");
        const len = Number(lenBig);
        out.fill(data[runs + i], pos, pos + len);
        pos += len;
    }
    if (pos !== numRleValues) throw new Error("rle sum");
    return out;
}

export function decodeRleInterleavedU32Into(data: Uint32Array, numRleValues: number, out: Uint32Array): Uint32Array {
    if ((data.length & 1) !== 0) throw new Error("rle odd");
    let pos = 0;
    for (let i = 0; i < data.length; i += 2) {
        const len = data[i];
        if (len > numRleValues - pos) throw new Error("rle overflow");
        out.fill(data[i + 1], pos, pos + len);
        pos += len;
    }
    if (pos !== numRleValues) throw new Error("rle sum");
    return out;
}

export function decodeRleInterleavedU64Into(
    data: BigUint64Array,
    numRleValues: number,
    out: BigUint64Array,
): BigUint64Array {
    if ((data.length & 1) !== 0) throw new Error("rle odd");
    let pos = 0;
    const maxRem = BigInt(numRleValues);
    for (let i = 0; i < data.length; i += 2) {
        const lenBig = data[i];
        if (lenBig > maxRem - BigInt(pos)) throw new Error("rle overflow");
        const len = Number(lenBig);
        out.fill(data[i + 1], pos, pos + len);
        pos += len;
    }
    if (pos !== numRleValues) throw new Error("rle sum");
    return out;
}

export function decodeByteRle(input: Uint8Array, numBytes: number): Uint8Array {
    const out = allocU8(numBytes);
    let outPos = 0;
    let inPos = 0;
    const inLen = input.length;
    while (outPos < numBytes && inPos < inLen) {
        const ctrl = input[inPos++];
        if (ctrl >= 128) {
            const count = 256 - ctrl;
            if (inPos + count > inLen || outPos + count > numBytes) {
                throw new RangeError("byte-rle");
            }
            out.set(input.subarray(inPos, inPos + count), outPos);
            inPos += count;
            outPos += count;
        } else {
            const count = ctrl + 3;
            if (inPos >= inLen || outPos + count > numBytes) {
                throw new RangeError("byte-rle");
            }
            out.fill(input[inPos++], outPos, outPos + count);
            outPos += count;
        }
    }
    if (outPos !== numBytes) throw new Error("byte-rle");
    return out;
}
