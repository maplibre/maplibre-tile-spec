import type { MltNestedColumn } from "../../columns";
import { type ByteReader, allocArray, chargeBytes, peekU8, readStr, readU8, readVarintU32 } from "./binary";
import { bitmapPopcount, isBitSet, normalizePresentBitmap } from "./presence";
import {
    FM_BOOL,
    FM_FLOAT,
    FM_INT,
    HAS_EXPLICIT_COUNT,
    LG_DICT,
    type ParsedStream,
    ST_DATA_NONE,
    ST_DATA_SINGLE,
    ST_LEN_NESTED,
    ST_PRESENT,
    decodeBoolValues,
    decodeFloatValues,
    decodeStreamI8,
    decodeStreamI32,
    decodeStreamI64,
    decodeStreamU8,
    decodeStreamU32,
    decodeStreamU64,
    parseStream02,
} from "./stream";
import { type RawStringsStreamSet, decodeStringsDense, expandDenseStrings, parseStrings02 } from "./strings";

const MAX_NESTED_DEPTH = 8;
const SHAPES_BIT = 0x40;
const NODE_COST = 64;

export type RawScalarColumn02 =
    | { dt: 2 | 3 | 4 | 5 | 6 | 7 | 8; s: ParsedStream; d: null }
    | { dt: 9 | 10; s: ParsedStream; d: ParsedStream | null };

export type RawColumnValues02 = RawScalarColumn02 | { dt: 11; s: RawStringsStreamSet; d: null };

export type DecodedScalar =
    | { kind: "bool"; values: Uint8Array }
    | { kind: "i8"; values: Int8Array }
    | { kind: "u8"; values: Uint8Array }
    | { kind: "i32"; values: Int32Array }
    | { kind: "u32"; values: Uint32Array }
    | { kind: "i64"; values: BigInt64Array }
    | { kind: "u64"; values: BigUint64Array }
    | { kind: "f32"; values: Float32Array }
    | { kind: "f64"; values: Float64Array };

export function decodeScalarValues(cv: RawScalarColumn02, scratch = false, owned64 = false): DecodedScalar {
    switch (cv.dt) {
        case 2:
            return { kind: "bool", values: decodeBoolValues(cv.s) };
        case 3:
            return { kind: "i8", values: decodeStreamI8(cv.s) };
        case 4:
            return { kind: "u8", values: decodeStreamU8(cv.s) };
        case 5:
            return { kind: "i32", values: decodeStreamI32(cv.s, scratch) };
        case 6:
            return { kind: "u32", values: decodeStreamU32(cv.s, scratch) };
        case 7:
            return { kind: "i64", values: decodeStreamI64(cv.s, owned64) };
        case 8:
            return { kind: "u64", values: decodeStreamU64(cv.s, owned64) };
        case 9:
            return { kind: "f32", values: decodeFloatValues(cv.s, cv.d, true) as Float32Array };
        case 10:
            return { kind: "f64", values: decodeFloatValues(cv.s, cv.d, false) as Float64Array };
    }
}

export function parseColumnValues02(r: ByteReader, dt: number, impliedCount: number): RawColumnValues02 {
    if (impliedCount < 0 && (peekU8(r) & HAS_EXPLICIT_COUNT) === 0) throw new Error("explicit count");
    if (dt === 2) return { dt: 2, s: parseStream02(r, FM_BOOL, 4, ST_DATA_NONE, impliedCount), d: null };
    if (dt >= 3 && dt <= 6)
        return { dt: dt as 3 | 4 | 5 | 6, s: parseStream02(r, FM_INT, 4, ST_DATA_NONE, impliedCount), d: null };
    if (dt === 7 || dt === 8) return { dt, s: parseStream02(r, FM_INT, 8, ST_DATA_NONE, impliedCount), d: null };
    if (dt === 9 || dt === 10) {
        const w = dt === 9 ? 4 : 8;
        const s = parseStream02(r, FM_FLOAT, w, ST_DATA_NONE, impliedCount);
        return { dt, s, d: s.l === LG_DICT ? parseStream02(r, FM_FLOAT, w, ST_DATA_SINGLE, s.n) : null };
    }
    if (dt === 11) return { dt: 11, s: parseStrings02(r, impliedCount), d: null };
    throw new Error("col type");
}

