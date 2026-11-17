import { peekU8, readStr, readU8, readVarintU32 } from "../codecs/binary.js";
import {
  PRES_BITMAP,
  PRES_RUNS,
  PRES_SPARSE,
  isBitSet,
  popcountPresencePayload,
} from "../codecs/presence.js";
import {
  FM_BOOL,
  FM_FLOAT,
  FM_INT,
  HAS_EXPLICIT_COUNT,
  LG_DICT,
  LG_NONE,
  LG_RUNS,
  ST_DATA_NONE,
  ST_DATA_SINGLE,
  ST_LEN_NESTED,
  ST_PRESENT,
  decodeBoolValues,
  decodeFloatColumnValues,
  decodeStreamI32,
  decodeStreamI64,
  decodeStreamI8,
  decodeStreamU32,
  decodeStreamU64,
  decodeStreamU8,
  parseStream02,
} from "./stream.js";
import { decodeStringsDense, parseStrings02 } from "./strings.js";

const MAX_NESTED_DEPTH = 8;
const SHAPES_BIT = 0x40;

/**
 * Parse a v2 value column/leaf stream set (`dataType` in `2..11`).
 * `impliedCount < 0` means `Count02::Explicit`.
 */
export function parseColumnValues02(r, dataType, impliedCount) {
  if (impliedCount < 0 && (peekU8(r) & HAS_EXPLICIT_COUNT) === 0) {
    throw new Error("explicit count");
  }
  if (dataType === 2) {
    return { dt: 2, s: parseStream02(r, FM_BOOL, 4, ST_DATA_NONE, impliedCount), d: null };
  }
  if (dataType <= 6) {
    return { dt: dataType, s: parseStream02(r, FM_INT, 4, ST_DATA_NONE, impliedCount), d: null };
  }
  if (dataType <= 8) {
    return { dt: dataType, s: parseStream02(r, FM_INT, 8, ST_DATA_NONE, impliedCount), d: null };
  }
  if (dataType <= 10) {
    const width = dataType === 9 ? 4 : 8;
    const s = parseStream02(r, FM_FLOAT, width, ST_DATA_NONE, impliedCount);
    const d =
      s.l === LG_DICT
        ? parseStream02(r, FM_FLOAT, width, ST_DATA_SINGLE, s.n)
        : null;
    return { dt: dataType, s, d };
  }
  if (dataType === 11) {
    return { dt: 11, s: parseStrings02(r, impliedCount), d: null };
  }
  throw new Error("col type");
}

function reqExplicit(r, count) {
  if (count < 0 && (peekU8(r) & HAS_EXPLICIT_COUNT) === 0) {
    throw new Error("nested explicit");
  }
}

function parseNodeByte(b) {
  const high = b & 0xf0;
  const data = b & 0x0f;
  const shapes = (high & SHAPES_BIT) !== 0;
  const pres = high & ~SHAPES_BIT;
  if ((pres !== 0 && pres !== 0x10) || data <= 1 || data > 14) {
    throw new Error("node type");
  }
  return { hasPres: pres === 0x10, shapes, data };
}

function presPopcount(s) {
  const coding =
    s.l === LG_NONE ? PRES_BITMAP : s.l === LG_RUNS ? PRES_RUNS : PRES_SPARSE;
  return popcountPresencePayload(s.d, s.n, coding);
}

function parseRowShapes(r, width, rowsImplied) {
  reqExplicit(r, -1);
  const tblStream = parseStream02(r, FM_BOOL, 4, ST_PRESENT, -1);
  reqExplicit(r, -1);
  const idsStream = parseStream02(r, FM_INT, 4, ST_DATA_NONE, -1);

  const table = decodeBoolValues(tblStream);
  const ids = decodeStreamU32(idsStream, false);

  if (width === 0 || table.length % width !== 0) throw new Error("shapes tbl");
  const shapeCount = table.length / width;
  for (let i = 0; i < ids.length; i++) {
    if (ids[i] >= shapeCount) throw new RangeError("shape id");
  }
  if (rowsImplied >= 0 && ids.length !== rowsImplied) {
    throw new Error("shape rows");
  }
  return { table, width, ids };
}

