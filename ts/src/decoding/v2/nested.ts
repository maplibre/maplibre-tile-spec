import type { MltNestedColumn } from "../../columns";
import { type ByteReader, allocArray, chargeBytes, peekU8, readStr, readU8, readVarintU32 } from "./codecs/binary";
import {
    PRES_BITMAP,
    PRES_RUNS,
    PRES_SPARSE,
    isBitSet,
    normalizePresentBitmap,
    popcountPresencePayload,
} from "./codecs/presence";
import {
    FM_BOOL,
    FM_FLOAT,
    FM_INT,
    HAS_EXPLICIT_COUNT,
    LG_DICT,
    LG_NONE,
    LG_RUNS,
    type ParsedStream,
    ST_DATA_NONE,
    ST_DATA_SINGLE,
    ST_LEN_NESTED,
    ST_PRESENT,
    decodeBoolValues,
    decodeFloatColumnValuesF32,
    decodeFloatColumnValuesF64,
    decodeStreamI8,
    decodeStreamI32,
    decodeStreamI64,
    decodeStreamU8,
    decodeStreamU32,
    decodeStreamU64,
    parseStream02,
} from "./stream";
import {
    type RawStringsStreamSet,
    chargeExpandedDictStrings,
    decodeStringsDense,
    isDictDenseStrings,
    parseStrings02,
} from "./strings";

const MAX_NESTED_DEPTH = 8;
const SHAPES_BIT = 0x40;
const NODE_COST = 64;

export type RawColumnValues02 =
    | { dt: 2 | 3 | 4 | 5 | 6 | 7 | 8; s: ParsedStream; d: null }
    | { dt: 9 | 10; s: ParsedStream; d: ParsedStream | null }
    | { dt: 11; s: RawStringsStreamSet; d: null };

interface RowShapes {
    table: Uint8Array;
    width: number;
    ids: Uint32Array;
}

type RawNode =
    | { k: 0; presence: ParsedStream | null; cv: RawColumnValues02 }
    | {
          k: 12;
          presence: ParsedStream | null;
          shapes: RowShapes | null;
          fields: Array<{ name: string; node: RawNode }>;
      }
    | {
          k: 13;
          presence: ParsedStream | null;
          lengths: ParsedStream;
          element: RawNode;
      }
    | {
          k: 14;
          presence: ParsedStream | null;
          lengths: ParsedStream | null;
          shapes: RowShapes | null;
          keysRaw: RawStringsStreamSet;
          value: RawNode;
      };

export interface RawNestedColumn {
    name: string;
    presenceBits: Uint8Array | undefined;
    valueCount: number;
    root: Exclude<RawNode, { k: 0 }>;
}

export function parseColumnValues02(r: ByteReader, dataType: number, impliedCount: number): RawColumnValues02 {
    if (impliedCount < 0 && (peekU8(r) & HAS_EXPLICIT_COUNT) === 0) {
        throw new Error("explicit count");
    }
    if (dataType === 2) {
        return { dt: 2, s: parseStream02(r, FM_BOOL, 4, ST_DATA_NONE, impliedCount), d: null };
    }
    if (dataType === 3 || dataType === 4 || dataType === 5 || dataType === 6) {
        return { dt: dataType, s: parseStream02(r, FM_INT, 4, ST_DATA_NONE, impliedCount), d: null };
    }
    if (dataType === 7 || dataType === 8) {
        return { dt: dataType, s: parseStream02(r, FM_INT, 8, ST_DATA_NONE, impliedCount), d: null };
    }
    if (dataType === 9 || dataType === 10) {
        const width = dataType === 9 ? 4 : 8;
        const s = parseStream02(r, FM_FLOAT, width, ST_DATA_NONE, impliedCount);
        const d = s.l === LG_DICT ? parseStream02(r, FM_FLOAT, width, ST_DATA_SINGLE, s.n) : null;
        return { dt: dataType, s, d };
    }
    if (dataType === 11) {
        return { dt: 11, s: parseStrings02(r, impliedCount), d: null };
    }
    throw new Error("col type");
}

