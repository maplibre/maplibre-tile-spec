const utf8 = new TextDecoder("utf-8", { fatal: true });

export const IS_LE = new Uint8Array(new Uint16Array([1]).buffer)[0] === 1;

const ARENA_BASE_U32 = 16384; // 64 KB initial
const ARENA_MAX_KEEP_U32 = 262144; // 1 MB max retained
let arenaU32 = new Uint32Array(ARENA_BASE_U32);
let arenaTop = 0;
let scratchU64 = new BigUint64Array(1024);

export const EMPTY_U32 = new Uint32Array(0);
export const EMPTY_I32 = new Int32Array(0);

export function resetArena() {
  arenaTop = 0;
}

export function trimArena() {
  arenaTop = 0;
  if (arenaU32.length > ARENA_MAX_KEEP_U32) {
    arenaU32 = new Uint32Array(ARENA_BASE_U32);
  }
  if (scratchU64.length > 16384) {
    scratchU64 = new BigUint64Array(1024);
  }
}

export function takeU32(n) {
  if (n === 0) return EMPTY_U32;
  const end = arenaTop + n;
  if (end <= arenaU32.length) {
    const view = arenaU32.subarray(arenaTop, end);
    arenaTop = end;
    return view;
  }
  let cap = arenaU32.length * 2;
  while (cap < n) cap *= 2;
  arenaU32 = new Uint32Array(cap);
  arenaTop = n;
  return arenaU32.subarray(0, n);
}

export function takeU64(n) {
  if (n > scratchU64.length) {
    let cap = scratchU64.length * 2;
    while (cap < n) cap *= 2;
    scratchU64 = new BigUint64Array(cap);
  }
  return scratchU64.subarray(0, n);
}

export const cursor = (b, p = 0, e = b.length) => ({ b, p, e });

export function peekU8(r) {
  if (r.p >= r.e) throw new RangeError("eof");
  return r.b[r.p];
}

export function readU8(r) {
  if (r.p >= r.e) throw new RangeError("eof");
  return r.b[r.p++];
}

export function readBytes(r, n) {
  const p = r.p;
  const q = p + n;
  if (!Number.isInteger(n) || n < 0 || q > r.e) throw new RangeError("eof");
  r.p = q;
  return r.b.subarray(p, q);
}

export function subCursor(r, n) {
  const p = r.p;
  const q = p + n;
  if (!Number.isInteger(n) || n < 0 || q > r.e) throw new RangeError("eof");
  r.p = q;
  return { b: r.b, p, e: q };
}

export function decodeUtf8(bytes) {
  return bytes.length === 0 ? "" : utf8.decode(bytes);
}

export function readStr(r) {
  const n = readVarintU32(r);
  if (n === 0) return "";
  const p = r.p;
  const q = p + n;
  if (q > r.e) throw new RangeError("eof");
  r.p = q;
  const b = r.b;
  if (n <= 16) {
    let or = 0;
    for (let i = p; i < q; i++) or |= b[i];
    if (or < 128) {
      if (n === 1) return String.fromCharCode(b[p]);
      if (n === 2) return String.fromCharCode(b[p], b[p + 1]);
      if (n === 3) return String.fromCharCode(b[p], b[p + 1], b[p + 2]);
      if (n === 4) return String.fromCharCode(b[p], b[p + 1], b[p + 2], b[p + 3]);
      return String.fromCharCode.apply(null, b.subarray(p, q));
    }
  }
  return utf8.decode(b.subarray(p, q));
}

export function readVarintU32(r) {
  const b = r.b;
  const e = r.e;
  let p = r.p;
  if (p >= e) throw new RangeError("eof");
  let c = b[p++];
  if (c < 128) {
    r.p = p;
    return c;
  }
  let v = c & 127;
  if (p >= e) throw new RangeError("eof");
  c = b[p++];
  if (c < 128) {
    if (c === 0) throw new Error("varint");
    r.p = p;
    return v | (c << 7);
  }
  v |= (c & 127) << 7;
  let s = 14;
  while (true) {
    if (p >= e) throw new RangeError("eof");
    c = b[p++];
    if (s === 28 && c >= 16) throw new RangeError("overflow");
    v |= (c & 127) << s;
    if (c < 128) {
      if (c === 0) throw new Error("varint");
      r.p = p;
      return v >>> 0;
    }
    s += 7;
  }
}

export function readVarintU64(r) {
  const b = r.b;
  const e = r.e;
  let p = r.p;
  if (p >= e) throw new RangeError("eof");
  let c = b[p++];
  if (c < 128) {
    r.p = p;
    return BigInt(c);
  }
  let v = BigInt(c & 127);
  let s = 7n;
  while (true) {
    if (p >= e) throw new RangeError("eof");
    c = b[p++];
    if (s === 63n && c > 1) throw new RangeError("overflow");
    v |= BigInt(c & 127) << s;
    if (c < 128) {
      if (c === 0) throw new Error("varint");
      r.p = p;
      return v;
    }
    s += 7n;
  }
}

