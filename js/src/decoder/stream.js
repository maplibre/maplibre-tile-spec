import { decodeAlpFloats, parseAlpScale } from "../codecs/alp.js";
import {
  allocF32,
  allocF64,
  allocI8,
  allocU8,
  allocU32,
  allocU64,
  cursor,
  decodeRawFloats,
  decodeRawU32Into,
  decodeRawU64Into,
  decodeVarintU32Into,
  decodeVarintU32sAllScratch,
  decodeVarintU64Into,
  decodeVarintU64sAllScratch,
  readBytes,
  readU8,
  readVarintI64,
  readVarintU32,
  takeU32,
  takeU64,
} from "../codecs/binary.js";
import { unpackBitpackU32Into, unpackBitpackU64Into } from "../codecs/bitpack.js";
import { decodeFastPforInto } from "../codecs/fastpfor.js";
import { decodeMortonDelta } from "../codecs/morton.js";
import {
  PRES_BITMAP,
  PRES_RUNS,
  PRES_SPARSE,
  maskBitmapTail,
  readPresenceBits,
  splitPresenceBytes,
} from "../codecs/presence.js";
import {
  decodeByteRle,
  decodeRleInterleavedInto,
  decodeRleSplitU32Into,
  decodeRleSplitU64Into,
} from "../codecs/rle.js";
import {
  asI32,
  asI64,
  cwDelta2Vec2InPlace,
  cwDeltaVec2InPlace,
  cwDeltaVec3InPlace,
  zigzagDelta2_32InPlace,
  zigzagDelta2_64InPlace,
  zigzagDelta32InPlace,
  zigzagDelta64InPlace,
  zigzagI32InPlace,
  zigzagI64InPlace,
} from "../codecs/zigzag.js";

// Stream types
export const ST_PRESENT = 0;
export const ST_DATA_NONE = 1;
export const ST_DATA_SINGLE = 2;
export const ST_DATA_SHARED = 3;
export const ST_DATA_VERTEX = 4;
export const ST_DATA_MORTON = 5;
export const ST_DATA_FSST = 6;
export const ST_OFF_VERTEX = 7;
export const ST_OFF_INDEX = 8;
export const ST_OFF_STRING = 9;
export const ST_OFF_KEY = 10;
export const ST_OFF_TREE_SYM = 11;
export const ST_OFF_TREE_CYC = 12;
export const ST_LEN_VARBIN = 13;
export const ST_LEN_GEOM = 14;
export const ST_LEN_PARTS = 15;
export const ST_LEN_RINGS = 16;
export const ST_LEN_TRIS = 17;
export const ST_LEN_SYM = 18;
export const ST_LEN_DICT = 19;
export const ST_LEN_NESTED = 20;

// Physical encodings
export const PH_NONE = 0;
export const PH_FPF256 = 1;
export const PH_FPF128 = 2;
export const PH_VARINT = 3;
export const PH_BITPACK = 4;

// Logical encodings
export const LG_NONE = 0;
export const LG_DELTA = 1;
export const LG_DELTA2 = 2;
export const LG_RLE = 3;
export const LG_DELTA_RLE = 4;
export const LG_CW_DELTA = 5;
export const LG_CW_DELTA2 = 6;
export const LG_MORTON = 7;
export const LG_RANS = 8;
export const LG_BYTE_RLE = 9;
export const LG_RUNS = 10;
export const LG_SPARSE = 11;
export const LG_ALP = 12;
export const LG_DICT = 13;

// v2 stream families
export const FM_INT = 0;
export const FM_STR = 1;
export const FM_BOOL = 2;
export const FM_FLOAT = 3;
export const FM_VERTEX = 4;
export const FM_BYTES = 5;

// Hint kinds for v1
export const HK_INT = 0;
export const HK_FLOAT = 1;
export const HK_BOOL = 2;
const HK_VERTEX = 3;

export const HAS_EXPLICIT_COUNT = 0x80;
export const NO_Z = 99;

