import { EMPTY_I32, cursor, readBytes, readVarintU32, takeU32 } from "./binary.js";

const STATE_LOW = 65536;
const SYMBOLS = 257;
const CONTEXTS = 14;
const MIN_SLAB = 4096;

const lowMask = (b) => (b === 0 ? 0 : b === 32 ? 0xffffffff : (1 << b) - 1);

// Lazily initialized symbol info & slab buffers (0 bytes allocated until an rANS stream is decoded).
let symInfo = null;
let slabCap = 0;
let slabOff = null;
let slabFreq = null;
let slabRawBits = null;
let slabNeg = null;
let slabDyCtx = null;
let slabHead = null;
let slabInvalid = null;
let firstScratch = null;

function getSymInfo() {
  if (symInfo !== null) return symInfo;
  symInfo = [0, 1, 2].map((depth) => {
    const rawBits = new Uint8Array(SYMBOLS);
    const negFlag = new Uint8Array(SYMBOLS);
    const dyCtxBase = new Uint8Array(SYMBOLS);
    const head = new Uint8Array(SYMBOLS);
    const valid = new Uint8Array(SYMBOLS);
    valid[0] = 1;
    dyCtxBase[0] = 2;
    for (let s = 1; s < SYMBOLS; s++) {
      const code = (s + 1) >>> 1;
      const len = code >>> depth;
      if (len < 1 || len > 32) continue;
      const kept = len - 1 < depth ? len - 1 : depth;
      const mantissa = code & lowMask(depth);
      if ((mantissa & lowMask(depth - kept)) !== 0) continue;
      const isNeg = (s & 1) === 0;
      if (len === 32 && (!isNeg || mantissa !== 0)) continue;
      valid[s] = 1;
      rawBits[s] = len - 1 - kept;
      negFlag[s] = isNeg ? 1 : 0;
      const cls = ((s + 1) >>> 1) >>> depth;
      dyCtxBase[s] = 2 + 2 * (cls < 6 ? cls : 5);
      head[s] = (1 << kept) | (mantissa >>> (depth - kept));
    }
    return { rawBits, negFlag, dyCtxBase, head, valid };
  });
  return symInfo;
}

function ensureSlab(size) {
  if (slabCap >= size) return;
  slabCap = size;
  slabOff = new Uint16Array(size);
  slabFreq = new Uint16Array(size);
  slabRawBits = new Uint8Array(size);
  slabNeg = new Uint8Array(size);
  slabDyCtx = new Uint8Array(size);
  slabHead = new Uint8Array(size);
  slabInvalid = new Uint8Array(size);
}

const ctxStart = new Uint32Array(16);
const ctxPrecision = new Uint8Array(16);
const laneLens = new Uint32Array(4);
const lanePos = new Uint32Array(4);
const laneState = new Uint32Array(4);
const laneBytes = [null, null, null, null];