export function readVarintI64(r) {
  const u = readVarintU64(r);
  return (u >> 1n) ^ -(u & 1n);
}

export function decodeVarintU32Into(bytes, count, out) {
  const outI32 = new Int32Array(out.buffer, out.byteOffset, count);
  const e = bytes.length | 0;
  const safe4 = (e - 20) | 0;
  const count4 = (count - 3) | 0;
  let p = 0;
  let i = 0;

  // 4-at-a-time single-byte fast path (`< 128` covers ~85% of MLT varints)
  while (i < count4 && p <= safe4) {
    const b0 = bytes[p];
    const b1 = bytes[(p + 1) | 0];
    const b2 = bytes[(p + 2) | 0];
    const b3 = bytes[(p + 3) | 0];
    if ((b0 | b1 | b2 | b3) < 128) {
      outI32[i] = b0;
      outI32[(i + 1) | 0] = b1;
      outI32[(i + 2) | 0] = b2;
      outI32[(i + 3) | 0] = b3;
      p = (p + 4) | 0;
      i = (i + 4) | 0;
      continue;
    }
    let c = b0;
    p = (p + 1) | 0;
    if (c < 128) {
      outI32[i++] = c;
      continue;
    }
    let v = c & 127;
    c = bytes[p++];
    if (c < 128) {
      if (c === 0) throw new Error("varint");
      outI32[i++] = v | (c << 7);
      continue;
    }
    v |= (c & 127) << 7;
    c = bytes[p++];
    if (c < 128) {
      if (c === 0) throw new Error("varint");
      outI32[i++] = v | (c << 14);
      continue;
    }
    v |= (c & 127) << 14;
    c = bytes[p++];
    if (c < 128) {
      if (c === 0) throw new Error("varint");
      outI32[i++] = v | (c << 21);
      continue;
    }
    v |= (c & 127) << 21;
    c = bytes[p++];
    if (c >= 16 || c === 0) throw new RangeError(c === 0 ? "varint" : "overflow");
    outI32[i++] = v | (c << 28);
  }

  const r = { b: bytes, p, e };
  for (; i < count; i++) outI32[i] = readVarintU32(r);
  return out;
}

export function decodeVarintU32sAllScratch(bytes) {
  const e = bytes.length;
  if (e === 0) return EMPTY_U32;
  const out = takeU32(e);
  const r = { b: bytes, p: 0, e };
  let n = 0;
  while (r.p < e) out[n++] = readVarintU32(r);
  return out.subarray(0, n);
}

export function decodeVarintU64Into(bytes, count, out) {
  const r = { b: bytes, p: 0, e: bytes.length };
  for (let i = 0; i < count; i++) out[i] = readVarintU64(r);
  return out;
}

export function decodeVarintU64sAllScratch(bytes) {
  if (bytes.length === 0) return new BigUint64Array(0);
  const out = takeU64(bytes.length);
  const r = { b: bytes, p: 0, e: bytes.length };
  let n = 0;
  while (r.p < r.e) out[n++] = readVarintU64(r);
  return out.subarray(0, n);
}

function copyLeTyped(bytes, count, width, TypedCtor, out) {
  if (bytes.length !== count * width) throw new Error("raw size");
  if (count === 0) return out;
  if (IS_LE && (bytes.byteOffset & (width - 1)) === 0) {
    out.set(new TypedCtor(bytes.buffer, bytes.byteOffset, count));
    return out;
  }
  const v = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  if (TypedCtor === Uint32Array) {
    for (let i = 0; i < count; i++) out[i] = v.getUint32(i * 4, true);
  } else if (TypedCtor === Float32Array) {
    for (let i = 0; i < count; i++) out[i] = v.getFloat32(i * 4, true);
  } else if (TypedCtor === Float64Array) {
    for (let i = 0; i < count; i++) out[i] = v.getFloat64(i * 8, true);
  } else {
    for (let i = 0; i < count; i++) out[i] = v.getBigUint64(i * 8, true);
  }
  return out;
}

export const decodeRawU32Into = (bytes, count, out) =>
  copyLeTyped(bytes, count, 4, Uint32Array, out);

export const decodeRawU64Into = (bytes, count, out) =>
  copyLeTyped(bytes, count, 8, BigUint64Array, out);

export const decodeRawFloats = (bytes, count, isF32) =>
  isF32
    ? copyLeTyped(bytes, count, 4, Float32Array, new Float32Array(count))
    : copyLeTyped(bytes, count, 8, Float64Array, new Float64Array(count));
