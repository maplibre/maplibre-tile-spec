// Single-TypedArray (`Int32Array`) in-place ZigZag / Delta / Componentwise-Delta transforms.
// Reading AND writing through one `Int32Array` view avoids aliased `Uint32Array`+`Int32Array`
// deoptimizations and keeps all values in `Signed32` registers (`u >>> 1` is unsigned even on `Int32`!).

export const asI32 = (buf) =>
  new Int32Array(buf.buffer, buf.byteOffset, buf.length);

export function zigzagI32InPlace(buf) {
  const out = asI32(buf);
  const n = out.length | 0;
  for (let i = 0; i < n; i = (i + 1) | 0) {
    const u = out[i];
    out[i] = (u >>> 1) ^ -(u & 1);
  }
  return out;
}

export function zigzagDelta32InPlace(buf) {
  const i32 = asI32(buf);
  const n = i32.length | 0;
  let s = 0;
  for (let i = 0; i < n; i = (i + 1) | 0) {
    const u = i32[i];
    s = (s + ((u >>> 1) ^ -(u & 1))) | 0;
    i32[i] = s;
  }
  return buf;
}

export function zigzagDelta2_32InPlace(buf) {
  const i32 = asI32(buf);
  const n = i32.length | 0;
  let v = 0;
  let d = 0;
  for (let i = 0; i < n; i = (i + 1) | 0) {
    const u = i32[i];
    d = (d + ((u >>> 1) ^ -(u & 1))) | 0;
    v = (v + d) | 0;
    i32[i] = v;
  }
  return buf;
}

export function cwDeltaVec2InPlace(buf) {
  const n = buf.length | 0;
  if ((n & 1) !== 0) throw new Error("vec2 len");
  const out = asI32(buf);
  let x = 0;
  let y = 0;
  for (let i = 0; i < n; i = (i + 2) | 0) {
    const ux = out[i];
    const uy = out[(i + 1) | 0];
    x = (x + ((ux >>> 1) ^ -(ux & 1))) | 0;
    y = (y + ((uy >>> 1) ^ -(uy & 1))) | 0;
    out[i] = x;
    out[(i + 1) | 0] = y;
  }
  return out;
}

export function cwDelta2Vec2InPlace(buf) {
  const n = buf.length | 0;
  if ((n & 1) !== 0) throw new Error("vec2 len");
  const out = asI32(buf);
  let x = 0;
  let y = 0;
  let dx = 0;
  let dy = 0;
  for (let i = 0; i < n; i = (i + 2) | 0) {
    const ux = out[i];
    const uy = out[(i + 1) | 0];
    dx = (dx + ((ux >>> 1) ^ -(ux & 1))) | 0;
    dy = (dy + ((uy >>> 1) ^ -(uy & 1))) | 0;
    x = (x + dx) | 0;
    y = (y + dy) | 0;
    out[i] = x;
    out[(i + 1) | 0] = y;
  }
  return out;
}

export function cwDeltaVec3InPlace(buf) {
  const n = buf.length | 0;
  if (n % 3 !== 0) throw new Error("vec3 len");
  const out = asI32(buf);
  let x = 0;
  let y = 0;
  let z = 0;
  for (let i = 0; i < n; i = (i + 3) | 0) {
    const ux = out[i];
    const uy = out[(i + 1) | 0];
    const uz = out[(i + 2) | 0];
    x = (x + ((ux >>> 1) ^ -(ux & 1))) | 0;
    y = (y + ((uy >>> 1) ^ -(uy & 1))) | 0;
    z = (z + ((uz >>> 1) ^ -(uz & 1))) | 0;
    out[i] = x;
    out[(i + 1) | 0] = y;
    out[(i + 2) | 0] = z;
  }
  return out;
}

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
