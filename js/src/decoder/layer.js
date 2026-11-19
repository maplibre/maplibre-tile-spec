import {
  allocF32,
  allocF64,
  allocI8,
  allocI32,
  allocU8,
  allocU32,
  checkScratchBytes,
  cursor,
  readStr,
  readU8,
  readVarintU32,
  readVarintU64,
  resetArena,
  resetBudget,
  subCursor,
  trimArena,
} from "../codecs/binary.js";
import { trimFsstBuffers } from "../codecs/fsst.js";
import { deinterleaveU64 } from "../codecs/morton.js";
import {
  PRES_BITMAP,
  bitmapPopcount,
  isBitSet,
  normalizePresentBitmap,
  readPresenceBits,
  scatterPresent,
} from "../codecs/presence.js";
import { GEOM_MESH, decodeGeometry, vertexIndexAt } from "./geometry.js";
import { parseColumnValues02, parseNested02 } from "./nested.js";
import {
  FM_BYTES,
  FM_INT,
  FM_STR,
  FM_VERTEX,
  HK_BOOL,
  HK_FLOAT,
  HK_INT,
  NO_Z,
  ST_DATA_FSST,
  ST_DATA_NONE,
  ST_DATA_SHARED,
  ST_DATA_SINGLE,
  ST_DATA_VERTEX,
  ST_LEN_DICT,
  ST_LEN_GEOM,
  ST_LEN_PARTS,
  ST_LEN_RINGS,
  ST_LEN_SYM,
  ST_LEN_TRIS,
  ST_LEN_VARBIN,
  ST_OFF_STRING,
  ST_OFF_TREE_CYC,
  ST_OFF_TREE_SYM,
  ST_OFF_VERTEX,
  decodeBoolBitmap,
  decodeBoolValues,
  decodeFloatColumnValues,
  decodeStreamI32,
  decodeStreamI64,
  decodeStreamI8,
  decodeStreamU32,
  decodeStreamU64,
  decodeStreamU8,
  parseStream01,
  parseStream02,
} from "./stream.js";
import {
  decodeSharedDictColumns,
  decodeStringsDense,
  materializeStringProperty,
  stringCodesForMValue,
} from "./strings.js";

const KIND_NAMES = [
  "",
  "",
  "bool",
  "i8",
  "u8",
  "i32",
  "u32",
  "i64",
  "u64",
  "f32",
  "f64",
  "string",
];

function expandScalar(dense, bits, featureCount, alloc) {
  if (bits === undefined) {
    if (dense.length !== featureCount) throw new Error("col count");
    return { values: dense };
  }
  const pop = bitmapPopcount(bits, featureCount);
  if (pop !== dense.length) throw new Error("popcount");
  const present = normalizePresentBitmap(bits, featureCount);
  if (present === undefined) {
    // All bits set: copy if `dense` came from scratch arena, else return directly
    const out = alloc(featureCount);
    out.set(dense);
    return { values: out };
  }
  const values = alloc(featureCount);
  if (pop > 0) scatterPresent(bits, featureCount, dense, values);
  return { values, present };
}

function expand64(denseBig, bits, featureCount) {
  const n = denseBig.length;
  if (bits === undefined) {
    if (n !== featureCount) throw new Error("col64 count");
    const values = allocF64(featureCount);
    for (let i = 0; i < featureCount; i++) values[i] = Number(denseBig[i]);
    return { values };
  }
  const pop = bitmapPopcount(bits, featureCount);
  if (pop !== n) throw new Error("popcount64");
  const present = normalizePresentBitmap(bits, featureCount);
  const values = allocF64(featureCount);
  if (present === undefined) {
    for (let i = 0; i < featureCount; i++) values[i] = Number(denseBig[i]);
    return { values };
  }
  if (pop > 0) {
    checkScratchBytes(pop * 8);
    const denseF64 = new Float64Array(pop);
    for (let i = 0; i < pop; i++) denseF64[i] = Number(denseBig[i]);
    scatterPresent(bits, featureCount, denseF64, values);
  }
  return { values, present };
}

function decodeId(is64, bits, stream, featureCount) {
  if (is64) return expand64(decodeStreamU64(stream, false), bits, featureCount);
  const hasSparse = bits !== undefined;
  const u32s = decodeStreamU32(stream, true);
  if (!hasSparse) {
    if (u32s.length !== featureCount) throw new Error("id count");
    const values = allocF64(featureCount);
    for (let i = 0; i < featureCount; i++) values[i] = u32s[i];
    return { values };
  }
  return expandScalar(u32s, bits, featureCount, allocF64);
}