interface RowShapes {
    table: Uint8Array;
    width: number;
    ids: Uint32Array;
}

type RawNode =
    | { k: 0; presence: ParsedStream | null; cv: RawColumnValues02 }
    | { k: 12; presence: ParsedStream | null; shapes: RowShapes | null; fields: Array<{ name: string; node: RawNode }> }
    | { k: 13; presence: ParsedStream | null; lengths: ParsedStream; element: RawNode }
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

function reqExplicit(r: ByteReader, count: number): void {
    if (count < 0 && (peekU8(r) & HAS_EXPLICIT_COUNT) === 0) throw new Error("nested explicit");
}

function parseNodeByte(b: number): { hasPres: boolean; shapes: boolean; data: number } {
    const high = b & 0xf0;
    const data = b & 0x0f;
    const pres = high & ~SHAPES_BIT;
    if ((pres !== 0 && pres !== 0x10) || data <= 1 || data > 14) throw new Error("node type");
    return { hasPres: pres === 0x10, shapes: (high & SHAPES_BIT) !== 0, data };
}

function parseRowShapes(r: ByteReader, width: number, rowsImplied: number): RowShapes {
    reqExplicit(r, -1);
    const table = decodeBoolValues(parseStream02(r, FM_BOOL, 4, ST_PRESENT, -1));
    reqExplicit(r, -1);
    const ids = decodeStreamU32(parseStream02(r, FM_INT, 4, ST_DATA_NONE, -1), false);
    if (width === 0 || table.length % width !== 0) throw new Error("shapes tbl");
    const shapeCount = table.length / width;
    for (let i = 0; i < ids.length; i++) if (ids[i] >= shapeCount) throw new RangeError("shape id");
    if (rowsImplied >= 0 && ids.length !== rowsImplied) throw new Error("shape rows");
    return { table, width, ids };
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
        const fields = new Array<{ name: string; node: RawNode }>(fieldCount);
        for (let i = 0; i < fieldCount; i++) {
            let childCount = count;
            if (shapes !== null) {
                if (parseNodeByte(peekU8(r)).hasPres) throw new Error("shaped child pres");
                if (count >= 0) {
                    childCount = 0;
                    for (let row = 0; row < shapes.ids.length; row++)
                        childCount += shapes.table[shapes.ids[row] * shapes.width + i];
                }
            }
            const field = parseNode(r, true, childCount, depth + 1);
            for (let j = 0; j < i; j++) if (fields[j].name === field.name) throw new Error("dup field");
            fields[i] = field;
        }
        return { k: 12, presence, shapes, fields };
    }
    if (kind === 13) {
        if (perRow) throw new Error("shaped list");
        reqExplicit(r, count);
        const lengths = parseStream02(r, FM_INT, 4, ST_LEN_NESTED, count);
        return { k: 13, presence, lengths, element: parseNode(r, false, -1, depth + 1).node };
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
    return { k: 14, presence, lengths, shapes, keysRaw, value: parseNode(r, false, -1, depth + 1).node };
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
    const bodyCount = nodeCount < 0 ? -1 : presence !== null ? bitmapPopcount(presence.d, presence.n) : nodeCount;
    if (typ.data <= 11) {
        if (typ.shapes) throw new Error("shaped leaf");
        reqExplicit(r, bodyCount);
        return { name, node: { k: 0, presence, cv: parseColumnValues02(r, typ.data, bodyCount) } };
    }
    return { name, node: parseInterior(r, typ.data, presence, bodyCount, typ.shapes, depth) };
}

export function parseNested02(
    r: ByteReader,
    name: string,
    presenceBits: Uint8Array | undefined,
    rootData: number,
    valueCount: number,
): RawNestedColumn {
    chargeBytes(NODE_COST);
    let perRow = false;
    if (r.p < r.e && peekU8(r) === SHAPES_BIT) {
        readU8(r);
        perRow = true;
    }
    return {
        name,
        presenceBits,
        valueCount,
        root: parseInterior(r, rootData, null, rootData === 12 ? valueCount : -1, perRow, 1),
    };
}

export function floatToJsonValue(v: number, isF32: boolean): number | string {
    if (Number.isFinite(v)) return v;
    const p = isF32 ? "f32" : "f64";
    return Number.isNaN(v) ? `${p}::NAN` : v > 0 ? `${p}::INFINITY` : `${p}::NEG_INFINITY`;
}