const FAMILY_TABLE = [
  [LG_NONE, LG_DELTA, LG_RLE, LG_DELTA_RLE, PH_BITPACK + 100, LG_DELTA2], // FM_INT
  [LG_NONE, LG_DELTA, LG_RLE, LG_DELTA_RLE, PH_BITPACK + 100, LG_DELTA2], // FM_STR
  [LG_NONE, LG_RUNS, LG_SPARSE], // FM_BOOL
  [LG_NONE, -1, LG_ALP, LG_DICT], // FM_FLOAT (code 1 unassigned)
  [LG_NONE, LG_DELTA, LG_CW_DELTA, LG_MORTON, LG_CW_DELTA2, LG_RANS], // FM_VERTEX
  [LG_NONE, 200], // FM_BYTES (200 = front-coded)
];

function makeStream(
  t,
  l,
  p,
  n,
  w,
  rleRuns,
  rleVals,
  mBits,
  mShift,
  alpScale,
  alpBase,
  zStep,
  zDatum,
  xyzLog,
  front,
  d,
) {
  return {
    t,
    l,
    p,
    n,
    w,
    rleRuns,
    rleVals,
    mBits,
    mShift,
    alpScale,
    alpBase,
    zStep,
    zDatum,
    xyzLog,
    front,
    d,
  };
}

export function parseStream01(r, hint, isBool = false) {
  const stByte = readU8(r);
  const cat = stByte & 0xf0;
  const sub = stByte & 0x0f;
  let t;
  let kind;
  if (cat === 0x00) {
    t = ST_PRESENT;
    kind = HK_BOOL;
  } else if (cat === 0x10) {
    if (sub > 5) throw new Error("v1 st");
    t = ST_DATA_NONE + sub;
    kind = sub === 3 || sub === 4 ? HK_VERTEX : hint;
  } else if (cat === 0x20) {
    if (sub > 3) throw new Error("v1 st");
    t = ST_OFF_VERTEX + sub;
    kind = HK_INT;
  } else if (cat === 0x30) {
    if (sub > 6) throw new Error("v1 st");
    t = ST_LEN_VARBIN + sub;
    kind = HK_INT;
  } else {
    throw new Error("v1 st");
  }

  const encByte = readU8(r);
  const logBits = encByte & 0xfc;
  const physBits = encByte & 0x03;
  if (physBits === 3) throw new Error("v1 phys");
  const p = physBits === 0 ? PH_NONE : physBits === 1 ? PH_FPF256 : PH_VARINT;

  let l;
  switch (logBits) {
    case 0x00:
      l = LG_NONE;
      break;
    case 0x20:
      if (kind !== HK_INT && kind !== HK_VERTEX) throw new Error("v1 log");
      l = LG_DELTA;
      break;
    case 0x2c:
      if (kind !== HK_INT) throw new Error("v1 log");
      l = LG_DELTA_RLE;
      break;
    case 0x40:
      if (kind !== HK_VERTEX) throw new Error("v1 log");
      l = LG_CW_DELTA;
      break;
    case 0x60:
      if (kind === HK_INT) l = LG_RLE;
      else if (kind === HK_BOOL) l = LG_BYTE_RLE;
      else throw new Error("v1 log");
      break;
    case 0x84:
      if (kind !== HK_VERTEX) throw new Error("v1 log");
      l = LG_MORTON;
      break;
    default:
      throw new Error("v1 log");
  }

  const w = readVarintU32(r);
  const byteLen = readVarintU32(r);

  let rleRuns = -2;
  let rleVals = 0;
  let mBits = 0;
  let mShift = 0;

  if (l === LG_RLE || l === LG_DELTA_RLE || l === LG_BYTE_RLE) {
    if (isBool) {
      rleRuns = (w + 7) >>> 3;
      rleVals = byteLen;
    } else {
      rleRuns = readVarintU32(r);
      rleVals = readVarintU32(r);
    }
  } else if (l === LG_MORTON) {
    mBits = readVarintU32(r);
    mShift = readVarintU32(r);
    if (mBits > 16) throw new Error("morton bits");
  }

  const wpv = kind === HK_VERTEX && l !== LG_MORTON ? 2 : 1;
  if (w % wpv !== 0) throw new Error("partial vertex");
  const n = w / wpv;
  const d = readBytes(r, byteLen);

  if ((l === LG_RLE || l === LG_DELTA_RLE) && p === PH_VARINT && !isBool) {
    const vr = cursor(d);
    let sum = 0;
    for (let i = 0; i < rleRuns; i++) sum += readVarintU32(vr);
    if (sum !== rleVals) throw new Error("rle sum");
  }

  return makeStream(
    t,
    l,
    p,
    n,
    w,
    rleRuns,
    rleVals,
    mBits,
    mShift,
    0,
    0n,
    NO_Z,
    0,
    0,
    0,
    d,
  );
}