function decodeValueProp(name, dt, s, dictStream, bits, featureCount) {
  if (dt === 11) {
    return materializeStringProperty(
      name,
      decodeStringsDense(s, false),
      bits,
      featureCount,
    );
  }
  if (dt === 7) {
    return {
      name,
      type: "i64",
      ...expand64(decodeStreamI64(s, false), bits, featureCount),
    };
  }
  if (dt === 8) {
    return {
      name,
      type: "u64",
      ...expand64(decodeStreamU64(s, false), bits, featureCount),
    };
  }

  const scratch = bits !== undefined;
  let dense;
  let alloc;
  switch (dt) {
    case 2:
      dense = decodeBoolValues(s);
      alloc = allocU8;
      break;
    case 3:
      dense = decodeStreamI8(s);
      alloc = allocI8;
      break;
    case 4:
      dense = decodeStreamU8(s);
      alloc = allocU8;
      break;
    case 5:
      dense = decodeStreamI32(s, scratch);
      alloc = allocI32;
      break;
    case 6:
      dense = decodeStreamU32(s, scratch);
      alloc = allocU32;
      break;
    case 9:
      dense = decodeFloatColumnValues(s, dictStream, true);
      alloc = allocF32;
      break;
    default:
      dense = decodeFloatColumnValues(s, dictStream, false);
      alloc = allocF64;
      break;
  }
  return {
    name,
    type: KIND_NAMES[dt],
    ...expandScalar(dense, bits, featureCount, alloc),
  };
}

// Valid v1 column type bitmask over 0..31: 0..4, 10..11, 16..30
const V1_VALID_MASK = 0x7fff0c1f;