type DecodedLeaf = DecodedScalar | { kind: "str"; values: string[] };
interface PresenceMask {
    bits: Uint8Array;
    rank: Uint32Array;
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
        const flat: DecodedLeaf =
            raw.cv.dt === 11
                ? { kind: "str", values: expandDenseStrings(decodeStringsDense(raw.cv.s, false)) }
                : decodeScalarValues(raw.cv, false, true);
        return { k: 0, presence, flat };
    }
    if (raw.k === 12) {
        const sh = raw.shapes;
        const fields = raw.fields.map((f, i) => {
            let held: Uint8Array | null = null;
            if (sh) {
                held = new Uint8Array(sh.ids.length);
                for (let r = 0; r < sh.ids.length; r++) held[r] = sh.table[sh.ids[r] * sh.width + i];
            }
            return { name: f.name, node: decodeTree(f.node, held) };
        });
        return { k: 12, presence, fields };
    }
    if (raw.k === 13) {
        return {
            k: 13,
            presence,
            offsets: buildOffsets(decodeStreamU32(raw.lengths, false)),
            element: decodeTree(raw.element, null),
        };
    }
    const keys = expandDenseStrings(decodeStringsDense(raw.keysRaw, false));
    let lengths: Uint32Array;
    let codes: Uint32Array | null = null;
    if (raw.shapes) {
        const { table, width, ids } = raw.shapes;
        lengths = new Uint32Array(ids.length);
        let total = 0;
        for (let r = 0; r < ids.length; r++) {
            let len = 0;
            for (let k = 0; k < width; k++) len += table[ids[r] * width + k];
            lengths[r] = len;
            total += len;
        }
        codes = new Uint32Array(total);
        let pos = 0;
        for (let r = 0; r < ids.length; r++) {
            for (let k = 0; k < width; k++) if (table[ids[r] * width + k] === 1) codes[pos++] = k;
        }
    } else if (raw.lengths) {
        lengths = decodeStreamU32(raw.lengths, false);
    } else {
        throw new Error("map lengths");
    }
    return { k: 14, presence, offsets: buildOffsets(lengths), keys, codes, value: decodeTree(raw.value, null) };
}

function parentCount(node: DecodedTree): number {
    if (node.presence !== null) return node.presence.bits.length;
    if (node.k === 0) return node.flat.values.length;
    if (node.k === 12) return node.fields.length === 0 ? 0 : parentCount(node.fields[0].node);
    return node.offsets.length - 1;
}

function checkTreeCounts(node: DecodedTree): void {
    if (node.k === 0) return;
    if (node.k === 12) {
        const p = node.presence !== null ? node.presence.rank[node.presence.bits.length] : parentCount(node);
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
    if ((node.codes ? node.codes.length : node.keys.length) !== entries || parentCount(node.value) !== entries) {
        throw new Error("map count");
    }
    checkTreeCounts(node.value);
}

function evalRow(node: DecodedTree, row: number, jsonMode: boolean): unknown {
    let own = row;
    if (node.presence !== null) {
        if (row >= node.presence.bits.length || node.presence.bits[row] === 0) return undefined;
        own = node.presence.rank[row];
    }
    if (node.k === 0) {
        const { kind, values } = node.flat;
        if (kind === "bool") return values[own] === 1;
        if (kind === "i64" || kind === "u64") return jsonMode ? Number(values[own]) : values[own];
        if (kind === "f32" || kind === "f64")
            return jsonMode ? floatToJsonValue(values[own], kind === "f32") : values[own];
        return values[own];
    }
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
        for (let e = start; e < end; e++) out[e - start] = evalRow(node.element, e, jsonMode) ?? null;
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
    if (parentCount(root) !== rawNested.valueCount) throw new Error("nested root count");
    const present = normalizePresentBitmap(rawNested.presenceBits, featureCount);
    const values = allocArray<unknown>(featureCount);
    let denseRow = 0;
    for (let f = 0; f < featureCount; f++) {
        values[f] = isBitSet(rawNested.presenceBits, f) ? (evalRow(root, denseRow++, jsonMode) ?? null) : null;
    }
    return present !== undefined ? { name: rawNested.name, present, values } : { name: rawNested.name, values };
}
