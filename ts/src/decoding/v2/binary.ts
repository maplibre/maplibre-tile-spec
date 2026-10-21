const utf8 = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true });
export const IS_LE = new Uint8Array(new Uint16Array([1]).buffer)[0] === 1;
export const DEFAULT_MAX_BYTES = 67108864;
export const EMPTY_U32 = new Uint32Array(0);
export const EMPTY_I32 = new Int32Array(0);

let budgetLeft = DEFAULT_MAX_BYTES;

export interface ByteReader {
    b: Uint8Array;
    p: number;
    e: number;
}

export function resetBudget(maxBytes = DEFAULT_MAX_BYTES): void {
    if (maxBytes !== Number.POSITIVE_INFINITY && (!Number.isFinite(maxBytes) || maxBytes < 0)) {
        throw new RangeError("maxBytes must be a non-negative number or Infinity");
    }
    budgetLeft = maxBytes;
}

export function checkScratchBytes(bytes: number): void {
    if (!Number.isFinite(bytes) || bytes < 0 || bytes > budgetLeft) {
        throw new RangeError("memory limit exceeded");
    }
}

export function chargeBytes(bytes: number): void {
    checkScratchBytes(bytes);
    budgetLeft -= bytes;
}

export function allocTyped<T extends ArrayBufferView>(
    Ctor: { new (n: number): T; BYTES_PER_ELEMENT: number },
    n: number,
): T {
    chargeBytes(n * Ctor.BYTES_PER_ELEMENT);
    return new Ctor(n);
}

export const allocU8 = (n: number): Uint8Array => allocTyped(Uint8Array, n);
export const allocI8 = (n: number): Int8Array => allocTyped(Int8Array, n);
export const allocU32 = (n: number): Uint32Array => allocTyped(Uint32Array, n);
export const allocI32 = (n: number): Int32Array => allocTyped(Int32Array, n);
export const allocF32 = (n: number): Float32Array => allocTyped(Float32Array, n);
export const allocF64 = (n: number): Float64Array => allocTyped(Float64Array, n);
export const allocU64 = (n: number): BigUint64Array => allocTyped(BigUint64Array, n);
export const allocI64 = (n: number): BigInt64Array => allocTyped(BigInt64Array, n);

export function allocArray<T>(n: number, bytesPerSlot = 8): T[] {
    chargeBytes(n * bytesPerSlot);
    return new Array<T>(n);
}

export function scratchU32(n: number): Uint32Array {
    checkScratchBytes(n * 4);
    return new Uint32Array(n);
}

export function scratchU64(n: number): BigUint64Array {
    checkScratchBytes(n * 8);
    return new BigUint64Array(n);
}

export const cursor = (b: Uint8Array, p = 0, e = b.length): ByteReader => ({ b, p, e });

export function peekU8(r: ByteReader): number {
    if (r.p >= r.e) throw new RangeError("eof");
    return r.b[r.p];
}

export function readU8(r: ByteReader): number {
    if (r.p >= r.e) throw new RangeError("eof");
    return r.b[r.p++];
}

export function readBytes(r: ByteReader, n: number): Uint8Array {
    const q = r.p + n;
    if (!Number.isInteger(n) || n < 0 || q > r.e) throw new RangeError("eof");
    const out = r.b.subarray(r.p, q);
    r.p = q;
    return out;
}

export function subCursor(r: ByteReader, n: number): ByteReader {
    const p = r.p;
    readBytes(r, n);
    return { b: r.b, p, e: r.p };
}

export const decodeUtf8 = (bytes: Uint8Array): string => (bytes.length === 0 ? "" : utf8.decode(bytes));
export const readStr = (r: ByteReader): string => decodeUtf8(readBytes(r, readVarintU32(r)));

export function readVarintU32(r: ByteReader): number {
    let v = 0;
    let s = 0;
    while (true) {
        const c = readU8(r);
        if (s === 28 && c >= 16) throw new RangeError("overflow");
        v |= (c & 127) << s;
        if (c < 128) {
            if (s > 0 && c === 0) throw new Error("varint");
            return v >>> 0;
        }
        s += 7;
    }
}

export function readVarintU64(r: ByteReader): bigint {
    let v = 0n;
    let s = 0n;
    while (true) {
        const c = readU8(r);
        if (s === 63n && c > 1) throw new RangeError("overflow");
        v |= BigInt(c & 127) << s;
        if (c < 128) {
            if (s > 0n && c === 0) throw new Error("varint");
            return v;
        }
        s += 7n;
    }
}

export function readVarintI64(r: ByteReader): bigint {
    const u = readVarintU64(r);
    return (u >> 1n) ^ -(u & 1n);
}

export function decodeVarintU32Into(bytes: Uint8Array, count: number, out: Uint32Array): Uint32Array {
    const r = cursor(bytes);
    for (let i = 0; i < count; i++) out[i] = readVarintU32(r);
    return out;
}