function decodeLayer01(r) {
  resetArena();
  const name = readStr(r);
  if (name.length === 0) throw new Error("layer name");
  const extent = readVarintU32(r);
  if (extent === 0) throw new Error("extent");
  const colCount = readVarintU32(r);
  if (r.e - r.p < colCount) throw new RangeError("v1 cols eof");

  const colsMeta = new Array(colCount);
  let geomCols = 0;
  let idCols = 0;

  for (let i = 0; i < colCount; i++) {
    const typ = readU8(r);
    if (typ > 30 || ((V1_VALID_MASK >>> typ) & 1) === 0) {
      throw new Error("v1 col");
    }
    const colName = typ >= 10 ? readStr(r) : "";
    let children = null;
    if (typ === 30) {
      const cc = readVarintU32(r);
      if (r.e - r.p < cc) throw new RangeError("v1 sdict eof");
      children = new Array(cc);
      for (let c = 0; c < cc; c++) {
        const ct = readU8(r);
        if (ct > 30 || ((V1_VALID_MASK >>> ct) & 1) === 0) {
          throw new Error("v1 child");
        }
        children[c] = { typ: ct, name: ct >= 10 ? readStr(r) : "" };
      }
    }
    if (typ === 4) geomCols++;
    else if (typ <= 3) idCols++;
    colsMeta[i] = { typ, name: colName, children };
  }
  if (geomCols > 1 || idCols > 1) throw new Error("v1 dup id/geom");

  let rawId = null;
  let geomTypesStream = null;
  let geomItems = null;
  const rawProps = [];

  const optBits = (typ) =>
    (typ & 1) === 0
      ? undefined
      : decodeBoolBitmap(parseStream01(r, HK_BOOL, true));

  for (let i = 0; i < colCount; i++) {
    const col = colsMeta[i];
    const { typ, name: colName } = col;
    if (typ <= 3) {
      const bits = optBits(typ);
      rawId = { is64: typ >= 2, bits, s: parseStream01(r, HK_INT) };
    } else if (typ === 4) {
      const sc = readVarintU32(r);
      if (sc === 0) throw new Error("v1 geom 0");
      if (r.e - r.p < sc) throw new RangeError("v1 geom eof");
      geomTypesStream = parseStream01(r, HK_INT);
      geomItems = new Array(sc - 1);
      for (let k = 0; k < sc - 1; k++) geomItems[k] = parseStream01(r, HK_INT);
    } else if (typ <= 11) {
      const bits = optBits(typ);
      rawProps.push({
        dt: 2,
        name: colName,
        bits,
        s: parseStream01(r, HK_BOOL, true),
      });
    } else if (typ <= 23) {
      const dtMap = [5, 5, 6, 6, 7, 7, 8, 8];
      const bits = optBits(typ);
      rawProps.push({
        dt: dtMap[typ - 16],
        name: colName,
        bits,
        s: parseStream01(r, HK_INT),
      });
    } else if (typ <= 27) {
      const bits = optBits(typ);
      rawProps.push({
        dt: typ <= 25 ? 9 : 10,
        name: colName,
        bits,
        s: parseStream01(r, HK_FLOAT),
      });
    } else if (typ <= 29) {
      let sc = readVarintU32(r);
      const bits = optBits(typ);
      if (bits !== undefined) {
        if (sc === 0) throw new Error("v1 str sc");
        sc--;
      }
      if (sc < 2 || sc > 5) throw new Error("v1 str sc");
      const s = new Array(sc);
      for (let k = 0; k < sc; k++) s[k] = parseStream01(r, HK_INT);
      const strRaw =
        sc === 2
          ? { m: 0, lengths: s[0], data: s[1] }
          : sc === 3
            ? { m: 1, lengths: s[0], offsets: s[1], data: s[2], front: 0 }
            : sc === 4
              ? {
                  m: 2,
                  symLens: s[0],
                  symTbl: s[1],
                  lengths: s[2],
                  corpus: s[3],
                }
              : {
                  m: 3,
                  symLens: s[0],
                  symTbl: s[1],
                  lengths: s[2],
                  corpus: s[3],
                  offsets: s[4],
                  front: 0,
                };
      rawProps.push({ dt: 11, name: colName, bits, s: strRaw });
    } else {
      // SharedDict (30)
      const streamCount = readVarintU32(r);
      const ds = [];
      while (ds.length < streamCount) {
        const st = parseStream01(r, HK_INT);
        ds.push(st);
        if (st.t === ST_DATA_SINGLE || st.t === ST_DATA_SHARED) break;
        if (ds.length >= 5) throw new Error("v1 sdict");
      }
      const children = new Array(col.children.length);
      let optCount = 0;
      for (let c = 0; c < col.children.length; c++) {
        const ch = col.children[c];
        const cSc = readVarintU32(r);
        const cBits = optBits(ch.typ);
        const isOpt = cBits !== undefined ? 1 : 0;
        optCount += isOpt;
        if (cSc >= isOpt && cSc - isOpt !== 1) throw new Error("v1 sdict ch");
        children[c] = {
          suffix: ch.name,
          bits: cBits,
          data: parseStream01(r, HK_INT),
        };
      }
      const exp = ds.length + children.length + optCount;
      if (streamCount !== exp && streamCount !== exp + 1) {
        throw new Error("v1 sdict sc");
      }
      if (ds.length !== 2 && ds.length !== 4) throw new Error("v1 sdict ds");
      const shared =
        ds.length === 2
          ? {
              prefix: colName,
              front: 0,
              fsst: 0,
              lengths: ds[0],
              data: ds[1],
              children,
            }
          : {
              prefix: colName,
              front: 0,
              fsst: 1,
              symLens: ds[0],
              symTbl: ds[1],
              lengths: ds[2],
              corpus: ds[3],
              children,
            };
      rawProps.push({ dt: 30, shared });
    }
  }

  if (r.p !== r.e) throw new Error("v1 trailing");
  if (geomItems === null) throw new Error("v1 no geom");

  const geometry = decodeGeometry(-1, 0, geomTypesStream, false, geomItems);
  const featureCount = geometry.types.length;
  const properties = [];
  for (let i = 0; i < rawProps.length; i++) {
    const p = rawProps[i];
    if (p.dt === 30) {
      properties.push(...decodeSharedDictColumns(p.shared, featureCount));
    } else {
      properties.push(
        decodeValueProp(p.name, p.dt, p.s, null, p.bits, featureCount),
      );
    }
  }

  const layer = {
    name,
    extent,
    version: 1,
    featureCount,
    geometry,
    properties,
  };
  if (rawId !== null) {
    layer.ids = decodeId(rawId.is64, rawId.bits, rawId.s, featureCount);
  }
  return layer;
}