export function parseStream02(
  r,
  family,
  wordWidth,
  t,
  impliedCount,
  expectedStrLayout = -1,
) {
  const encByte = readU8(r);
  const explicit = (encByte & HAS_EXPLICIT_COUNT) !== 0;
  const code = (encByte & 0x70) >>> 4;
  const physField = encByte & 0x0c;
  const extField = encByte & 0x03;

  const members = FAMILY_TABLE[family];
  if (code >= members.length || members[code] < 0) {
    throw new Error("v2 enc");
  }
  let l = members[code];

  if (family === FM_STR) {
    if (expectedStrLayout >= 0 && extField !== expectedStrLayout) {
      throw new Error("v2 str ext");
    }
  } else if (family === FM_VERTEX) {
    if (extField > 1) throw new Error("v2 vtx ext");
  } else if (extField !== 0) {
    throw new Error("v2 ext");
  }

  const isXyz = family === FM_VERTEX && extField === 1;
  let p = PH_NONE;
  let omitsLen = false;
  let front = 0;

  const physInt = () => {
    if (physField === 0x04) return PH_NONE;
    if (physField === 0x08) return PH_VARINT;
    if (physField === 0x0c) return PH_FPF128;
    throw new Error("v2 phys");
  };
  const rawInt = () => {
    if (physField === 0x00) {
      omitsLen = true;
      return PH_NONE;
    }
    return physInt();
  };

  if (family === FM_INT || family === FM_STR) {
    if (l === LG_NONE) p = rawInt();
    else if (l === LG_DELTA || l === LG_DELTA2) p = physInt();
    else if (l === LG_RLE || l === LG_DELTA_RLE) {
      if (physField !== 0) throw new Error("v2 rle phys");
      p = PH_VARINT;
    } else {
      // bitpack
      if (physField !== 0) throw new Error("v2 bp phys");
      l = LG_NONE;
      p = PH_BITPACK;
    }
  } else if (family === FM_BYTES) {
    if (explicit || physField !== 0x04) throw new Error("v2 bytes");
    if (l === 200) {
      front = 1;
      l = LG_NONE;
    }
  } else if (family === FM_BOOL) {
    if (physField !== 0) throw new Error("v2 bool");
    omitsLen = true;
  } else if (family === FM_FLOAT) {
    if (l === LG_NONE) {
      if (physField === 0x00) omitsLen = true;
      else if (physField !== 0x04) throw new Error("v2 float");
    } else {
      p = physInt();
    }
  } else if (isXyz) {
    if (l === LG_NONE) p = rawInt();
    else if (l === LG_DELTA || l === LG_CW_DELTA) p = physInt();
    else throw new Error("v2 xyz");
  } else {
    if (l === LG_NONE) p = rawInt();
    else if (l === LG_RANS) {
      if (physField !== 0) throw new Error("v2 rans");
    } else {
      p = physInt();
    }
  }

  const wpv = isXyz ? 3 : family === FM_VERTEX && l !== LG_MORTON ? 2 : 1;
  let n;
  let w;
  let byteLen;

  if (family === FM_BYTES) {
    byteLen = readVarintU32(r);
    n = byteLen;
    w = byteLen;
  } else {
    if (explicit) n = readVarintU32(r);
    else if (impliedCount >= 0) n = impliedCount;
    else throw new Error("v2 no count");

    w = n * wpv;
    if (!Number.isSafeInteger(w) || w > 0xffffffff) {
      throw new RangeError("overflow");
    }

    if (family === FM_BOOL) {
      const coding =
        l === LG_NONE ? PRES_BITMAP : l === LG_RUNS ? PRES_RUNS : PRES_SPARSE;
      const d = splitPresenceBytes(r, n, coding);
      return makeStream(
        t,
        l,
        PH_NONE,
        n,
        w,
        -2,
        0,
        0,
        0,
        0,
        0n,
        NO_Z,
        0,
        0,
        0,
        d,
      );
    }

    if (omitsLen) {
      byteLen = w * wordWidth;
      if (!Number.isSafeInteger(byteLen) || byteLen > 0xffffffff) {
        throw new RangeError("overflow");
      }
    } else {
      byteLen = readVarintU32(r);
    }
  }

  let rleRuns = -2;
  let rleVals = 0;
  let mBits = 0;
  let mShift = 0;
  let alpScale = 0;
  let alpBase = 0n;
  let zStep = NO_Z;
  let zDatum = 0;
  let xyzLog = 0;

  if (l === LG_RLE || l === LG_DELTA_RLE) {
    rleRuns = -1;
    rleVals = n;
  } else if (family === FM_FLOAT && l === LG_ALP) {
    alpScale = parseAlpScale(readU8(r));
    alpBase = readVarintI64(r);
  } else if (isXyz) {
    const codeByte = readU8(r);
    if (codeByte >= 16) throw new Error("z_step");
    zStep = (codeByte & 7) - 3;
    zDatum = (codeByte & 8) >>> 3;
    xyzLog = l;
  } else if (family === FM_VERTEX && l === LG_MORTON) {
    mBits = readVarintU32(r);
    mShift = readVarintU32(r);
    if (mBits > 16) throw new Error("morton bits");
  }

  const d = readBytes(r, byteLen);
  return makeStream(
    t,
    l,
    p,
    n,
    w,
    rleRuns,
    rleVals,
    mBits,
    mShift,
    alpScale,
    alpBase,
    zStep,
    zDatum,
    xyzLog,
    front,
    d,
  );
}