function shapePresentCount(sh, key) {
  const { table, width, ids } = sh;
  let c = 0;
  for (let r = 0; r < ids.length; r++) c += table[ids[r] * width + key];
  return c;
}

function parseInterior(r, kind, presence, count, perRow, depth) {
  if (kind === 12) {
    // Struct
    const fieldCount = readVarintU32(r);
    if (fieldCount === 0) throw new Error("empty struct");
    if (r.e - r.p < fieldCount) throw new RangeError("struct eof");
    const shapes = perRow ? parseRowShapes(r, fieldCount, count) : null;
    const fields = new Array(fieldCount);
    for (let i = 0; i < fieldCount; i++) {
      let childCount = count;
      if (shapes !== null) {
        if (parseNodeByte(peekU8(r)).hasPres) throw new Error("shaped child pres");
        childCount = count < 0 ? -1 : shapePresentCount(shapes, i);
      }
      const field = parseNode(r, true, childCount, depth + 1);
      for (let j = 0; j < i; j++) {
        if (fields[j].name === field.name) throw new Error("dup field");
      }
      fields[i] = field;
    }
    return { k: 12, presence, shapes, fields };
  }

  if (kind === 13) {
    // List
    if (perRow) throw new Error("shaped list");
    reqExplicit(r, count);
    const lengths = parseStream02(r, FM_INT, 4, ST_LEN_NESTED, count);
    const element = parseNode(r, false, -1, depth + 1).node;
    return { k: 13, presence, lengths, element };
  }

  // Map (14)
  let keysRaw;
  let lengths = null;
  let shapes = null;
  if (perRow) {
    reqExplicit(r, -1);
    keysRaw = parseStrings02(r, -1);
    shapes = parseRowShapes(r, keysRaw.n, count);
  } else {
    reqExplicit(r, count);
    lengths = parseStream02(r, FM_INT, 4, ST_LEN_NESTED, count);
    reqExplicit(r, -1);
    keysRaw = parseStrings02(r, -1);
  }
  const value = parseNode(r, false, -1, depth + 1).node;
  return { k: 14, presence, lengths, shapes, keysRaw, value };
}

function parseNode(r, named, parentCount, depth) {
  if (depth > MAX_NESTED_DEPTH) throw new Error("nested depth");
  const typ = parseNodeByte(readU8(r));
  const name = named ? readStr(r) : "";
  const nodeCount = typ.data === 13 || typ.data === 14 ? -1 : parentCount;

  let presence = null;
  if (typ.hasPres) {
    reqExplicit(r, nodeCount);
    presence = parseStream02(r, FM_BOOL, 4, ST_PRESENT, nodeCount);
  }
  const bodyCount =
    nodeCount < 0
      ? -1
      : presence !== null
        ? presPopcount(presence)
        : nodeCount;

  if (typ.data <= 11) {
    if (typ.shapes) throw new Error("shaped leaf");
    reqExplicit(r, bodyCount);
    return {
      name,
      node: { k: 0, presence, cv: parseColumnValues02(r, typ.data, bodyCount) },
    };
  }
  return {
    name,
    node: parseInterior(r, typ.data, presence, bodyCount, typ.shapes, depth),
  };
}

/**
 * Parse (and advance cursor past) a v2 nested column (`rootData` in `12..14`).
 */
export function parseNested02(r, name, presenceBits, rootData, valueCount) {
  const count = rootData === 12 ? valueCount : -1;
  let perRow = false;
  if (r.p < r.e && peekU8(r) === SHAPES_BIT) {
    readU8(r);
    perRow = true;
  }
  const root = parseInterior(r, rootData, null, count, perRow, 1);
  return { name, presenceBits, valueCount, root };
}