export function decodeVarintU32sAll(bytes: Uint8Array): Uint32Array {
    const out = scratchU32(bytes.length);
    const r = cursor(bytes);
    let n = 0;
    while (r.p < r.e) out[n++] = readVarintU32(r);
    return out.subarray(0, n);
}

export function decodeVarintU64Into(bytes: Uint8Array, count: number, out: BigUint64Array): BigUint64Array {
    const r = cursor(bytes);
    for (let i = 0; i < count; i++) out[i] = readVarintU64(r);
    return out;
}

export function decodeVarintU64sAll(bytes: Uint8Array): BigUint64Array {
    const out = scratchU64(bytes.length);
    const r = cursor(bytes);
    let n = 0;
    while (r.p < r.e) out[n++] = readVarintU64(r);
    return out.subarray(0, n);
}

export function copyLittleEndian<T extends ArrayBufferView & { set(src: T): void }>(
    Ctor: { new (buf: ArrayBufferLike, byteOffset?: number, length?: number): T; BYTES_PER_ELEMENT: number },
    bytes: Uint8Array,
    count: number,
    out: T,
): T {
    const bpe = Ctor.BYTES_PER_ELEMENT;
    if (bytes.length !== count * bpe) throw new Error("raw size");
    if (count === 0) return out;
    if (IS_LE && bytes.byteOffset % bpe === 0) {
        out.set(new Ctor(bytes.buffer, bytes.byteOffset, count));
        return out;
    }
    const copy = new Uint8Array(count * bpe);
    if (IS_LE) {
        copy.set(bytes);
    } else {
        for (let i = 0; i < count; i++) {
            for (let b = 0; b < bpe; b++) copy[i * bpe + b] = bytes[i * bpe + (bpe - 1 - b)];
        }
    }
    out.set(new Ctor(copy.buffer, 0, count));
    return out;
}

export function unpackBitpackU32Into(data: Uint8Array, count: number, out: Uint32Array): Uint32Array {
    if (data.length === 0) throw new Error("bitpack");
    const width = data[0];
    if (width < 1 || width > 32 || data.length - 1 !== Math.ceil((count * width) / 8)) {
        throw new Error("bitpack");
    }
    if (width <= 24) {
        const mask = (1 << width) - 1;
        let acc = 0;
        let bits = 0;
        let p = 1;
        for (let i = 0; i < count; i++) {
            while (bits < width) {
                acc |= data[p++] << bits;
                bits += 8;
            }
            out[i] = acc & mask;
            acc >>>= width;
            bits -= width;
        }
        return out;
    }
    const mask = (1n << BigInt(width)) - 1n;
    const wBig = BigInt(width);
    let acc = 0n;
    let bits = 0;
    let p = 1;
    for (let i = 0; i < count; i++) {
        while (bits < width) {
            acc |= BigInt(data[p++]) << BigInt(bits);
            bits += 8;
        }
        out[i] = Number(acc & mask) >>> 0;
        acc >>= wBig;
        bits -= width;
    }
    return out;
}

export function unpackBitpackU64Into(data: Uint8Array, count: number, out: BigUint64Array): BigUint64Array {
    const tmp = unpackBitpackU32Into(data, count, scratchU32(count));
    for (let i = 0; i < count; i++) out[i] = BigInt(tmp[i]);
    return out;
}

export function decodeRleInterleavedU32(data: Uint32Array, total: number, out: Uint32Array): Uint32Array {
    if ((data.length & 1) !== 0) throw new Error("rle odd");
    let pos = 0;
    for (let i = 0; i < data.length; i += 2) {
        const len = data[i];
        if (len > total - pos) throw new Error("rle overflow");
        out.fill(data[i + 1], pos, pos + len);
        pos += len;
    }
    if (pos !== total) throw new Error("rle sum");
    return out;
}

export function decodeRleInterleavedU64(data: BigUint64Array, total: number, out: BigUint64Array): BigUint64Array {
    if ((data.length & 1) !== 0) throw new Error("rle odd");
    let pos = 0;
    const maxRem = BigInt(total);
    for (let i = 0; i < data.length; i += 2) {
        if (data[i] > maxRem - BigInt(pos)) throw new Error("rle overflow");
        const len = Number(data[i]);
        out.fill(data[i + 1], pos, pos + len);
        pos += len;
    }
    if (pos !== total) throw new Error("rle sum");
    return out;
}

export const asI32 = (buf: Uint32Array | Int32Array): Int32Array =>
    new Int32Array(buf.buffer, buf.byteOffset, buf.length);
export const asI64 = (buf: BigUint64Array): BigInt64Array => new BigInt64Array(buf.buffer, buf.byteOffset, buf.length);

export function zigzagI32InPlace(buf: Uint32Array): Int32Array {
    const out = asI32(buf);
    for (let i = 0; i < out.length; i++) out[i] = (out[i] >>> 1) ^ -(out[i] & 1);
    return out;
}

