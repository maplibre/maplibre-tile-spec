import { decodeUtf8 } from "../codecs/binary.js";
import { decodeFrontCoded, decodeFsstBytes } from "../codecs/fsst.js";
import {
  bitmapPopcount,
  normalizePresentBitmap,
  scatterPresent,
} from "../codecs/presence.js";
import {
  FM_BYTES,
  FM_INT,
  FM_STR,
  ST_DATA_FSST,
  ST_DATA_NONE,
  ST_DATA_SINGLE,
  ST_LEN_DICT,
  ST_LEN_SYM,
  ST_LEN_VARBIN,
  ST_OFF_STRING,
  decodeStreamU32,
  parseStream02,
} from "./stream.js";

/**
 * Decode the entire UTF-8 byte corpus in ONE native SIMD `TextDecoder.decode` call,
 * then slice entries in O(1) per string (sharing V8's SlicedString backing store!).
 * Notice `fullStr.length === bLen` iff every byte in `bytes` is pure 1-byte ASCII!
 */
export function sliceStrings(bytes, lengths) {
  const bLen = bytes.length;
  const count = lengths.length;
  const entries = new Array(count);
  let pos = 0;

  if (bLen <= 64) {
    let or = 0;
    for (let i = 0; i < bLen; i++) or |= bytes[i];
    if (or < 128) {
      const fullAscii = bLen === 0 ? "" : String.fromCharCode.apply(null, bytes);
      for (let i = 0; i < count; i++) {
        const next = pos + lengths[i];
        if (next > bLen) throw new RangeError("str eof");
        entries[i] = fullAscii.slice(pos, next);
        pos = next;
      }
      return entries;
    }
  }

  const fullStr = decodeUtf8(bytes);
  if (fullStr.length === bLen) {
    for (let i = 0; i < count; i++) {
      const next = pos + lengths[i];
      if (next > bLen) throw new RangeError("str eof");
      entries[i] = fullStr.slice(pos, next);
      pos = next;
    }
    return entries;
  }

  let u = 0;
  for (let i = 0; i < count; i++) {
    const len = lengths[i];
    if (pos + len > bLen) throw new RangeError("str eof");
    pos += len;
    const u0 = u;
    let rem = len;
    while (rem > 0) {
      const c = fullStr.charCodeAt(u++);
      if (c < 0x80) rem--;
      else if (c < 0x800) rem -= 2;
      else if (c >= 0xd800 && c <= 0xdbff) {
        u++;
        rem -= 4;
      } else {
        rem -= 3;
      }
    }
    if (rem !== 0) throw new TypeError("utf8 split");
    entries[i] = fullStr.slice(u0, u);
  }
  return entries;
}

export function decodeDictEntries(front, rawLengths, blobBytes) {
  if (front === 1) {
    const rebuilt = decodeFrontCoded(rawLengths, blobBytes);
    return sliceStrings(rebuilt.bytes, rebuilt.entryLengths);
  }
  return sliceStrings(blobBytes, rawLengths);
}

export function parseStrings02(r, impliedCount) {
  const layout = r.b[r.p] & 3;
  const leadType =
    layout === 0
      ? ST_LEN_VARBIN
      : layout === 2
        ? ST_LEN_DICT
        : ST_OFF_STRING;
  const leading = parseStream02(r, FM_STR, 4, leadType, impliedCount, layout);
  const next = leading.n;

  if (layout === 0) {
    const data = parseStream02(r, FM_BYTES, 4, ST_DATA_NONE, next);
    return { m: 0, lengths: leading, data, n: next };
  }
  if (layout === 1) {
    const lengths = parseStream02(r, FM_INT, 4, ST_LEN_DICT, next);
    const data = parseStream02(r, FM_BYTES, 4, ST_DATA_SINGLE, next);
    return {
      m: 1,
      offsets: leading,
      lengths,
      data,
      front: data.front,
      n: next,
    };
  }
  if (layout === 2) {
    const symLens = parseStream02(r, FM_INT, 4, ST_LEN_SYM, next);
    const symTbl = parseStream02(r, FM_BYTES, 4, ST_DATA_FSST, next);
    const corpus = parseStream02(r, FM_BYTES, 4, ST_DATA_SINGLE, next);
    return { m: 2, lengths: leading, symLens, symTbl, corpus, n: next };
  }
  const lengths = parseStream02(r, FM_INT, 4, ST_LEN_DICT, next);
  const symLens = parseStream02(r, FM_INT, 4, ST_LEN_SYM, next);
  const symTbl = parseStream02(r, FM_BYTES, 4, ST_DATA_FSST, next);
  const corpus = parseStream02(r, FM_BYTES, 4, ST_DATA_SINGLE, next);
  return {
    m: 3,
    offsets: leading,
    lengths,
    symLens,
    symTbl,
    corpus,
    front: corpus.front,
    n: next,
  };
}