// ── Materialize parsed nested columns into JSON (imported ONLY by geojson.js!) ──

export function floatToJsonValue(v, isF32) {
  if (Number.isFinite(v)) return v;
  const p = isF32 ? "f32" : "f64";
  return Number.isNaN(v)
    ? `${p}::NAN`
    : v > 0
      ? `${p}::INFINITY`
      : `${p}::NEG_INFINITY`;
}

function decodeLeafFlat(cv) {
  switch (cv.dt) {
    case 2:
      return { dt: 2, v: decodeBoolValues(cv.s) };
    case 3:
      return { dt: 3, v: decodeStreamI8(cv.s) };
    case 4:
      return { dt: 4, v: decodeStreamU8(cv.s) };
    case 5:
      return { dt: 5, v: decodeStreamI32(cv.s, false) };
    case 6:
      return { dt: 6, v: decodeStreamU32(cv.s, false) };
    case 7:
      return { dt: 7, v: decodeStreamI64(cv.s, true) };
    case 8:
      return { dt: 8, v: decodeStreamU64(cv.s, true) };
    case 9:
      return { dt: 9, v: decodeFloatColumnValues(cv.s, cv.d, true) };
    case 10:
      return { dt: 10, v: decodeFloatColumnValues(cv.s, cv.d, false) };
    default: {
      const sd = decodeStringsDense(cv.s, false);
      const strings = sd.dict
        ? Array.from(sd.codes, (c) => sd.entries[c])
        : sd.values;
      return { dt: 11, v: strings };
    }
  }
}

function shapeHeld(sh, key) {
  const { table, width, ids } = sh;
  const out = new Uint8Array(ids.length);
  for (let r = 0; r < ids.length; r++) out[r] = table[ids[r] * width + key];
  return out;
}

function shapeLengthsAndCodes(sh) {
  const { table, width, ids } = sh;
  const lengths = new Uint32Array(ids.length);
  let total = 0;
  for (let r = 0; r < ids.length; r++) {
    let len = 0;
    const base = ids[r] * width;
    for (let k = 0; k < width; k++) len += table[base + k];
    lengths[r] = len;
    total += len;
  }
  const codes = new Uint32Array(total);
  let pos = 0;
  for (let r = 0; r < ids.length; r++) {
    const base = ids[r] * width;
    for (let k = 0; k < width; k++) if (table[base + k] === 1) codes[pos++] = k;
  }
  return { lengths, codes };
}

function buildOffsets(lengths) {
  const off = new Uint32Array(lengths.length + 1);
  let total = 0;
  for (let i = 0; i < lengths.length; i++) {
    total += lengths[i];
    if (total > 0xffffffff) throw new RangeError("overflow");
    off[i + 1] = total;
  }
  return off;
}

function decodeTree(raw, overridePres) {
  const presence = overridePres ?? (raw.presence ? decodeBoolValues(raw.presence) : null);
  if (raw.k === 0) {
    return { k: 0, presence, flat: decodeLeafFlat(raw.cv) };
  }
  if (raw.k === 12) {
    const fields = raw.fields.map((f, i) => ({
      name: f.name,
      node: decodeTree(f.node, raw.shapes ? shapeHeld(raw.shapes, i) : null),
    }));
    return { k: 12, presence, fields };
  }
  if (raw.k === 13) {
    const lengths = decodeStreamU32(raw.lengths, false);
    return {
      k: 13,
      presence,
      offsets: buildOffsets(lengths),
      element: decodeTree(raw.element, null),
    };
  }
  const kd = decodeStringsDense(raw.keysRaw, false);
  const keys = kd.dict ? Array.from(kd.codes, (c) => kd.entries[c]) : kd.values;
  let lengths;
  let codes = null;
  if (raw.shapes) {
    const sc = shapeLengthsAndCodes(raw.shapes);
    lengths = sc.lengths;
    codes = sc.codes;
  } else {
    lengths = decodeStreamU32(raw.lengths, false);
  }
  return {
    k: 14,
    presence,
    offsets: buildOffsets(lengths),
    keys,
    codes,
    value: decodeTree(raw.value, null),
  };
}