export function decodeBoolBitmap(s) {
  const nb = (s.n + 7) >>> 3;
  if (s.l === LG_BYTE_RLE) return maskBitmapTail(decodeByteRle(s.d, nb), s.n);
  if (s.l === LG_NONE && s.p === PH_NONE) {
    if (s.d.length !== nb) throw new Error("bool len");
    return maskBitmapTail(s.d, s.n);
  }
  if (s.l === LG_RUNS || s.l === LG_SPARSE) {
    const r = cursor(s.d);
    const bits = readPresenceBits(
      r,
      s.n,
      s.l === LG_RUNS ? PRES_RUNS : PRES_SPARSE,
    );
    if (r.p !== r.e) throw new Error("bool trailing");
    return bits;
  }
  throw new Error("bool enc");
}

export function decodeBoolValues(s) {
  const bits = decodeBoolBitmap(s);
  const out = allocU8(s.n);
  const full = s.n >>> 3;
  for (let b = 0; b < full; b++) {
    const v = bits[b];
    if (v === 0) continue;
    const base = b << 3;
    out[base] = v & 1;
    out[base + 1] = (v >>> 1) & 1;
    out[base + 2] = (v >>> 2) & 1;
    out[base + 3] = (v >>> 3) & 1;
    out[base + 4] = (v >>> 4) & 1;
    out[base + 5] = (v >>> 5) & 1;
    out[base + 6] = (v >>> 6) & 1;
    out[base + 7] = v >>> 7;
  }
  for (let i = full << 3; i < s.n; i++) {
    out[i] = (bits[full] >>> (i & 7)) & 1;
  }
  return out;
}