export function decodeRansVertices(data, runOffsets, numWords) {
  if ((numWords & 1) !== 0) throw new Error("rans odd");
  const vertexCount = numWords >>> 1;

  if (!firstScratch || firstScratch.length < vertexCount) {
    firstScratch = new Uint8Array(Math.max(1024, vertexCount));
  }
  const first = firstScratch.subarray(0, vertexCount);
  if (runOffsets === undefined) {
    first.fill(1);
  } else {
    if (
      (runOffsets.length > 0 && runOffsets[0] !== 0) ||
      (runOffsets.length === 0 ? 0 : runOffsets[runOffsets.length - 1]) !==
        vertexCount
    ) {
      throw new Error("rans runs");
    }
    first.fill(0);
    for (let i = 0; i < runOffsets.length; i++) {
      const o = runOffsets[i];
      if (o < vertexCount) first[o] = 1;
      else if (o !== vertexCount) throw new Error("rans runs");
    }
  }

  if (data.length === 0) throw new Error("rans empty");
  const depth = data[0];
  if (depth > 2) throw new Error("rans depth");
  const info = getSymInfo()[depth];

  const tBytes = data.subarray(1);
  let tBit = 0;
  const getBits = (bits) => {
    if (bits === 0) return 0;
    const bi = tBit >>> 3;
    const sh = tBit & 7;
    const w =
      (bi < tBytes.length ? tBytes[bi] : 0) |
      ((bi + 1 < tBytes.length ? tBytes[bi + 1] : 0) << 8) |
      ((bi + 2 < tBytes.length ? tBytes[bi + 2] : 0) << 16);
    tBit += bits;
    return (w >>> sh) & lowMask(bits);
  };
  const getGamma = () => {
    let z = 0;
    while (z <= 12) {
      if (getBits(1) === 1) return (1 << z) | getBits(z);
      z++;
    }
    throw new Error("rans freq");
  };

  const tablePrec = new Int8Array(CONTEXTS).fill(-1);
  const tableFreqs = new Array(CONTEXTS);
  let usedSlots = 1;

  for (let c = 0; c < CONTEXTS; c++) {
    const present = getBits(1);
    if (tBit > tBytes.length * 8) throw new Error("rans trunc");
    if (present === 0) continue;
    const precision = getBits(4);
    const max = getBits(9);
    if (precision > 11 || max >= SYMBOLS) throw new Error("rans table");
    const freq = takeU32(max + 1);
    freq.fill(0);
    let sum = 0;
    let bad = 0;
    if (precision === 0) {
      freq[max] = 1;
      sum = 1;
      if (!info.valid[max]) bad = 1;
    } else {
      for (let s = 0; s <= max; s++) {
        const f = getGamma() - 1;
        freq[s] = f;
        sum += f;
        if (f > 0 && !info.valid[s]) bad = 1;
      }
    }
    if (tBit > tBytes.length * 8) throw new Error("rans trunc");
    if (sum !== 1 << precision || freq[max] === 0 || bad) {
      throw new Error("rans freq");
    }
    tablePrec[c] = precision;
    tableFreqs[c] = freq;
    usedSlots += 1 << precision;
  }

  let slabSize = MIN_SLAB;
  while (slabSize < usedSlots) slabSize <<= 1;
  ensureSlab(slabSize);

  slabFreq[0] = 1;
  slabInvalid[0] = 1;
  ctxStart.fill(0);
  ctxPrecision.fill(0);

  let at = 1;
  for (let c = 0; c < CONTEXTS; c++) {
    const prec = tablePrec[c];
    if (prec < 0) continue;
    ctxStart[c] = at;
    ctxPrecision[c] = prec;
    const freq = tableFreqs[c];
    for (let s = 0; s < freq.length; s++) {
      const f = freq[s];
      if (f === 0) continue;
      const rb = info.rawBits[s];
      const ng = info.negFlag[s];
      const dy = info.dyCtxBase[s];
      const hd = info.head[s];
      for (let k = 0; k < f; k++) {
        const slot = at + k;
        slabOff[slot] = k;
        slabFreq[slot] = f;
        slabRawBits[slot] = rb;
        slabNeg[slot] = ng;
        slabDyCtx[slot] = dy;
        slabHead[slot] = hd;
        slabInvalid[slot] = 0;
      }
      at += f;
    }
  }

  const r = cursor(data, 1 + ((tBit + 7) >>> 3), data.length);
  for (let l = 0; l < 4; l++) laneLens[l] = readVarintU32(r);
  for (let l = 0; l < 4; l++) {
    const len = laneLens[l];
    if (len < 4 || (len & 1) !== 0 || r.e - r.p < len) {
      throw new Error("rans lane");
    }
    const bytes = readBytes(r, len);
    laneBytes[l] = bytes;
    lanePos[l] = 4;
    laneState[l] =
      (bytes[0] | (bytes[1] << 8) | (bytes[2] << 16) | (bytes[3] << 24)) >>> 0;
  }

  const raw = readBytes(r, r.e - r.p);
  const rawLen = raw.length;
  let rawBit = 0;
  const slabMask = slabSize - 1;

  const stepLane = (l, c) => {
    const prec = ctxPrecision[c];
    let st = laneState[l];
    const slot = (ctxStart[c] + (st & ((1 << prec) - 1))) & slabMask;
    st = (Math.imul(slabFreq[slot], st >>> prec) + slabOff[slot]) >>> 0;
    if (st < STATE_LOW) {
      const bytes = laneBytes[l];
      const p = lanePos[l];
      const word = p + 2 <= bytes.length ? bytes[p] | (bytes[p + 1] << 8) : 0;
      st = ((st << 16) | word) >>> 0;
      lanePos[l] = p + 2;
    }
    laneState[l] = st;
    return slot;
  };

  const readDelta = (slot) => {
    const rb = slabRawBits[slot];
    let low = 0;
    if (rb > 0) {
      const bi = rawBit >>> 3;
      const sh = rawBit & 7;
      const lo32 =
        ((bi < rawLen ? raw[bi] : 0) |
          ((bi + 1 < rawLen ? raw[bi + 1] : 0) << 8) |
          ((bi + 2 < rawLen ? raw[bi + 2] : 0) << 16) |
          ((bi + 3 < rawLen ? raw[bi + 3] : 0) << 24)) >>>
        0;
      if (sh === 0) {
        low = (lo32 & lowMask(rb)) >>> 0;
      } else {
        const b4 = bi + 4 < rawLen ? raw[bi + 4] : 0;
        low = (((lo32 >>> sh) | (b4 << (32 - sh))) & lowMask(rb)) >>> 0;
      }
      rawBit += rb;
    }
    const mag = ((slabHead[slot] << rb) | low) | 0;
    const sign = -slabNeg[slot];
    return ((mag ^ sign) - sign) | 0;
  };

  const out = numWords === 0 ? EMPTY_I32 : new Int32Array(numWords);
  let x = 0;
  let y = 0;
  let bad = 0;

  const pairs = vertexCount & ~1;
  for (let v = 0; v < pairs; v += 2) {
    const f0 = first[v];
    const f1 = first[v + 1];
    const ex0 = stepLane(0, f0);
    const ex1 = stepLane(2, f1);
    const ey0 = stepLane(1, slabDyCtx[ex0] + f0);
    const ey1 = stepLane(3, slabDyCtx[ex1] + f1);
    bad |= slabInvalid[ex0] | slabInvalid[ey0] | slabInvalid[ex1] | slabInvalid[ey1];

    x = (x + readDelta(ex0)) | 0;
    y = (y + readDelta(ey0)) | 0;
    out[v * 2] = x;
    out[v * 2 + 1] = y;

    x = (x + readDelta(ex1)) | 0;
    y = (y + readDelta(ey1)) | 0;
    out[v * 2 + 2] = x;
    out[v * 2 + 3] = y;
  }

  if ((vertexCount & 1) !== 0) {
    const v = vertexCount - 1;
    const f0 = first[v];
    const ex0 = stepLane(0, f0);
    const ey0 = stepLane(1, slabDyCtx[ex0] + f0);
    bad |= slabInvalid[ex0] | slabInvalid[ey0];
    x = (x + readDelta(ex0)) | 0;
    y = (y + readDelta(ey0)) | 0;
    out[v * 2] = x;
    out[v * 2 + 1] = y;
  }

  if (bad !== 0) throw new Error("rans missing table");
  for (let l = 0; l < 4; l++) {
    if (laneState[l] !== STATE_LOW || lanePos[l] !== laneBytes[l].length) {
      throw new Error("rans state");
    }
    laneBytes[l] = null;
  }
  if (((rawBit + 7) >>> 3) !== rawLen) throw new Error("rans raw len");
  if (firstScratch.length > 65536) firstScratch = null;
  return out;
}