function reqExplicit(r: ByteReader, count: number): void {
    if (count < 0 && (peekU8(r) & HAS_EXPLICIT_COUNT) === 0) {
        throw new Error("nested explicit");
    }
}

function parseNodeByte(b: number): { hasPres: boolean; shapes: boolean; data: number } {
    const high = b & 0xf0;
    const data = b & 0x0f;
    const shapes = (high & SHAPES_BIT) !== 0;
    const pres = high & ~SHAPES_BIT;
    if ((pres !== 0 && pres !== 0x10) || data <= 1 || data > 14) {
        throw new Error("node type");
    }
    return { hasPres: pres === 0x10, shapes, data };
}

function presPopcount(s: ParsedStream): number {
    const coding = s.l === LG_NONE ? PRES_BITMAP : s.l === LG_RUNS ? PRES_RUNS : PRES_SPARSE;
    return popcountPresencePayload(s.d, s.n, coding);
}

function parseRowShapes(r: ByteReader, width: number, rowsImplied: number): RowShapes {
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

function shapePresentCount(sh: RowShapes, key: number): number {
    const { table, width, ids } = sh;
    let c = 0;
    for (let row = 0; row < ids.length; row++) c += table[ids[row] * width + key];
    return c;
}

function parseInterior(
    r: ByteReader,
    kind: number,
    presence: ParsedStream | null,
    count: number,
    perRow: boolean,
    depth: number,
): Exclude<RawNode, { k: 0 }> {
    if (kind === 12) {
        const fieldCount = readVarintU32(r);
        if (fieldCount === 0) throw new Error("empty struct");
        if (r.e - r.p < fieldCount) throw new RangeError("struct eof");
        chargeBytes(fieldCount * NODE_COST);
        const shapes = perRow ? parseRowShapes(r, fieldCount, count) : null;
        const fields: Array<{ name: string; node: RawNode }> = new Array(fieldCount);
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
        if (perRow) throw new Error("shaped list");
        reqExplicit(r, count);
        const lengths = parseStream02(r, FM_INT, 4, ST_LEN_NESTED, count);
        const element = parseNode(r, false, -1, depth + 1).node;
        return { k: 13, presence, lengths, element };
    }

    let keysRaw: RawStringsStreamSet;
    let lengths: ParsedStream | null = null;
    let shapes: RowShapes | null = null;
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

function parseNode(r: ByteReader, named: boolean, parentCount: number, depth: number): { name: string; node: RawNode } {
    if (depth > MAX_NESTED_DEPTH) throw new Error("nested depth");
    chargeBytes(NODE_COST);
    const typ = parseNodeByte(readU8(r));
    const name = named ? readStr(r) : "";
    const nodeCount = typ.data === 13 || typ.data === 14 ? -1 : parentCount;

    let presence: ParsedStream | null = null;
    if (typ.hasPres) {
        reqExplicit(r, nodeCount);
        presence = parseStream02(r, FM_BOOL, 4, ST_PRESENT, nodeCount);
    }
    const bodyCount = nodeCount < 0 ? -1 : presence !== null ? presPopcount(presence) : nodeCount;

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

export function parseNested02(
    r: ByteReader,
    name: string,
    presenceBits: Uint8Array | undefined,
    rootData: number,
    valueCount: number,
): RawNestedColumn {
    chargeBytes(NODE_COST);
    const count = rootData === 12 ? valueCount : -1;
    let perRow = false;
    if (r.p < r.e && peekU8(r) === SHAPES_BIT) {
        readU8(r);
        perRow = true;
    }
    const root = parseInterior(r, rootData, null, count, perRow, 1);
    return { name, presenceBits, valueCount, root };
}

export function floatToJsonValue(v: number, isF32: boolean): number | string {
    if (Number.isFinite(v)) return v;
    const p = isF32 ? "f32" : "f64";
    return Number.isNaN(v) ? `${p}::NAN` : v > 0 ? `${p}::INFINITY` : `${p}::NEG_INFINITY`;
}

type DecodedLeaf =
    | { dt: 2; v: Uint8Array }
    | { dt: 3; v: Int8Array }
    | { dt: 4; v: Uint8Array }
    | { dt: 5; v: Int32Array }
    | { dt: 6; v: Uint32Array }
    | { dt: 7; v: BigInt64Array }
    | { dt: 8; v: BigUint64Array }
    | { dt: 9; v: Float32Array }
    | { dt: 10; v: Float64Array }
    | { dt: 11; v: string[] };

interface PresenceMask {
    readonly bits: Uint8Array;
    readonly rank: Uint32Array;
}

function makePresenceMask(bits: Uint8Array | null): PresenceMask | null {
    if (bits === null) return null;
    const rank = new Uint32Array(bits.length + 1);
    let acc = 0;
    for (let i = 0; i < bits.length; i++) {
        rank[i] = acc;
        acc += bits[i];
    }
    rank[bits.length] = acc;
    return { bits, rank };
}

type DecodedTree =
    | { k: 0; presence: PresenceMask | null; flat: DecodedLeaf }
    | { k: 12; presence: PresenceMask | null; fields: Array<{ name: string; node: DecodedTree }> }
    | { k: 13; presence: PresenceMask | null; offsets: Uint32Array; element: DecodedTree }
    | {
          k: 14;
          presence: PresenceMask | null;
          offsets: Uint32Array;
          keys: string[];
          codes: Uint32Array | null;
          value: DecodedTree;
      };

function decodeLeafFlat(cv: RawColumnValues02): DecodedLeaf {
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
            return { dt: 9, v: decodeFloatColumnValuesF32(cv.s, cv.d) };
        case 10:
            return { dt: 10, v: decodeFloatColumnValuesF64(cv.s, cv.d) };
        case 11: {
            const sd = decodeStringsDense(cv.s, false);
            if (isDictDenseStrings(sd)) {
                chargeExpandedDictStrings(sd.entries, sd.codes);
                const strings = allocArray<string>(sd.codes.length);
                for (let i = 0; i < sd.codes.length; i++) strings[i] = sd.entries[sd.codes[i]];
                return { dt: 11, v: strings };
            }
            return { dt: 11, v: sd.values };
        }
    }
}

function shapeHeld(sh: RowShapes, key: number): Uint8Array {
    const { table, width, ids } = sh;
    const out = new Uint8Array(ids.length);
    for (let r = 0; r < ids.length; r++) out[r] = table[ids[r] * width + key];
    return out;
}

function shapeLengthsAndCodes(sh: RowShapes): { lengths: Uint32Array; codes: Uint32Array } {
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

function buildOffsets(lengths: Uint32Array): Uint32Array {
    const off = new Uint32Array(lengths.length + 1);
    let total = 0;
    for (let i = 0; i < lengths.length; i++) {
        total += lengths[i];
        if (total > 0xffffffff) throw new RangeError("overflow");
        off[i + 1] = total;
    }
    return off;
}

function decodeTree(raw: RawNode, overridePres: Uint8Array | null): DecodedTree {
    const presence = makePresenceMask(overridePres ?? (raw.presence ? decodeBoolValues(raw.presence) : null));
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
    let keys: string[];
    if (isDictDenseStrings(kd)) {
        chargeExpandedDictStrings(kd.entries, kd.codes);
        keys = allocArray<string>(kd.codes.length);
        for (let i = 0; i < kd.codes.length; i++) keys[i] = kd.entries[kd.codes[i]];
    } else {
        keys = kd.values;
    }
    let lengths: Uint32Array;
    let codes: Uint32Array | null = null;
    if (raw.shapes) {
        const sc = shapeLengthsAndCodes(raw.shapes);
        lengths = sc.lengths;
        codes = sc.codes;
    } else if (raw.lengths) {
        lengths = decodeStreamU32(raw.lengths, false);
    } else {
        throw new Error("map lengths");
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

function parentCount(node: DecodedTree): number {
    if (node.presence !== null) return node.presence.bits.length;
    if (node.k === 0) return node.flat.v.length;
    if (node.k === 12) return structPresent(node);
    return node.offsets.length - 1;
}

function structPresent(node: Extract<DecodedTree, { k: 12 }>): number {
    if (node.presence !== null) {
        return node.presence.rank[node.presence.bits.length];
    }
    return node.fields.length === 0 ? 0 : parentCount(node.fields[0].node);
}

function checkTreeCounts(node: DecodedTree): void {
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

function leafValueAt(flat: DecodedLeaf, idx: number, jsonMode: boolean): unknown {
    if (flat.dt === 2) return flat.v[idx] === 1;
    if (flat.dt === 7 || flat.dt === 8) return jsonMode ? Number(flat.v[idx]) : flat.v[idx];
    if (flat.dt === 9) return jsonMode ? floatToJsonValue(flat.v[idx], true) : flat.v[idx];
    if (flat.dt === 10) return jsonMode ? floatToJsonValue(flat.v[idx], false) : flat.v[idx];
    return flat.v[idx];
}

function evalRow(node: DecodedTree, row: number, jsonMode: boolean): unknown {
    let own = row;
    if (node.presence !== null) {
        if (row >= node.presence.bits.length || node.presence.bits[row] === 0) return undefined;
        own = node.presence.rank[row];
    }
    if (node.k === 0) return leafValueAt(node.flat, own, jsonMode);
    if (node.k === 12) {
        const obj: Record<string, unknown> = {};
        const sorted = [...node.fields].sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));
        for (const f of sorted) {
            const v = evalRow(f.node, own, jsonMode);
            if (v !== undefined) obj[f.name] = v;
        }
        return obj;
    }
    const start = node.offsets[own];
    const end = node.offsets[own + 1];
    if (node.k === 13) {
        const out = allocArray<unknown>(end - start);
        for (let e = start; e < end; e++) {
            const v = evalRow(node.element, e, jsonMode);
            out[e - start] = v === undefined ? null : v;
        }
        return out;
    }
    const pairs: Array<[string, unknown]> = [];
    for (let e = start; e < end; e++) {
        const ki = node.codes ? node.codes[e] : e;
        if (ki >= node.keys.length) throw new RangeError("map key");
        const v = evalRow(node.value, e, jsonMode);
        if (v !== undefined) pairs.push([node.keys[ki], v]);
    }
    pairs.sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
    const obj: Record<string, unknown> = {};
    for (const [k, v] of pairs) obj[k] = v;
    return obj;
}

export function decodeNestedColumn(
    rawNested: RawNestedColumn,
    featureCount: number,
    jsonMode = false,
): MltNestedColumn {
    const root = decodeTree(rawNested.root, null);
    checkTreeCounts(root);
    if (parentCount(root) !== rawNested.valueCount) {
        throw new Error("nested root count");
    }
    const present = normalizePresentBitmap(rawNested.presenceBits, featureCount);
    const values = allocArray<unknown>(featureCount);
    let denseRow = 0;
    for (let f = 0; f < featureCount; f++) {
        if (!isBitSet(rawNested.presenceBits, f)) {
            values[f] = null;
            continue;
        }
        const v = evalRow(root, denseRow++, jsonMode);
        values[f] = v === undefined ? null : v;
    }
    return present !== undefined ? { name: rawNested.name, present, values } : { name: rawNested.name, values };
}

export function decodeNestedColumnJson(rawNested: RawNestedColumn, featureCount: number): MltNestedColumn {
    return decodeNestedColumn(rawNested, featureCount, true);
}