function readColPresence02(r, high, sharedBitfields, featureCount) {
  if (high === 0x00) return undefined;
  if (high < 0x40) {
    const code = (high >>> 4) - 1;
    if (code > 2) throw new Error("v2 pres");
    return readPresenceBits(r, featureCount, code);
  }
  const idx = (high - 0x40) >>> 4;
  if (idx >= sharedBitfields.length) throw new Error("v2 spres");
  return sharedBitfields[idx];
}

function decodeMValue02(name, scope, bits, cv, geometry, featureCount) {
  const present = normalizePresentBitmap(bits, featureCount);
  let expected = 0;
  if (present === undefined) {
    if (scope === "triangle") {
      const triOff = geometry.triangleOffsets;
      expected = triOff ? triOff[featureCount] : 0;
    } else {
      expected = vertexIndexAt(geometry, featureCount);
    }
  } else {
    for (let f = 0; f < featureCount; f++) {
      if (!isBitSet(present, f)) continue;
      if (scope === "triangle") {
        const triOff = geometry.triangleOffsets;
        expected += triOff ? triOff[f + 1] - triOff[f] : 0;
      } else {
        expected += vertexIndexAt(geometry, f + 1) - vertexIndexAt(geometry, f);
      }
    }
  }
  let col;
  let actual;

  if (cv.dt === 11) {
    const { codes, dictionary } = stringCodesForMValue(
      decodeStringsDense(cv.s, true),
    );
    actual = codes.length;
    col = { name, scope, type: "string", codes, dictionary };
  } else if (cv.dt === 7 || cv.dt === 8) {
    const big =
      cv.dt === 7 ? decodeStreamI64(cv.s, false) : decodeStreamU64(cv.s, false);
    actual = big.length;
    const values = new Float64Array(actual);
    for (let i = 0; i < actual; i++) values[i] = Number(big[i]);
    col = { name, scope, type: KIND_NAMES[cv.dt], values };
  } else {
    let values;
    switch (cv.dt) {
      case 2:
        values = decodeBoolValues(cv.s);
        break;
      case 3:
        values = decodeStreamI8(cv.s);
        break;
      case 4:
        values = decodeStreamU8(cv.s);
        break;
      case 5:
        values = decodeStreamI32(cv.s, false);
        break;
      case 6:
        values = decodeStreamU32(cv.s, false);
        break;
      case 9:
        values = decodeFloatColumnValues(cv.s, cv.d, true);
        break;
      default:
        values = decodeFloatColumnValues(cv.s, cv.d, false);
        break;
    }
    actual = values.length;
    col = { name, scope, type: KIND_NAMES[cv.dt], values };
  }

  if (actual !== expected) throw new Error("mval len");
  if (present !== undefined) col.present = present;
  return col;
}

// GeoLayout bitmasks over 0..14:
// hasGeo:   2,3, 6,7, 10,11, 13, 14 -> 0b110_1100_1100_1100 = 0x6ccc
// hasParts: 4..11, 13               -> 0b010_1111_1111_0000 = 0x2ff0
// hasRings: 8..11, 13               -> 0b010_1111_0000_0000 = 0x2f00
const GEO_HAS_GEO = 0x6ccc;
const GEO_HAS_PARTS = 0x2ff0;
const GEO_HAS_RINGS = 0x2f00;

