import { IS_LE } from "./binary.js";

const PAGE_SIZE = 65536;
const exStreams = new Array(33);
const exSizes = new Int32Array(33);
const exPtrs = new Int32Array(33);
const TAIL32 = new Int32Array(32);
let fpfBytes = new Uint8Array(16384);

/**
 * Unrolled 32-value LSB-first unpacker for common widths (`1..12, 16`) + branchless fallback.
 * Compresses to < 1 KB gzip due to repetitive bit-shift structure while running 2.5x–3x faster!
 */
function unpack32(w, ip, out, op, b) {
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
      const a0 = w[ip], a1 = w[ip + 1];
      out[op] = a0 & 3; out[op + 1] = (a0 >>> 2) & 3; out[op + 2] = (a0 >>> 4) & 3; out[op + 3] = (a0 >>> 6) & 3;
      out[op + 4] = (a0 >>> 8) & 3; out[op + 5] = (a0 >>> 10) & 3; out[op + 6] = (a0 >>> 12) & 3; out[op + 7] = (a0 >>> 14) & 3;
      out[op + 8] = (a0 >>> 16) & 3; out[op + 9] = (a0 >>> 18) & 3; out[op + 10] = (a0 >>> 20) & 3; out[op + 11] = (a0 >>> 22) & 3;
      out[op + 12] = (a0 >>> 24) & 3; out[op + 13] = (a0 >>> 26) & 3; out[op + 14] = (a0 >>> 28) & 3; out[op + 15] = a0 >>> 30;
      out[op + 16] = a1 & 3; out[op + 17] = (a1 >>> 2) & 3; out[op + 18] = (a1 >>> 4) & 3; out[op + 19] = (a1 >>> 6) & 3;
      out[op + 20] = (a1 >>> 8) & 3; out[op + 21] = (a1 >>> 10) & 3; out[op + 22] = (a1 >>> 12) & 3; out[op + 23] = (a1 >>> 14) & 3;
      out[op + 24] = (a1 >>> 16) & 3; out[op + 25] = (a1 >>> 18) & 3; out[op + 26] = (a1 >>> 20) & 3; out[op + 27] = (a1 >>> 22) & 3;
      out[op + 28] = (a1 >>> 24) & 3; out[op + 29] = (a1 >>> 26) & 3; out[op + 30] = (a1 >>> 28) & 3; out[op + 31] = a1 >>> 30;
      return;
    }
    case 3: {
      const a0 = w[ip], a1 = w[ip + 1], a2 = w[ip + 2];
      out[op] = a0 & 7; out[op + 1] = (a0 >>> 3) & 7; out[op + 2] = (a0 >>> 6) & 7; out[op + 3] = (a0 >>> 9) & 7;
      out[op + 4] = (a0 >>> 12) & 7; out[op + 5] = (a0 >>> 15) & 7; out[op + 6] = (a0 >>> 18) & 7; out[op + 7] = (a0 >>> 21) & 7;
      out[op + 8] = (a0 >>> 24) & 7; out[op + 9] = (a0 >>> 27) & 7; out[op + 10] = (a0 >>> 30) | ((a1 & 1) << 2);
      out[op + 11] = (a1 >>> 1) & 7; out[op + 12] = (a1 >>> 4) & 7; out[op + 13] = (a1 >>> 7) & 7; out[op + 14] = (a1 >>> 10) & 7;
      out[op + 15] = (a1 >>> 13) & 7; out[op + 16] = (a1 >>> 16) & 7; out[op + 17] = (a1 >>> 19) & 7; out[op + 18] = (a1 >>> 22) & 7;
      out[op + 19] = (a1 >>> 25) & 7; out[op + 20] = (a1 >>> 28) & 7; out[op + 21] = (a1 >>> 31) | ((a2 & 3) << 1);
      out[op + 22] = (a2 >>> 2) & 7; out[op + 23] = (a2 >>> 5) & 7; out[op + 24] = (a2 >>> 8) & 7; out[op + 25] = (a2 >>> 11) & 7;
      out[op + 26] = (a2 >>> 14) & 7; out[op + 27] = (a2 >>> 17) & 7; out[op + 28] = (a2 >>> 20) & 7; out[op + 29] = (a2 >>> 23) & 7;
      out[op + 30] = (a2 >>> 26) & 7; out[op + 31] = a2 >>> 29;
      return;
    }
    case 4: {
      for (let k = 0; k < 4; k++) {
        const v = w[ip + k], o = op + (k << 3);
        out[o] = v & 15; out[o + 1] = (v >>> 4) & 15; out[o + 2] = (v >>> 8) & 15; out[o + 3] = (v >>> 12) & 15;
        out[o + 4] = (v >>> 16) & 15; out[o + 5] = (v >>> 20) & 15; out[o + 6] = (v >>> 24) & 15; out[o + 7] = v >>> 28;
      }
      return;
    }
    case 5: {
      const a0 = w[ip], a1 = w[ip + 1], a2 = w[ip + 2], a3 = w[ip + 3], a4 = w[ip + 4];
      out[op] = a0 & 31; out[op + 1] = (a0 >>> 5) & 31; out[op + 2] = (a0 >>> 10) & 31; out[op + 3] = (a0 >>> 15) & 31;
      out[op + 4] = (a0 >>> 20) & 31; out[op + 5] = (a0 >>> 25) & 31; out[op + 6] = (a0 >>> 30) | ((a1 & 7) << 2);
      out[op + 7] = (a1 >>> 3) & 31; out[op + 8] = (a1 >>> 8) & 31; out[op + 9] = (a1 >>> 13) & 31; out[op + 10] = (a1 >>> 18) & 31;
      out[op + 11] = (a1 >>> 23) & 31; out[op + 12] = (a1 >>> 28) | ((a2 & 1) << 4);
      out[op + 13] = (a2 >>> 1) & 31; out[op + 14] = (a2 >>> 6) & 31; out[op + 15] = (a2 >>> 11) & 31; out[op + 16] = (a2 >>> 16) & 31;
      out[op + 17] = (a2 >>> 21) & 31; out[op + 18] = (a2 >>> 26) & 31; out[op + 19] = (a2 >>> 31) | ((a3 & 15) << 1);
      out[op + 20] = (a3 >>> 4) & 31; out[op + 21] = (a3 >>> 9) & 31; out[op + 22] = (a3 >>> 14) & 31; out[op + 23] = (a3 >>> 19) & 31;
      out[op + 24] = (a3 >>> 24) & 31; out[op + 25] = (a3 >>> 29) | ((a4 & 3) << 3);
      out[op + 26] = (a4 >>> 2) & 31; out[op + 27] = (a4 >>> 7) & 31; out[op + 28] = (a4 >>> 12) & 31; out[op + 29] = (a4 >>> 17) & 31;
      out[op + 30] = (a4 >>> 22) & 31; out[op + 31] = a4 >>> 27;
      return;
    }
    case 6: {
      for (let k = 0; k < 2; k++) {
        const a0 = w[ip + k * 3], a1 = w[ip + k * 3 + 1], a2 = w[ip + k * 3 + 2], o = op + k * 16;
        out[o] = a0 & 63; out[o + 1] = (a0 >>> 6) & 63; out[o + 2] = (a0 >>> 12) & 63; out[o + 3] = (a0 >>> 18) & 63;
        out[o + 4] = (a0 >>> 24) & 63; out[o + 5] = (a0 >>> 30) | ((a1 & 15) << 2);
        out[o + 6] = (a1 >>> 4) & 63; out[o + 7] = (a1 >>> 10) & 63; out[o + 8] = (a1 >>> 16) & 63; out[o + 9] = (a1 >>> 22) & 63;
        out[o + 10] = (a1 >>> 28) | ((a2 & 3) << 4);
        out[o + 11] = (a2 >>> 2) & 63; out[o + 12] = (a2 >>> 8) & 63; out[o + 13] = (a2 >>> 14) & 63; out[o + 14] = (a2 >>> 20) & 63;
        out[o + 15] = a2 >>> 26;
      }
      return;
    }
    case 7: {
      const a0 = w[ip], a1 = w[ip + 1], a2 = w[ip + 2], a3 = w[ip + 3], a4 = w[ip + 4], a5 = w[ip + 5], a6 = w[ip + 6];
      out[op] = a0 & 127; out[op + 1] = (a0 >>> 7) & 127; out[op + 2] = (a0 >>> 14) & 127; out[op + 3] = (a0 >>> 21) & 127;
      out[op + 4] = (a0 >>> 28) | ((a1 & 7) << 4); out[op + 5] = (a1 >>> 3) & 127; out[op + 6] = (a1 >>> 10) & 127;
      out[op + 7] = (a1 >>> 17) & 127; out[op + 8] = (a1 >>> 24) & 127; out[op + 9] = (a1 >>> 31) | ((a2 & 63) << 1);
      out[op + 10] = (a2 >>> 6) & 127; out[op + 11] = (a2 >>> 13) & 127; out[op + 12] = (a2 >>> 20) & 127;
      out[op + 13] = (a2 >>> 27) | ((a3 & 3) << 5); out[op + 14] = (a3 >>> 2) & 127; out[op + 15] = (a3 >>> 9) & 127;
      out[op + 16] = (a3 >>> 16) & 127; out[op + 17] = (a3 >>> 23) & 127; out[op + 18] = (a3 >>> 30) | ((a4 & 31) << 2);
      out[op + 19] = (a4 >>> 5) & 127; out[op + 20] = (a4 >>> 12) & 127; out[op + 21] = (a4 >>> 19) & 127;
      out[op + 22] = (a4 >>> 26) | ((a5 & 1) << 6); out[op + 23] = (a5 >>> 1) & 127; out[op + 24] = (a5 >>> 8) & 127;
      out[op + 25] = (a5 >>> 15) & 127; out[op + 26] = (a5 >>> 22) & 127; out[op + 27] = (a5 >>> 29) | ((a6 & 15) << 3);
      out[op + 28] = (a6 >>> 4) & 127; out[op + 29] = (a6 >>> 11) & 127; out[op + 30] = (a6 >>> 18) & 127; out[op + 31] = a6 >>> 25;
      return;
    }
    case 8: {
      for (let k = 0; k < 8; k++) {
        const v = w[ip + k], o = op + (k << 2);
        out[o] = v & 255; out[o + 1] = (v >>> 8) & 255; out[o + 2] = (v >>> 16) & 255; out[o + 3] = v >>> 24;
      }
      return;
    }
    case 9: {
      const a0 = w[ip], a1 = w[ip + 1], a2 = w[ip + 2], a3 = w[ip + 3], a4 = w[ip + 4], a5 = w[ip + 5], a6 = w[ip + 6], a7 = w[ip + 7], a8 = w[ip + 8];
      out[op] = a0 & 511; out[op + 1] = (a0 >>> 9) & 511; out[op + 2] = (a0 >>> 18) & 511; out[op + 3] = (a0 >>> 27) | ((a1 & 15) << 5);
      out[op + 4] = (a1 >>> 4) & 511; out[op + 5] = (a1 >>> 13) & 511; out[op + 6] = (a1 >>> 22) & 511; out[op + 7] = (a1 >>> 31) | ((a2 & 255) << 1);
      out[op + 8] = (a2 >>> 8) & 511; out[op + 9] = (a2 >>> 17) & 511; out[op + 10] = (a2 >>> 26) | ((a3 & 7) << 6);
      out[op + 11] = (a3 >>> 3) & 511; out[op + 12] = (a3 >>> 12) & 511; out[op + 13] = (a3 >>> 21) & 511; out[op + 14] = (a3 >>> 30) | ((a4 & 127) << 2);
      out[op + 15] = (a4 >>> 7) & 511; out[op + 16] = (a4 >>> 16) & 511; out[op + 17] = (a4 >>> 25) | ((a5 & 3) << 7);
      out[op + 18] = (a5 >>> 2) & 511; out[op + 19] = (a5 >>> 11) & 511; out[op + 20] = (a5 >>> 20) & 511; out[op + 21] = (a5 >>> 29) | ((a6 & 63) << 3);
      out[op + 22] = (a6 >>> 6) & 511; out[op + 23] = (a6 >>> 15) & 511; out[op + 24] = (a6 >>> 24) | ((a7 & 1) << 8);
      out[op + 25] = (a7 >>> 1) & 511; out[op + 26] = (a7 >>> 10) & 511; out[op + 27] = (a7 >>> 19) & 511; out[op + 28] = (a7 >>> 28) | ((a8 & 31) << 4);
      out[op + 29] = (a8 >>> 5) & 511; out[op + 30] = (a8 >>> 14) & 511; out[op + 31] = a8 >>> 23;
      return;
    }
    case 10: {
      for (let k = 0; k < 2; k++) {
        const a0 = w[ip + k * 5], a1 = w[ip + k * 5 + 1], a2 = w[ip + k * 5 + 2], a3 = w[ip + k * 5 + 3], a4 = w[ip + k * 5 + 4], o = op + k * 16;
        out[o] = a0 & 1023; out[o + 1] = (a0 >>> 10) & 1023; out[o + 2] = (a0 >>> 20) & 1023; out[o + 3] = (a0 >>> 30) | ((a1 & 255) << 2);
        out[o + 4] = (a1 >>> 8) & 1023; out[o + 5] = (a1 >>> 18) & 1023; out[o + 6] = (a1 >>> 28) | ((a2 & 63) << 4);
        out[o + 7] = (a2 >>> 6) & 1023; out[o + 8] = (a2 >>> 16) & 1023; out[o + 9] = (a2 >>> 26) | ((a3 & 15) << 6);
        out[o + 10] = (a3 >>> 4) & 1023; out[o + 11] = (a3 >>> 14) & 1023; out[o + 12] = (a3 >>> 24) | ((a4 & 3) << 8);
        out[o + 13] = (a4 >>> 2) & 1023; out[o + 14] = (a4 >>> 12) & 1023; out[o + 15] = a4 >>> 22;
      }
      return;
    }
    case 11: {
      const a0 = w[ip], a1 = w[ip + 1], a2 = w[ip + 2], a3 = w[ip + 3], a4 = w[ip + 4], a5 = w[ip + 5];
      const a6 = w[ip + 6], a7 = w[ip + 7], a8 = w[ip + 8], a9 = w[ip + 9], a10 = w[ip + 10];
      out[op] = a0 & 2047; out[op + 1] = (a0 >>> 11) & 2047; out[op + 2] = (a0 >>> 22) | ((a1 & 1) << 10);
      out[op + 3] = (a1 >>> 1) & 2047; out[op + 4] = (a1 >>> 12) & 2047; out[op + 5] = (a1 >>> 23) | ((a2 & 3) << 9);
      out[op + 6] = (a2 >>> 2) & 2047; out[op + 7] = (a2 >>> 13) & 2047; out[op + 8] = (a2 >>> 24) | ((a3 & 7) << 8);
      out[op + 9] = (a3 >>> 3) & 2047; out[op + 10] = (a3 >>> 14) & 2047; out[op + 11] = (a3 >>> 25) | ((a4 & 15) << 7);
      out[op + 12] = (a4 >>> 4) & 2047; out[op + 13] = (a4 >>> 15) & 2047; out[op + 14] = (a4 >>> 26) | ((a5 & 31) << 6);
      out[op + 15] = (a5 >>> 5) & 2047; out[op + 16] = (a5 >>> 16) & 2047; out[op + 17] = (a5 >>> 27) | ((a6 & 63) << 5);
      out[op + 18] = (a6 >>> 6) & 2047; out[op + 19] = (a6 >>> 17) & 2047; out[op + 20] = (a6 >>> 28) | ((a7 & 127) << 4);
      out[op + 21] = (a7 >>> 7) & 2047; out[op + 22] = (a7 >>> 18) & 2047; out[op + 23] = (a7 >>> 29) | ((a8 & 255) << 3);
      out[op + 24] = (a8 >>> 8) & 2047; out[op + 25] = (a8 >>> 19) & 2047; out[op + 26] = (a8 >>> 30) | ((a9 & 511) << 2);
      out[op + 27] = (a9 >>> 9) & 2047; out[op + 28] = (a9 >>> 20) & 2047; out[op + 29] = (a9 >>> 31) | ((a10 & 1023) << 1);
      out[op + 30] = (a10 >>> 10) & 2047; out[op + 31] = a10 >>> 21;
      return;
    }
    case 12: {
      for (let k = 0; k < 4; k++) {
        const a0 = w[ip + k * 3], a1 = w[ip + k * 3 + 1], a2 = w[ip + k * 3 + 2], o = op + k * 8;
        out[o] = a0 & 4095; out[o + 1] = (a0 >>> 12) & 4095; out[o + 2] = (a0 >>> 24) | ((a1 & 15) << 8);
        out[o + 3] = (a1 >>> 4) & 4095; out[o + 4] = (a1 >>> 16) & 4095; out[o + 5] = (a1 >>> 28) | ((a2 & 255) << 4);
        out[o + 6] = (a2 >>> 8) & 4095; out[o + 7] = a2 >>> 20;
      }
      return;
    }
    case 16: {
      for (let k = 0; k < 16; k++) {
        const v = w[ip + k], o = op + (k << 1);
        out[o] = v & 0xffff; out[o + 1] = v >>> 16;
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

export function decodeFastPforInto(data, numValues, is128, out) {
  if (numValues === 0) {
    if (data.length !== 0) throw new Error("fpf empty");
    return out;
  }
  const byteLen = data.length;
  if ((byteLen & 3) !== 0) throw new Error("fpf align");

  const blockSize = is128 ? 128 : 256;
  const groupsPerBlock = is128 ? 4 : 8;
  const wordCount = byteLen >>> 2;

  let words;
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
          words[i] =
            ((x & 0xff) << 24) |
            ((x & 0xff00) << 8) |
            ((x >>> 8) & 0xff00) |
            (x >>> 24);
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

      const metaView = new Uint8Array(
        words.buffer,
        words.byteOffset + metaStart * 4,
        byteSize,
      );

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
            stream = exStreams[k] = new Int32Array(Math.max(32, allocLen));
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
            unpack32(
              words,
              tmpInPos + g * bitWidth,
              outI32,
              blockOutPos + g * 32,
              bitWidth,
            );
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