export function decodePhysicalU32Into(s, out) {
  switch (s.p) {
    case PH_NONE:
      return decodeRawU32Into(s.d, s.w, out);
    case PH_FPF256:
      return decodeFastPforInto(s.d, s.w, false, out);
    case PH_FPF128:
      return decodeFastPforInto(s.d, s.w, true, out);
    case PH_BITPACK:
      return unpackBitpackU32Into(s.d, s.w, out);
    case PH_VARINT:
      return decodeVarintU32Into(s.d, s.w, out);
    default:
      throw new Error("u32 phys");
  }
}

function decodePhysicalU64Scratch(s) {
  if (s.rleRuns === -1 && s.p === PH_VARINT) {
    return decodeVarintU64sAllScratch(s.d);
  }
  const out = takeU64(s.w);
  switch (s.p) {
    case PH_NONE:
      return decodeRawU64Into(s.d, s.w, out);
    case PH_BITPACK:
      return unpackBitpackU64Into(s.d, s.w, takeU32(s.w), out);
    case PH_VARINT:
      return decodeVarintU64Into(s.d, s.w, out);
    default:
      throw new Error("u64 phys");
  }
}

function decodeRleWordsU32(s, scratch) {
  const words =
    s.rleRuns === -1 && s.p === PH_VARINT
      ? decodeVarintU32sAllScratch(s.d)
      : decodePhysicalU32Into(s, takeU32(s.w));
  const out = scratch ? takeU32(s.rleVals) : allocU32(s.rleVals);
  return s.rleRuns === -1
    ? decodeRleInterleavedInto(words, s.rleVals, out, false)
    : decodeRleSplitU32Into(words, s.rleRuns, s.rleVals, out);
}

/**
 * Decode an integer stream into `Uint32Array` (`scratch = true` borrows from arena).
 * @param {any} s
 * @param {boolean} [scratch=false]
 * @returns {Uint32Array}
 */
export function decodeStreamU32(s, scratch = false) {
  if (s.l === LG_RLE) return decodeRleWordsU32(s, scratch);
  if (s.l === LG_DELTA_RLE) {
    return zigzagDelta32InPlace(decodeRleWordsU32(s, scratch));
  }
  const out = scratch ? takeU32(s.w) : allocU32(s.w);
  decodePhysicalU32Into(s, out);
  if (s.l === LG_NONE || s.l === LG_DICT) return out;
  if (s.l === LG_DELTA) return zigzagDelta32InPlace(out);
  if (s.l === LG_DELTA2) return zigzagDelta2_32InPlace(out);
  throw new Error("u32 log");
}

/**
 * Decode an integer or vertex stream into `Int32Array` (`scratch = true` borrows from arena).
 * @param {any} s
 * @param {boolean} [scratch=false]
 * @returns {Int32Array}
 */
export function decodeStreamI32(s, scratch = false) {
  if (s.l === LG_MORTON) {
    const words = decodePhysicalU32Into(s, takeU32(s.w));
    return decodeMortonDelta(words, s.mBits, s.mShift);
  }
  if (s.l === LG_RLE) {
    return zigzagI32InPlace(decodeRleWordsU32(s, scratch));
  }
  if (s.l === LG_DELTA_RLE) {
    return asI32(zigzagDelta32InPlace(decodeRleWordsU32(s, scratch)));
  }

  const out = scratch ? takeU32(s.w) : allocU32(s.w);
  decodePhysicalU32Into(s, out);

  if (s.zStep !== NO_Z) {
    if (out.length % 3 !== 0) throw new Error("xyz len");
    if (s.xyzLog === LG_NONE) return zigzagI32InPlace(out);
    if (s.xyzLog === LG_DELTA) return asI32(zigzagDelta32InPlace(out));
    if (s.xyzLog === LG_CW_DELTA) return cwDeltaVec3InPlace(out);
    throw new Error("xyz log");
  }

  switch (s.l) {
    case LG_NONE:
      return zigzagI32InPlace(out);
    case LG_DELTA:
      return asI32(zigzagDelta32InPlace(out));
    case LG_DELTA2:
      return asI32(zigzagDelta2_32InPlace(out));
    case LG_CW_DELTA:
      return cwDeltaVec2InPlace(out);
    case LG_CW_DELTA2:
      return cwDelta2Vec2InPlace(out);
    default:
      throw new Error("i32 log");
  }
}