export function zigzagDelta32InPlace(buf: Uint32Array, order = 1): Uint32Array {
    const i32 = asI32(buf);
    let v = 0;
    let d = 0;
    for (let i = 0; i < i32.length; i++) {
        const z = (i32[i] >>> 1) ^ -(i32[i] & 1);
        d = order === 2 ? (d + z) | 0 : z;
        v = (v + d) | 0;
        i32[i] = v;
    }
    return buf;
}

export function cwDeltaVecInPlace(buf: Uint32Array, dim: 2 | 3, order = 1): Int32Array {
    if (buf.length % dim !== 0) throw new Error("vec len");
    const out = asI32(buf);
    const v = [0, 0, 0];
    const d = [0, 0, 0];
    for (let i = 0; i < out.length; i += dim) {
        for (let c = 0; c < dim; c++) {
            const z = (out[i + c] >>> 1) ^ -(out[i + c] & 1);
            d[c] = order === 2 ? (d[c] + z) | 0 : z;
            v[c] = (v[c] + d[c]) | 0;
            out[i + c] = v[c];
        }
    }
    return out;
}

export function zigzagI64InPlace(buf: BigUint64Array): BigInt64Array {
    const out = asI64(buf);
    for (let i = 0; i < buf.length; i++) out[i] = (buf[i] >> 1n) ^ -(buf[i] & 1n);
    return out;
}

export function zigzagDelta64InPlace(buf: BigUint64Array, order = 1): BigUint64Array {
    let v = 0n;
    let d = 0n;
    for (let i = 0; i < buf.length; i++) {
        const z = (buf[i] >> 1n) ^ -(buf[i] & 1n);
        d = order === 2 ? BigInt.asIntN(64, d + z) : z;
        v = BigInt.asIntN(64, v + d);
        buf[i] = BigInt.asUintN(64, v);
    }
    return buf;
}

const POW10: readonly number[] = Array.from({ length: 23 }, (_, i) => 10 ** i);
const ALP_MAX_SAFE = BigInt(Number.MAX_SAFE_INTEGER);
const ALP_MIN_SAFE = BigInt(Number.MIN_SAFE_INTEGER);

export function parseAlpScale(code: number): number {
    if (code >= 190) throw new Error("alp scale");
    let e = 0;
    while (((e + 1) * (e + 2)) / 2 <= code) e++;
    return (e << 8) | (code - (e * (e + 1)) / 2);
}

export function decodeAlpFloats(
    offsets: Uint32Array | BigUint64Array,
    base: bigint,
    scalePack: number,
    isF32: boolean,
): Float32Array | Float64Array {
    const n = offsets.length;
    const out = isF32 ? allocF32(n) : allocF64(n);
    const factor = POW10[scalePack & 0xff] / POW10[scalePack >>> 8];
    const baseNum = Number(base);
    if (offsets instanceof Uint32Array && base >= ALP_MIN_SAFE && base + 0xffffffffn <= ALP_MAX_SAFE) {
        for (let i = 0; i < n; i++) out[i] = (baseNum + offsets[i]) * factor;
        return out;
    }
    for (let i = 0; i < n; i++) {
        const digits = BigInt.asIntN(64, base + BigInt(offsets[i]));
        if (digits < ALP_MIN_SAFE || digits > ALP_MAX_SAFE) throw new Error("alp range");
        out[i] = Number(digits) * factor;
    }
    return out;
}

const COMPACT_16 = new Uint8Array(65536);
for (let i = 0; i < 65536; i++) {
    let v = i & 0x5555;
    v = (v | (v >>> 1)) & 0x3333;
    v = (v | (v >>> 2)) & 0x0f0f;
    COMPACT_16[i] = (v | (v >>> 4)) & 0x00ff;
}

export function deinterleaveU64(word: bigint): [number, number] {
    let x = 0;
    let y = 0;
    for (let s = 0n; s < 64n; s += 16n) {
        const chunk = Number((word >> s) & 0xffffn);
        const shift = Number(s >> 1n);
        x |= COMPACT_16[chunk] << shift;
        y |= COMPACT_16[chunk >>> 1] << shift;
    }
    return [x >>> 0, y >>> 0];
}

export function decodeMortonDelta(deltas: Uint32Array, numBits: number, shift: number): Int32Array {
    const mask = numBits === 0 ? 0 : (1 << numBits) - 1;
    const out = allocI32(deltas.length * 2);
    let prev = 0;
    for (let i = 0; i < deltas.length; i++) {
        prev = (prev + (deltas[i] | 0)) | 0;
        const m = prev >>> 0;
        out[i * 2] = (((COMPACT_16[m & 0xffff] | (COMPACT_16[m >>> 16] << 8)) & mask) - shift) | 0;
        const my = m >>> 1;
        out[i * 2 + 1] = (((COMPACT_16[my & 0xffff] | (COMPACT_16[my >>> 16] << 8)) & mask) - shift) | 0;
    }
    return out;
}
