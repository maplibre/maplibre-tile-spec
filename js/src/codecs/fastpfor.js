import { IS_LE } from "./binary.js";

const PAGE_SIZE = 65536;
const exStreams = new Array(33);
const exSizes = new Uint32Array(33);
const exPtrs = new Uint32Array(33);
const TAIL32 = new Uint32Array(32);
let fpfBytes = new Uint8Array(16384);

/**
 * Branchless 32-value LSB-first unpacker for `bitWidth` in `1..31`.
 * Uses `(lo >>> sh) | ((hi << 1) << (31 - sh))` (valid for all `sh` in `0..31` without `sh === 0` UB!)
 * up to the penultimate word, then extracts remaining values from the final word without OOB reads.
 */
function unpack32(words, inPos, out, outPos, bitWidth) {
  if (bitWidth === 2) {
    const w0 = words[inPos];
    const w1 = words[inPos + 1];
    for (let i = 0; i < 16; i++) {
      const sh = i << 1;
      out[outPos + i] = (w0 >>> sh) & 3;
      out[outPos + 16 + i] = (w1 >>> sh) & 3;
    }
    return;
  }

  const mask = (1 << bitWidth) - 1;
  const splitBit = (bitWidth - 1) << 5;
  let bitPos = 0;
  let i = 0;

  while (bitPos < splitBit) {
    const w = inPos + (bitPos >>> 5);
    const sh = bitPos & 31;
    out[outPos + i] =
      ((words[w] >>> sh) | ((words[w + 1] << 1) << (31 - sh))) & mask;
    bitPos += bitWidth;
    i++;
  }

  const last = words[inPos + bitWidth - 1];
  for (; i < 32; i++, bitPos += bitWidth) {
    out[outPos + i] = (last >>> (bitPos & 31)) & mask;
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
    words = new Uint32Array(data.buffer, data.byteOffset, wordCount);
  } else {
    if (fpfBytes.length < byteLen) {
      fpfBytes = new Uint8Array(Math.max(byteLen, fpfBytes.length * 2));
    }
    if (IS_LE) {
      fpfBytes.set(data);
      words = new Uint32Array(fpfBytes.buffer, 0, wordCount);
      if (!is128) {
        for (let i = 0; i < wordCount; i++) {
          const x = words[i];
          words[i] =
            (((x & 0xff) << 24) |
              ((x & 0xff00) << 8) |
              ((x >>> 8) & 0xff00) |
              (x >>> 24)) >>>
            0;
        }
      }
    } else {
      words = new Uint32Array(fpfBytes.buffer, 0, wordCount);
      const dv = new DataView(data.buffer, data.byteOffset, byteLen);
      for (let i = 0; i < wordCount; i++) words[i] = dv.getUint32(i * 4, is128);
    }
  }

  let inPos = 0;
  let outPos = 0;

  if (wordCount > 0) {
    const alignedLength = words[inPos++];
    if ((alignedLength & (blockSize - 1)) !== 0 || alignedLength > numValues) {
      throw new Error("fpf len");
    }

    while (outPos < alignedLength) {
      const thisSize = Math.min(PAGE_SIZE, alignedLength - outPos);
      if (inPos >= wordCount) throw new Error("fpf page");
      const pageStart = inPos;
      const whereMeta = words[pageStart];
      if (whereMeta === 0 || pageStart + whereMeta >= wordCount) {
        throw new Error("fpf meta");
      }
      const packedStart = pageStart + 1;
      const packedEnd = pageStart + whereMeta;
      const byteSize = words[packedEnd];
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
          const size = words[inExcept++];
          if (size > PAGE_SIZE) throw new Error("fpf ex size");
          const allocLen = (size + 31) & ~31;
          let stream = exStreams[k];
          if (!stream || stream.length < allocLen) {
            stream = exStreams[k] = new Uint32Array(Math.max(32, allocLen));
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
          out.fill(0, blockOutPos, blockOutPos + blockSize);
        } else if (bitWidth === 32) {
          if (tmpInPos + blockSize > packedEnd) throw new Error("fpf blk");
          out.set(words.subarray(tmpInPos, tmpInPos + blockSize), blockOutPos);
          tmpInPos += blockSize;
        } else {
          const blkWords = groupsPerBlock * bitWidth;
          if (tmpInPos + blkWords > packedEnd) throw new Error("fpf blk");
          for (let g = 0; g < groupsPerBlock; g++) {
            unpack32(
              words,
              tmpInPos + g * bitWidth,
              out,
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
            const patch = (1 << bitWidth) >>> 0;
            for (let e = 0; e < exCount; e++) {
              const pos = metaView[bytePos++];
              if (pos >= blockSize) throw new Error("fpf pos");
              out[blockOutPos + pos] = (out[blockOutPos + pos] | patch) >>> 0;
            }
          } else {
            const stream = exStreams[k];
            let ptr = exPtrs[k];
            if (!stream || ptr + exCount > exSizes[k]) throw new Error("fpf ex");
            for (let e = 0; e < exCount; e++) {
              const pos = metaView[bytePos++];
              if (pos >= blockSize) throw new Error("fpf pos");
              out[blockOutPos + pos] =
                (out[blockOutPos + pos] | ((stream[ptr++] << bitWidth) >>> 0)) >>> 0;
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
        out[outPos++] = acc >>> 0;
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
