import { cursor, readBytes, readVarintU64 } from "./binary.js";

export const PRES_BITMAP = 0;
export const PRES_RUNS = 1;
export const PRES_SPARSE = 2;

const POP = new Uint8Array(256);
for (let i = 1; i < 256; i++) POP[i] = POP[i >> 1] + (i & 1);

export function maskBitmapTail(bytes, count) {
  const byteLen = (count + 7) >>> 3;
  const rem = count & 7;
  if (rem === 0) {
    return bytes.length === byteLen ? bytes : bytes.subarray(0, byteLen);
  }
  const mask = (1 << rem) - 1;
  if ((bytes[byteLen - 1] & ~mask) === 0 && bytes.length === byteLen) {
    return bytes;
  }
  const copy = bytes.slice(0, byteLen);
  copy[byteLen - 1] &= mask;
  return copy;
}

export function bitmapPopcount(bitmap, count) {
  const full = count >>> 3;
  let ones = 0;
  for (let i = 0; i < full; i++) ones += POP[bitmap[i]];
  const rem = count & 7;
  if (rem > 0) ones += POP[bitmap[full] & ((1 << rem) - 1)];
  return ones;
}

export const isBitSet = (bitmap, i) =>
  bitmap === undefined || ((bitmap[i >>> 3] >>> (i & 7)) & 1) === 1;

export function normalizePresentBitmap(bitmap, count) {
  if (bitmap === undefined || count === 0) return undefined;
  return bitmapPopcount(bitmap, count) === count
    ? undefined
    : maskBitmapTail(bitmap, count);
}

/**
 * Scatter `dense` values into `target` (`length === count`) at set-bit positions of `bitmap`
 * using fast byte-skipping + `ctz32` bit scanning (`O(popcount + count/8)` instead of `O(count)`).
 * @param {Uint8Array} bitmap
 * @param {number} count
 * @param {ArrayLike<any>} dense
 * @param {any} target
 */
export function scatterPresent(bitmap, count, dense, target) {
  const fullBytes = count >>> 3;
  let d = 0;
  for (let bIdx = 0; bIdx < fullBytes; bIdx++) {
    let byte = bitmap[bIdx];
    if (byte === 0) continue;
    const base = bIdx << 3;
    if (byte === 0xff) {
      target[base] = dense[d];
      target[base + 1] = dense[d + 1];
      target[base + 2] = dense[d + 2];
      target[base + 3] = dense[d + 3];
      target[base + 4] = dense[d + 4];
      target[base + 5] = dense[d + 5];
      target[base + 6] = dense[d + 6];
      target[base + 7] = dense[d + 7];
      d += 8;
      continue;
    }
    while (byte !== 0) {
      const lsb = byte & -byte;
      target[base + 31 - Math.clz32(lsb)] = dense[d++];
      byte ^= lsb;
    }
  }
  const rem = count & 7;
  if (rem > 0) {
    let byte = bitmap[fullBytes] & ((1 << rem) - 1);
    const base = fullBytes << 3;
    while (byte !== 0) {
      const lsb = byte & -byte;
      target[base + 31 - Math.clz32(lsb)] = dense[d++];
      byte ^= lsb;
    }
  }
}

export function readPresenceBits(r, count, coding) {
  const byteLen = (count + 7) >>> 3;
  if (coding === PRES_BITMAP) {
    return maskBitmapTail(readBytes(r, byteLen), count);
  }
  if (coding === PRES_SPARSE) {
    const sumLen = (byteLen + 7) >>> 3;
    const summary = readBytes(r, sumLen);
    for (let b = byteLen; b < sumLen * 8; b++) {
      if (((summary[b >>> 3] >>> (b & 7)) & 1) !== 0) {
        throw new Error("sparse summary");
      }
    }
    const stored = readBytes(r, bitmapPopcount(summary, byteLen));
    const plain = new Uint8Array(byteLen);
    let s = 0;
    for (let i = 0; i < byteLen; i++) {
      if (((summary[i >>> 3] >>> (i & 7)) & 1) !== 0) {
        const v = stored[s++];
        if (v === 0) throw new Error("sparse zero");
        plain[i] = v;
      }
    }
    return maskBitmapTail(plain, count);
  }
  if (coding === PRES_RUNS) {
    const plain = new Uint8Array(byteLen);
    let at = 0;
    let present = false;
    const maxBig = BigInt(count);
    while (at < count) {
      const lenBig = readVarintU64(r);
      if (lenBig > maxBig - BigInt(at)) throw new Error("runs overflow");
      const end = at + Number(lenBig);
      if (present) {
        let i = at;
        while (i < end && (i & 7) !== 0) {
          plain[i >>> 3] |= 1 << (i & 7);
          i++;
        }
        const fullEnd = end & ~7;
        if (i < fullEnd) {
          plain.fill(0xff, i >>> 3, fullEnd >>> 3);
          i = fullEnd;
        }
        while (i < end) {
          plain[i >>> 3] |= 1 << (i & 7);
          i++;
        }
      }
      at = end;
      present = !present;
    }
    return plain;
  }
  throw new Error("presence coding");
}

export function splitPresenceBytes(r, count, coding) {
  const start = r.p;
  const byteLen = (count + 7) >>> 3;
  if (coding === PRES_BITMAP) return readBytes(r, byteLen);
  if (coding === PRES_SPARSE) {
    const sumLen = (byteLen + 7) >>> 3;
    const summary = readBytes(r, sumLen);
    let ones = 0;
    for (let i = 0; i < sumLen; i++) ones += POP[summary[i]];
    readBytes(r, ones);
    return r.b.subarray(start, r.p);
  }
  if (coding === PRES_RUNS) {
    let at = 0;
    const maxBig = BigInt(count);
    while (at < count) {
      const lenBig = readVarintU64(r);
      if (lenBig > maxBig - BigInt(at)) throw new Error("runs overflow");
      at += Number(lenBig);
    }
    return r.b.subarray(start, r.p);
  }
  throw new Error("presence coding");
}

export function popcountPresencePayload(payload, count, coding) {
  if (coding === PRES_BITMAP) return bitmapPopcount(payload, count);
  if (coding === PRES_SPARSE) {
    const byteLen = (count + 7) >>> 3;
    let s = (byteLen + 7) >>> 3;
    let total = 0;
    for (let at = 0; at < byteLen; at++) {
      if (((payload[at >>> 3] >>> (at & 7)) & 1) !== 0) {
        const valid = Math.min(8, count - at * 8);
        total += POP[payload[s++] & (valid === 8 ? 0xff : (1 << valid) - 1)];
      }
    }
    return total;
  }
  const r = cursor(payload);
  let at = 0;
  let present = false;
  let total = 0;
  while (at < count) {
    const len = Number(readVarintU64(r));
    if (len > count - at) throw new Error("runs overflow");
    if (present) total += len;
    at += len;
    present = !present;
  }
  return total;
}