export function decodeStreamI8(s) {
  const i32s = decodeStreamI32(s, true);
  const out = allocI8(i32s.length);
  for (let i = 0; i < i32s.length; i++) {
    const v = i32s[i];
    if (v < -128 || v > 127) throw new RangeError("i8");
    out[i] = v;
  }
  return out;
}

export function decodeStreamU8(s) {
  const u32s = decodeStreamU32(s, true);
  const out = allocU8(u32s.length);
  for (let i = 0; i < u32s.length; i++) {
    const v = u32s[i];
    if (v > 255) throw new RangeError("u8");
    out[i] = v;
  }
  return out;
}

function decodeRleWordsU64(s, owned) {
  const words = decodePhysicalU64Scratch(s);
  const out = owned ? allocU64(s.rleVals) : takeU64(s.rleVals);
  return s.rleRuns === -1
    ? decodeRleInterleavedInto(words, s.rleVals, out, true)
    : decodeRleSplitU64Into(words, s.rleRuns, s.rleVals, out);
}

function decode64Core(s, signed, owned) {
  let words;
  if (s.l === LG_RLE || s.l === LG_DELTA_RLE) {
    words = decodeRleWordsU64(s, owned);
    if (s.l === LG_DELTA_RLE) zigzagDelta64InPlace(words);
    else if (signed) return zigzagI64InPlace(words);
    return signed ? asI64(words) : words;
  }
  const phys = decodePhysicalU64Scratch(s);
  if (owned) {
    words = allocU64(s.w);
    words.set(phys);
  } else {
    words = phys;
  }
  if (s.l === LG_NONE || (!signed && s.l === LG_DICT)) {
    return signed ? zigzagI64InPlace(words) : words;
  }
  if (s.l === LG_DELTA) zigzagDelta64InPlace(words);
  else if (s.l === LG_DELTA2) zigzagDelta2_64InPlace(words);
  else throw new Error("64 log");
  return signed ? asI64(words) : words;
}

export const decodeStreamI64 = (s, owned = false) => decode64Core(s, true, owned);
export const decodeStreamU64 = (s, owned = false) => decode64Core(s, false, owned);

export function decodeRawFloatStream(s, isF32) {
  if (s.l !== LG_NONE || s.p !== PH_NONE) throw new Error("float enc");
  return decodeRawFloats(s.d, s.n, isF32);
}

export function decodeFloatColumnValues(dataStream, dictStream, isF32) {
  if (dictStream !== null) {
    const dict = decodeRawFloatStream(dictStream, isF32);
    const codes = decodeStreamU32(dataStream, true);
    const n = codes.length;
    const out = isF32 ? allocF32(n) : allocF64(n);
    const dLen = dict.length;
    const stride = isF32 ? 1 : 2;
    const dBits = new Uint32Array(dict.buffer, dict.byteOffset, dLen * stride);
    const oBits = new Uint32Array(out.buffer, 0, n * stride);
    for (let i = 0; i < n; i++) {
      const c = codes[i];
      if (c >= dLen) throw new RangeError("fdict");
      if (stride === 1) {
        oBits[i] = dBits[c];
      } else {
        oBits[i * 2] = dBits[c * 2];
        oBits[i * 2 + 1] = dBits[c * 2 + 1];
      }
    }
    return out;
  }
  if (dataStream.l === LG_ALP) {
    const offsets =
      dataStream.p === PH_FPF128 || dataStream.p === PH_FPF256
        ? decodePhysicalU32Into(dataStream, takeU32(dataStream.w))
        : decodePhysicalU64Scratch(dataStream);
    return decodeAlpFloats(
      offsets,
      dataStream.alpBase,
      dataStream.alpScale,
      isF32,
    );
  }
  return decodeRawFloatStream(dataStream, isF32);
}
