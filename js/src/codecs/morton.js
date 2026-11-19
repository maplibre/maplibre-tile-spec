import { allocI32 } from "./binary.js";

export function deinterleaveU64(code) {
  return [compactU64(code), compactU64(code >> 1n)];
}

function compactU64(code) {
  let c = code & 0x5555_5555_5555_5555n;
  c = (c | (c >> 1n)) & 0x3333_3333_3333_3333n;
  c = (c | (c >> 2n)) & 0x0f0f_0f0f_0f0f_0f0fn;
  c = (c | (c >> 4n)) & 0x00ff_00ff_00ff_00ffn;
  c = (c | (c >> 8n)) & 0x0000_ffff_0000_ffffn;
  c = (c | (c >> 16n)) & 0x0000_0000_ffff_ffffn;
  return Number(c) >>> 0;
}

/**
 * O(1) bit-compacting Morton 16-bit deinterleave (replaces O(bits) inner bit loop!).
 */
function compact16(x) {
  x &= 0x55555555;
  x = (x | (x >>> 1)) & 0x33333333;
  x = (x | (x >>> 2)) & 0x0f0f0f0f;
  x = (x | (x >>> 4)) & 0x00ff00ff;
  return (x | (x >>> 8)) & 0x0000ffff;
}

export function decodeMortonDelta(data, bits, shift) {
  if (bits > 16) throw new Error("morton bits");
  const mask = bits === 0 ? 0 : (1 << bits) - 1;
  const n = data.length;
  const out = allocI32(n * 2);
  let prev = 0;
  for (let i = 0; i < n; i++) {
    prev = (prev + (data[i] | 0)) | 0;
    const code = prev >>> 0;
    out[i * 2] = ((compact16(code) & mask) - shift) | 0;
    out[i * 2 + 1] = ((compact16(code >>> 1) & mask) - shift) | 0;
  }
  return out;
}