function parentCount(node) {
  if (node.presence !== null) return node.presence.length;
  if (node.k === 0) return node.flat.v.length;
  if (node.k === 12) return structPresent(node);
  return node.offsets.length - 1;
}

function structPresent(node) {
  if (node.presence !== null) {
    let c = 0;
    for (let i = 0; i < node.presence.length; i++) c += node.presence[i];
    return c;
  }
  return node.fields.length === 0 ? 0 : parentCount(node.fields[0].node);
}

function checkTreeCounts(node) {
  if (node.k === 0) return;
  if (node.k === 12) {
    const p = structPresent(node);
    for (const f of node.fields) {
      if (parentCount(f.node) !== p) throw new Error("struct count");
      checkTreeCounts(f.node);
    }
    return;
  }
  const entries = node.offsets[node.offsets.length - 1];
  if (node.k === 13) {
    if (parentCount(node.element) !== entries) throw new Error("list count");
    checkTreeCounts(node.element);
    return;
  }
  const keyLen = node.codes ? node.codes.length : node.keys.length;
  if (keyLen !== entries || parentCount(node.value) !== entries) {
    throw new Error("map count");
  }
  checkTreeCounts(node.value);
}

function leafJsonAt(flat, idx) {
  const val = flat.v[idx];
  if (flat.dt === 2) return val === 1;
  if (flat.dt === 7 || flat.dt === 8) return Number(val);
  if (flat.dt === 9) return floatToJsonValue(val, true);
  if (flat.dt === 10) return floatToJsonValue(val, false);
  return val;
}

function evalRow(node, row) {
  let own = row;
  if (node.presence !== null) {
    if (row >= node.presence.length || node.presence[row] === 0) return undefined;
    own = 0;
    for (let i = 0; i < row; i++) own += node.presence[i];
  }
  if (node.k === 0) return leafJsonAt(node.flat, own);
  if (node.k === 12) {
    const obj = {};
    const sorted = [...node.fields].sort((a, b) =>
      a.name < b.name ? -1 : a.name > b.name ? 1 : 0,
    );
    for (const f of sorted) {
      const v = evalRow(f.node, own);
      if (v !== undefined) obj[f.name] = v;
    }
    return obj;
  }
  const start = node.offsets[own];
  const end = node.offsets[own + 1];
  if (node.k === 13) {
    const out = new Array(end - start);
    for (let e = start; e < end; e++) {
      const v = evalRow(node.element, e);
      out[e - start] = v === undefined ? null : v;
    }
    return out;
  }
  const pairs = [];
  for (let e = start; e < end; e++) {
    const ki = node.codes ? node.codes[e] : e;
    if (ki >= node.keys.length) throw new RangeError("map key");
    const v = evalRow(node.value, e);
    if (v !== undefined) pairs.push([node.keys[ki], v]);
  }
  pairs.sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
  const obj = {};
  for (const [k, v] of pairs) obj[k] = v;
  return obj;
}

export function decodeNestedColumnJson(rawNested, featureCount) {
  const root = decodeTree(rawNested.root, null);
  checkTreeCounts(root);
  if (parentCount(root) !== rawNested.valueCount) {
    throw new Error("nested root count");
  }
  const values = new Array(featureCount);
  let denseRow = 0;
  for (let f = 0; f < featureCount; f++) {
    if (!isBitSet(rawNested.presenceBits, f)) {
      values[f] = null;
      continue;
    }
    const v = evalRow(root, denseRow++);
    values[f] = v === undefined ? null : v;
  }
  return { name: rawNested.name, values };
}
