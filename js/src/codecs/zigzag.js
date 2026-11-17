// In-place 32-bit and 64-bit ZigZag / Delta / Componentwise-Delta transforms.
// Operating in-place over the physical word buffer eliminates a 2nd allocation per stream
// and keeps working sets hot in L1 cache.

/**
 * In-place ZigZag decode over `u32` words; returns an `Int32Array` sharing the same buffer.
 * @param {Uint32Array} buf
 * @returns {Int32Array}
 */
export function zigzagI32InPlace(buf) {
  const out = new Int32Array(buf.buffer, buf.byteOffset, buf.length);
  for (let i = 0; i < buf.length; i++) {
    const u = buf[i];
    out[i] = (u >>> 1) ^ -(u & 1);
  }
  return out;
}

/**
 * In-place ZigZag + prefix-sum delta over 32-bit words (`buf` modified in place).
 * @param {Uint32Array} buf
 * @returns {Uint32Array}
 */
export function zigzagDelta32InPlace(buf) {
  let s = 0;
  for (let i = 0; i < buf.length; i++) {
    const u = buf[i];
    s = (s + ((u >>> 1) ^ -(u & 1))) | 0;
    buf[i] = s >>> 0;
  }
  return buf;
}

/**
 * In-place ZigZag + second-order delta over 32-bit words (`buf` modified in place).
 * @param {Uint32Array} buf
 * @returns {Uint32Array}
 */
export function zigzagDelta2_32InPlace(buf) {
  let v = 0;
  let d = 0;
  for (let i = 0; i < buf.length; i++) {
    const u = buf[i];
    d = (d + ((u >>> 1) ^ -(u & 1))) | 0;
    v = (v + d) | 0;
    buf[i] = v >>> 0;
  }
  return buf;
}

/**
 * In-place componentwise ZigZag + delta over interleaved `(x, y)` pairs.
 * @param {Uint32Array} buf
 * @returns {Int32Array}
 */
export function cwDeltaVec2InPlace(buf) {
  const n = buf.length;
  if ((n & 1) !== 0) throw new Error("vec2 len");
  const out = new Int32Array(buf.buffer, buf.byteOffset, n);
  let x = 0;
  let y = 0;
  for (let i = 0; i < n; i += 2) {
    const ux = buf[i];
    const uy = buf[i + 1];
    x = (x + ((ux >>> 1) ^ -(ux & 1))) | 0;
    y = (y + ((uy >>> 1) ^ -(uy & 1))) | 0;
    out[i] = x;
    out[i + 1] = y;
  }
  return out;
}

/**
 * In-place componentwise ZigZag + second-order delta over interleaved `(x, y)` pairs.
 * @param {Uint32Array} buf
 * @returns {Int32Array}
 */
export function cwDelta2Vec2InPlace(buf) {
  const n = buf.length;
  if ((n & 1) !== 0) throw new Error("vec2 len");
  const out = new Int32Array(buf.buffer, buf.byteOffset, n);
  let x = 0;
  let y = 0;
  let dx = 0;
  let dy = 0;
  for (let i = 0; i < n; i += 2) {
    const ux = buf[i];
    const uy = buf[i + 1];
    dx = (dx + ((ux >>> 1) ^ -(ux & 1))) | 0;
    dy = (dy + ((uy >>> 1) ^ -(uy & 1))) | 0;
    x = (x + dx) | 0;
    y = (y + dy) | 0;
    out[i] = x;
    out[i + 1] = y;
  }
  return out;
}

/**
 * In-place componentwise ZigZag + delta over interleaved `(x, y, z)` triples.
 * @param {Uint32Array} buf
 * @returns {Int32Array}
 */
export function cwDeltaVec3InPlace(buf) {
  const n = buf.length;
  if (n % 3 !== 0) throw new Error("vec3 len");
  const out = new Int32Array(buf.buffer, buf.byteOffset, n);
  let x = 0;
  let y = 0;
  let z = 0;
  for (let i = 0; i < n; i += 3) {
    const ux = buf[i];
    const uy = buf[i + 1];
    const uz = buf[i + 2];
    x = (x + ((ux >>> 1) ^ -(ux & 1))) | 0;
    y = (y + ((uy >>> 1) ^ -(uy & 1))) | 0;
    z = (z + ((uz >>> 1) ^ -(uz & 1))) | 0;
    out[i] = x;
    out[i + 1] = y;
    out[i + 2] = z;
  }
  return out;
}

/**
 * View a `Uint32Array` slice as an `Int32Array` over the exact same memory.
 * @param {Uint32Array} buf
 * @returns {Int32Array}
 */
export const asI32 = (buf) =>
  new Int32Array(buf.buffer, buf.byteOffset, buf.length);

export function zigzagI64InPlace(buf) {
  const out = new BigInt64Array(buf.buffer, buf.byteOffset, buf.length);
  for (let i = 0; i < buf.length; i++) {
    const u = buf[i];
    out[i] = (u >> 1n) ^ -(u & 1n);
  }
  return out;
}

export function zigzagDelta64InPlace(buf) {
  let s = 0n;
  for (let i = 0; i < buf.length; i++) {
    const u = buf[i];
    s = BigInt.asIntN(64, s + ((u >> 1n) ^ -(u & 1n)));
    buf[i] = BigInt.asUintN(64, s);
  }
  return buf;
}

export function zigzagDelta2_64InPlace(buf) {
  let v = 0n;
  let d = 0n;
  for (let i = 0; i < buf.length; i++) {
    const u = buf[i];
    d = BigInt.asIntN(64, d + ((u >> 1n) ^ -(u & 1n)));
    v = BigInt.asIntN(64, v + d);
    buf[i] = BigInt.asUintN(64, v);
  }
  return buf;
}

export const asI64 = (buf) =>
  new BigInt64Array(buf.buffer, buf.byteOffset, buf.length);
