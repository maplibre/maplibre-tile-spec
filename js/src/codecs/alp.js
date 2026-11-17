const POW10 = [
  1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14,
  1e15, 1e16, 1e17, 1e18,
];
const FRAC10 = [
  1e0, 1e-1, 1e-2, 1e-3, 1e-4, 1e-5, 1e-6, 1e-7, 1e-8, 1e-9, 1e-10, 1e-11, 1e-12,
  1e-13, 1e-14, 1e-15, 1e-16, 1e-17, 1e-18,
];

export function parseAlpScale(byte) {
  for (let e = 0; e <= 18; e++) {
    const f = byte - ((e * (e + 1)) >> 1);
    if (f >= 0 && f <= e) return (e << 8) | f;
  }
  throw new Error("alp scale");
}

export function decodeAlpFloats(offsets, base, scalePacked, isF32) {
  const down = POW10[scalePacked & 0xff];
  const upRecip = FRAC10[scalePacked >>> 8];
  const out = isF32
    ? new Float32Array(offsets.length)
    : new Float64Array(offsets.length);

  if (
    offsets instanceof Uint32Array &&
    base >= -9007199254740991n &&
    base <= 9007199254740991n - 4294967295n
  ) {
    // Fast pure-Number path when base + u32 offset stays strictly inside safe f64 integer range
    const baseNum = Number(base);
    for (let i = 0; i < offsets.length; i++) {
      out[i] = (baseNum + offsets[i]) * down * upRecip;
    }
    return out;
  }

  const baseU64 = BigInt.asUintN(64, base);
  const isU32 = offsets instanceof Uint32Array;
  for (let i = 0; i < offsets.length; i++) {
    const off = isU32 ? BigInt(offsets[i]) : offsets[i];
    out[i] = Number(BigInt.asIntN(64, baseU64 + off)) * down * upRecip;
  }
  return out;
}