function decodeLayer02(r, keepNested) {
  resetArena();
  const name = readStr(r);
  if (name.length === 0) throw new Error("v2 name");

  const hdr = readU8(r);
  const hasMValues = (hdr & 0x80) !== 0;
  const uniNib = (hdr & 0x70) >>> 4;
  if (uniNib > 7) throw new Error("v2 hdr");
  const uniformType = uniNib === 0 ? -1 : uniNib - 1;
  const extent = 1 << ((hdr & 0x0f) + 6);

  const featureCount = readVarintU32(r);

  const layout = readU8(r);
  const sharedCoded = (layout & 0x80) !== 0;
  const sharedCount = (layout & 0x70) >>> 4;
  const geoLayout = layout & 0x0f;
  if ((sharedCoded && sharedCount === 0) || geoLayout > 0x0e) {
    throw new Error("v2 layout");
  }
  if (
    hasMValues &&
    (geoLayout === 0 || geoLayout === 1 || geoLayout === 0x0c)
  ) {
    throw new Error("v2 mval layout");
  }
  if ((uniformType === GEOM_MESH) !== (geoLayout === 0x0e)) {
    throw new Error("v2 mesh layout");
  }

  const sharedBitfields = new Array(sharedCount);
  for (let i = 0; i < sharedCount; i++) {
    let coding = PRES_BITMAP;
    if (sharedCoded) {
      const cb = readU8(r);
      if ((cb & 0x8f) !== 0) throw new Error("v2 spres byte");
      coding = (cb & 0x70) >>> 4;
      if (coding > 2) throw new Error("v2 spres code");
    }
    sharedBitfields[i] = readPresenceBits(r, featureCount, coding);
  }

  // Parse and immediately decode geometry!
  const typesStream =
    uniformType >= 0
      ? null
      : parseStream02(r, FM_INT, 4, ST_LEN_VARBIN, featureCount);
  const items = [];
  if (((GEO_HAS_GEO >>> geoLayout) & 1) !== 0) {
    items.push(parseStream02(r, FM_INT, 4, ST_LEN_GEOM, featureCount));
  }
  if (((GEO_HAS_PARTS >>> geoLayout) & 1) !== 0) {
    items.push(parseStream02(r, FM_INT, 4, ST_LEN_PARTS, featureCount));
  }
  if (((GEO_HAS_RINGS >>> geoLayout) & 1) !== 0) {
    items.push(parseStream02(r, FM_INT, 4, ST_LEN_RINGS, featureCount));
  }
  if (geoLayout >= 0x0c) {
    items.push(parseStream02(r, FM_INT, 4, ST_LEN_TRIS, featureCount));
    items.push(parseStream02(r, FM_INT, 4, ST_OFF_TREE_SYM, featureCount));
    items.push(parseStream02(r, FM_INT, 4, ST_OFF_TREE_CYC, featureCount));
  }
  const vtxStream = parseStream02(
    r,
    FM_VERTEX,
    4,
    ST_DATA_VERTEX,
    featureCount,
  );
  if (geoLayout === 0x0e && vtxStream.zStep === NO_Z) {
    throw new Error("v2 mesh z");
  }
  if (geoLayout !== 0x0e && vtxStream.zStep !== NO_Z && vtxStream.zDatum === 1) {
    throw new Error("v2 ground z");
  }
  items.push(vtxStream);
  if (geoLayout <= 0x0b && (geoLayout & 1) === 1) {
    items.push(parseStream02(r, FM_INT, 4, ST_OFF_VERTEX, featureCount));
  }

  // Decode geometry right away and reclaim geometry scratch arena for columns!
  const geometry = decodeGeometry(
    uniformType,
    featureCount,
    typesStream,
    true,
    items,
  );
  resetArena();

  let columnCount;
  let mValueCount = 0;
  if (!hasMValues) {
    columnCount = readVarintU32(r);
  } else {
    [columnCount, mValueCount] = deinterleaveU64(readVarintU64(r));
    if (mValueCount === 0) throw new Error("v2 empty mvals");
  }
  if (r.e - r.p < columnCount) throw new RangeError("v2 cols eof");

  let ids;
  const properties = [];
  const rawNested = keepNested ? [] : null;
  const seen = new Set();
  const reg = (n) => {
    if (seen.has(n)) throw new Error("dup col");
    seen.add(n);
  };

  for (let i = 0; i < columnCount; i++) {
    resetArena();
    const tb = readU8(r);
    const high = tb & 0xf0;
    const data = tb & 0x0f;

    if (data === 0x0f) {
      if (high !== 0x00 && high !== 0x10) throw new Error("v2 sdict");
      const prefix = readStr(r);
      const cc = readVarintU32(r);
      if (r.e - r.p < cc) throw new RangeError("v2 sdict eof");
      let shared;
      if (high === 0x00) {
        const lengths = parseStream02(r, FM_INT, 4, ST_LEN_DICT, -1);
        const blob = parseStream02(r, FM_BYTES, 4, ST_DATA_SHARED, -1);
        shared = {
          prefix,
          front: blob.front,
          fsst: 0,
          lengths,
          data: blob,
          children: new Array(cc),
        };
      } else {
        const lengths = parseStream02(r, FM_INT, 4, ST_LEN_DICT, -1);
        const symLens = parseStream02(r, FM_INT, 4, ST_LEN_SYM, -1);
        const symTbl = parseStream02(r, FM_BYTES, 4, ST_DATA_FSST, -1);
        const corpus = parseStream02(r, FM_BYTES, 4, ST_DATA_SHARED, -1);
        shared = {
          prefix,
          front: corpus.front,
          fsst: 1,
          lengths,
          symLens,
          symTbl,
          corpus,
          children: new Array(cc),
        };
      }
      for (let c = 0; c < cc; c++) {
        const cb = readU8(r);
        if ((cb & 0x0f) !== 0x0b) throw new Error("v2 sdict ch");
        const suffix = readStr(r);
        reg(`${prefix}${suffix}`);
        const cBits = readColPresence02(
          r,
          cb & 0xf0,
          sharedBitfields,
          featureCount,
        );
        const cCount =
          cBits === undefined
            ? featureCount
            : bitmapPopcount(cBits, featureCount);
        const cData = parseStream02(r, FM_STR, 4, ST_OFF_STRING, cCount, 1);
        shared.children[c] = { suffix, bits: cBits, data: cData };
      }
      properties.push(...decodeSharedDictColumns(shared, featureCount));
      continue;
    }

    const colName = data >= 2 ? readStr(r) : "";
    const bits = readColPresence02(r, high, sharedBitfields, featureCount);
    const dCount =
      bits === undefined ? featureCount : bitmapPopcount(bits, featureCount);

    if (data <= 1) {
      if (ids !== undefined) throw new Error("dup id");
      const s = parseStream02(
        r,
        FM_INT,
        data === 1 ? 8 : 4,
        ST_DATA_NONE,
        dCount,
      );
      ids = decodeId(data === 1, bits, s, featureCount);
    } else if (data <= 11) {
      reg(colName);
      const cv = parseColumnValues02(r, data, dCount);
      properties.push(
        decodeValueProp(colName, data, cv.s, cv.d, bits, featureCount),
      );
    } else {
      reg(colName);
      const parsed = parseNested02(r, colName, bits, data, dCount);
      if (rawNested !== null) rawNested.push(parsed);
    }
  }

  const mValues = new Array(mValueCount);
  if (hasMValues) {
    if (r.e - r.p < mValueCount) throw new RangeError("v2 mvals eof");
    const isMesh = geoLayout === 0x0e;
    for (let m = 0; m < mValueCount; m++) {
      resetArena();
      const tb = readU8(r);
      const data = tb & 0x0f;
      if (data < 2 || data > 11) throw new Error("v2 mval type");
      let scope = "vertex";
      if (isMesh) {
        const sb = readU8(r);
        if (sb === 1) scope = "triangle";
        else if (sb !== 0) throw new Error("v2 mval scope");
      }
      const mName = readStr(r);
      reg(mName);
      const bits = readColPresence02(
        r,
        tb & 0xf0,
        sharedBitfields,
        featureCount,
      );
      const cv = parseColumnValues02(r, data, -1);
      mValues[m] = decodeMValue02(
        mName,
        scope,
        bits,
        cv,
        geometry,
        featureCount,
      );
    }
  }

  if (r.p !== r.e) throw new Error("v2 trailing");

  const layer = {
    name,
    extent,
    version: 2,
    featureCount,
    geometry,
    properties,
    mValues,
  };
  if (ids !== undefined) layer.ids = ids;
  if (rawNested !== null && rawNested.length > 0) layer._rawNested = rawNested;
  return layer;
}

export function decodeRawTileLayers(data, options = {}) {
  if (!(data instanceof Uint8Array)) {
    throw new TypeError("expected Uint8Array");
  }
  resetBudget(options.maxBytes);
  const filterSet =
    options.layers !== undefined ? new Set(options.layers) : null;
  const keepNested = options.keepNested === true;

  const r = cursor(data);
  const layers = [];

  try {
    while (r.p < r.e) {
      const size = readVarintU32(r);
      const tag = readU8(r);
      if (size === 0) throw new Error("zero layer size");
      const body = subCursor(r, size - 1);
      if (tag !== 1 && tag !== 2) continue;

      if (filterSet !== null) {
        const peek = cursor(body.b, body.p, body.e);
        const nm = readStr(peek);
        if (nm.length === 0) throw new Error("layer name");
        if (!filterSet.has(nm)) continue;
      }

      layers.push(
        tag === 1 ? decodeLayer01(body) : decodeLayer02(body, keepNested),
      );
    }
  } finally {
    trimArena();
    trimFsstBuffers();
  }

  return layers;
}