export function decodeStringsDense(raw, ownedCodes = false) {
  if (raw.m === 0) {
    return {
      dict: false,
      values: sliceStrings(raw.data.d, decodeStreamU32(raw.lengths, true)),
    };
  }
  if (raw.m === 2) {
    const expanded = decodeFsstBytes(
      decodeStreamU32(raw.symLens, true),
      raw.symTbl.d,
      raw.corpus.d,
    );
    return {
      dict: false,
      values: sliceStrings(expanded, decodeStreamU32(raw.lengths, true)),
    };
  }
  const blob =
    raw.m === 1
      ? raw.data.d
      : decodeFsstBytes(
          decodeStreamU32(raw.symLens, true),
          raw.symTbl.d,
          raw.corpus.d,
        );
  const entries = decodeDictEntries(
    raw.front | 0,
    decodeStreamU32(raw.lengths, true),
    blob,
  );
  const codes = decodeStreamU32(raw.offsets, !ownedCodes);
  const entryLen = entries.length;
  for (let i = 0; i < codes.length; i++) {
    if (codes[i] >= entryLen) throw new RangeError("dict code");
  }
  return { dict: true, entries, codes };
}

/**
 * Scatter `entries[codes[d++]]` directly into `target` at set bits of `bitmap`
 * without allocating an intermediate `denseStrings` JS array!
 */
function scatterDictPresent(bitmap, count, entries, codes, target) {
  const fullBytes = count >>> 3;
  let d = 0;
  for (let bIdx = 0; bIdx < fullBytes; bIdx++) {
    let byte = bitmap[bIdx];
    if (byte === 0) continue;
    const base = bIdx << 3;
    if (byte === 0xff) {
      target[base] = entries[codes[d]];
      target[base + 1] = entries[codes[d + 1]];
      target[base + 2] = entries[codes[d + 2]];
      target[base + 3] = entries[codes[d + 3]];
      target[base + 4] = entries[codes[d + 4]];
      target[base + 5] = entries[codes[d + 5]];
      target[base + 6] = entries[codes[d + 6]];
      target[base + 7] = entries[codes[d + 7]];
      d += 8;
      continue;
    }
    while (byte !== 0) {
      const lsb = byte & -byte;
      target[base + 31 - Math.clz32(lsb)] = entries[codes[d++]];
      byte ^= lsb;
    }
  }
  const rem = count & 7;
  if (rem > 0) {
    let byte = bitmap[fullBytes] & ((1 << rem) - 1);
    const base = fullBytes << 3;
    while (byte !== 0) {
      const lsb = byte & -byte;
      target[base + 31 - Math.clz32(lsb)] = entries[codes[d++]];
      byte ^= lsb;
    }
  }
}

export function materializeStringProperty(
  name,
  dense,
  presenceBits,
  featureCount,
) {
  const denseLen = dense.dict ? dense.codes.length : dense.values.length;

  if (presenceBits === undefined) {
    if (denseLen !== featureCount) throw new Error("str count");
    let values;
    if (dense.dict) {
      const { entries, codes } = dense;
      values = new Array(featureCount);
      for (let i = 0; i < featureCount; i++) values[i] = entries[codes[i]];
    } else {
      values = dense.values;
    }
    return { name, type: "string", values };
  }

  const pop = bitmapPopcount(presenceBits, featureCount);
  if (pop !== denseLen) throw new Error("str popcount");
  const present = normalizePresentBitmap(presenceBits, featureCount);
  if (present === undefined) {
    let values;
    if (dense.dict) {
      const { entries, codes } = dense;
      values = new Array(featureCount);
      for (let i = 0; i < featureCount; i++) values[i] = entries[codes[i]];
    } else {
      values = dense.values;
    }
    return { name, type: "string", values };
  }

  const values = new Array(featureCount).fill("");
  if (pop > 0) {
    if (dense.dict) {
      scatterDictPresent(
        presenceBits,
        featureCount,
        dense.entries,
        dense.codes,
        values,
      );
    } else {
      scatterPresent(presenceBits, featureCount, dense.values, values);
    }
  }
  return { name, type: "string", values, present };
}

export function decodeSharedDictColumns(shared, featureCount) {
  const blob =
    shared.fsst === 0
      ? shared.data.d
      : decodeFsstBytes(
          decodeStreamU32(shared.symLens, true),
          shared.symTbl.d,
          shared.corpus.d,
        );
  const entries = decodeDictEntries(
    shared.front,
    decodeStreamU32(shared.lengths, true),
    blob,
  );
  const entryLen = entries.length;

  return shared.children.map((ch) => {
    const codes = decodeStreamU32(ch.data, true);
    for (let i = 0; i < codes.length; i++) {
      if (codes[i] >= entryLen) throw new RangeError("sdict code");
    }
    return materializeStringProperty(
      `${shared.prefix}${ch.suffix}`,
      { dict: true, entries, codes },
      ch.bits,
      featureCount,
    );
  });
}

export function stringCodesForMValue(dense) {
  if (dense.dict) {
    return { codes: dense.codes, dictionary: dense.entries };
  }
  const map = new Map();
  const dictionary = [];
  const vals = dense.values;
  const codes = new Uint32Array(vals.length);
  for (let i = 0; i < vals.length; i++) {
    const s = vals[i];
    let c = map.get(s);
    if (c === undefined) {
      c = dictionary.length;
      dictionary.push(s);
      map.set(s, c);
    }
    codes[i] = c;
  }
  return { codes, dictionary };
}
