import { takeU32 } from "./binary.js";

const symLo = new Uint32Array(256);
const symHi = new Uint32Array(256);
const symLens = new Uint8Array(256);

let fsstBuf = new Uint8Array(16384);
let fsstView = new DataView(fsstBuf.buffer);
let frontBuf = new Uint8Array(16384);

export function trimFsstBuffers() {
  if (fsstBuf.length > 1048576) {
    fsstBuf = new Uint8Array(16384);
    fsstView = new DataView(fsstBuf.buffer);
  }
  if (frontBuf.length > 1048576) {
    frontBuf = new Uint8Array(16384);
  }
}

/**
 * Expand an FSST corpus into a slice of reusable `fsstBuf` using 2-word `DataView` stores.
 * @param {Uint32Array} symbolLengths
 * @param {Uint8Array} symbolTable
 * @param {Uint8Array} corpus
 * @returns {Uint8Array}
 */
export function decodeFsstBytes(symbolLengths, symbolTable, corpus) {
  const symCount = symbolLengths.length;
  const count = symCount < 255 ? symCount : 255;
  const tblLen = symbolTable.length;
  let pos = 0;
  for (let i = 0; i < symCount; i++) {
    const len = symbolLengths[i];
    if (len > 8 || pos + len > tblLen) throw new Error("fsst table");
    if (i < count) {
      symLens[i] = len;
      let lo = 0;
      let hi = 0;
      const nLo = len < 4 ? len : 4;
      for (let k = 0; k < nLo; k++) lo |= symbolTable[pos + k] << (k << 3);
      for (let k = 4; k < len; k++) hi |= symbolTable[pos + k] << ((k - 4) << 3);
      symLo[i] = lo >>> 0;
      symHi[i] = hi >>> 0;
    }
    pos += len;
  }

  const cLen = corpus.length;
  const maxOut = cLen * 8 + 8;
  if (fsstBuf.length < maxOut) {
    let cap = fsstBuf.length * 2;
    while (cap < maxOut) cap *= 2;
    fsstBuf = new Uint8Array(cap);
    fsstView = new DataView(fsstBuf.buffer);
  }

  const buf = fsstBuf;
  const view = fsstView;
  let outLen = 0;
  for (let i = 0; i < cLen; i++) {
    const code = corpus[i];
    if (code < count) {
      view.setUint32(outLen, symLo[code], true);
      view.setUint32(outLen + 4, symHi[code], true);
      outLen += symLens[code];
    } else if (code === 255) {
      if (++i >= cLen) throw new Error("fsst escape");
      buf[outLen++] = corpus[i];
    } else {
      throw new Error("fsst code");
    }
  }

  return buf.subarray(0, outLen);
}

/**
 * Rebuild a front-coded dictionary into reusable `frontBuf` and an arena `entryLengths` slice.
 * @param {Uint32Array} lengths
 * @param {Uint8Array} suffixes
 * @returns {{ bytes: Uint8Array, entryLengths: Uint32Array }}
 */
export function decodeFrontCoded(lengths, suffixes) {
  if ((lengths.length & 1) !== 0) throw new Error("front odd");
  const entryCount = lengths.length >>> 1;
  const entryLengths = takeU32(entryCount);

  let totalBytes = 0;
  let prevLen = 0;
  let suffixSum = 0;
  for (let i = 0; i < entryCount; i++) {
    const shared = lengths[i];
    const suffixLen = lengths[entryCount + i];
    if (shared > prevLen) throw new Error("front prefix");
    suffixSum += suffixLen;
    if (suffixSum > suffixes.length) throw new Error("front suffix");
    prevLen = shared + suffixLen;
    entryLengths[i] = prevLen;
    totalBytes += prevLen;
  }
  if (suffixSum !== suffixes.length) throw new Error("front trailing");

  if (frontBuf.length < totalBytes) {
    let cap = frontBuf.length * 2;
    while (cap < totalBytes) cap *= 2;
    frontBuf = new Uint8Array(cap);
  }
  const bytes = frontBuf.subarray(0, totalBytes);
  let outPos = 0;
  let prevStart = 0;
  let suffixPos = 0;
  for (let i = 0; i < entryCount; i++) {
    const shared = lengths[i];
    const suffixLen = lengths[entryCount + i];
    const start = outPos;
    if (shared > 0) {
      bytes.copyWithin(outPos, prevStart, prevStart + shared);
      outPos += shared;
    }
    if (suffixLen > 0) {
      bytes.set(suffixes.subarray(suffixPos, suffixPos + suffixLen), outPos);
      suffixPos += suffixLen;
      outPos += suffixLen;
    }
    prevStart = start;
  }

  return { bytes, entryLengths };
}
