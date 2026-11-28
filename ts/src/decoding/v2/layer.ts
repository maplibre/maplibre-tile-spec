import type {
    DecodeTileColumnsOptions,
    MltColumnLayer,
    MltGeometryColumns,
    MltIdColumn,
    MltMValueColumn,
    MltMValueScope,
    MltNestedColumn,
    MltPropertyColumn,
} from "../../columns";
import { decodeV1LayerAsColumns } from "../../mltDecoder";
import {
    type ByteReader,
    allocF32,
    allocF64,
    allocI8,
    allocI32,
    allocI64,
    allocU8,
    allocU32,
    allocU64,
    cursor,
    deinterleaveU64,
    readStr,
    readU8,
    readVarintU32,
    readVarintU64,
    resetBudget,
    subCursor,
} from "./binary";
import { GEOM_MESH, decodeGeometry02, vertexIndexAt } from "./geometry";
import {
    type RawColumnValues02,
    type RawNestedColumn,
    decodeNestedColumn,
    decodeScalarValues,
    parseColumnValues02,
    parseNested02,
} from "./nested";
import {
    type MutableIndexable,
    PRES_BITMAP,
    bitmapPopcount,
    isBitSet,
    normalizePresentBitmap,
    readPresenceBits,
    scatterPresent,
} from "./presence";
import {
    FM_BYTES,
    FM_INT,
    FM_STR,
    FM_VERTEX,
    NO_Z,
    type ParsedStream,
    ST_DATA_FSST,
    ST_DATA_NONE,
    ST_DATA_SHARED,
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
    decodeStreamU32,
    decodeStreamU64,
    parseStream02,
} from "./stream";
import {
    type RawSharedDict,
    decodeSharedDictColumns,
    decodeStringsDense,
    materializeStringProperty,
    stringCodesForMValue,
} from "./strings";

type TypedCol<V> = ArrayLike<V> & MutableIndexable<V> & { set(src: ArrayLike<V>): void };

function expandCol<V extends number | bigint, A extends TypedCol<V>>(
    dense: A,
    bits: Uint8Array | undefined,
    featureCount: number,
    alloc: (n: number) => A,
    copyDense: boolean,
): { values: A; present?: Uint8Array } {
    if (bits === undefined) {
        if (dense.length !== featureCount) throw new Error("col count");
        if (!copyDense) return { values: dense };
        const out = alloc(featureCount);
        out.set(dense);
        return { values: out };
    }
    const pop = bitmapPopcount(bits, featureCount);
    if (pop !== dense.length) throw new Error("popcount");
    const present = normalizePresentBitmap(bits, featureCount);
    const values = alloc(featureCount);
    if (present === undefined) {
        values.set(dense);
        return { values };
    }
    if (pop > 0) scatterPresent(bits, featureCount, dense, values);
    return { values, present };
}

function decodeValueProp02(
    name: string,
    cv: RawColumnValues02,
    bits: Uint8Array | undefined,
    featureCount: number,
): MltPropertyColumn {
    if (cv.dt === 11) {
        return materializeStringProperty(name, decodeStringsDense(cv.s, false), bits, featureCount);
    }
    const scratch = bits !== undefined;
    const dec = decodeScalarValues(cv, scratch, false);
    switch (dec.kind) {
        case "bool":
            return { name, kind: "bool", ...expandCol(dec.values, bits, featureCount, allocU8, false) };
        case "i8":
            return { name, kind: "i8", ...expandCol(dec.values, bits, featureCount, allocI8, false) };
        case "u8":
            return { name, kind: "u8", ...expandCol(dec.values, bits, featureCount, allocU8, false) };
        case "i32":
            return { name, kind: "i32", ...expandCol(dec.values, bits, featureCount, allocI32, scratch) };
        case "u32":
            return { name, kind: "u32", ...expandCol(dec.values, bits, featureCount, allocU32, scratch) };
        case "i64":
            return { name, kind: "i64", ...expandCol(dec.values, bits, featureCount, allocI64, true) };
        case "u64":
            return { name, kind: "u64", ...expandCol(dec.values, bits, featureCount, allocU64, true) };
        case "f32":
            return { name, kind: "f32", ...expandCol(dec.values, bits, featureCount, allocF32, false) };
        case "f64":
            return { name, kind: "f64", ...expandCol(dec.values, bits, featureCount, allocF64, false) };
    }
}

function readColPresence02(
    r: ByteReader,
    high: number,
    sharedBitfields: readonly Uint8Array[],
    featureCount: number,
): Uint8Array | undefined {
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

function decodeMValue02(
    name: string,
    scope: MltMValueScope,
    bits: Uint8Array | undefined,
    cv: RawColumnValues02,
    geometry: MltGeometryColumns,
    featureCount: number,
): MltMValueColumn {
    const present = normalizePresentBitmap(bits, featureCount);
    const offsets = allocU32(featureCount + 1);
    let cursorPos = 0;
    const triOff = geometry.triangleOffsets;
    for (let f = 0; f < featureCount; f++) {
        if (present === undefined || isBitSet(present, f)) {
            cursorPos +=
                scope === "triangle"
                    ? triOff
                        ? triOff[f + 1] - triOff[f]
                        : 0
                    : vertexIndexAt(geometry, f + 1) - vertexIndexAt(geometry, f);
        }
        offsets[f + 1] = cursorPos;
    }
    const base = { name, scope, offsets, ...(present !== undefined ? { present } : {}) };
    if (cv.dt === 11) {
        const { codes, dictionary } = stringCodesForMValue(decodeStringsDense(cv.s, true));
        if (codes.length !== cursorPos) throw new Error("mval len");
        return { ...base, kind: "strDict", codes, dictionary };
    }
    const dec = decodeScalarValues(cv, false, true);
    if (dec.values.length !== cursorPos) throw new Error("mval len");
    return { ...base, ...dec };
}

const GEO_HAS_GEO = 0x6ccc;
const GEO_HAS_PARTS = 0x2ff0;
const GEO_HAS_RINGS = 0x2f00;

function decodeLayer02(r: ByteReader, includeNested: boolean, nestedJsonMode: boolean): MltColumnLayer {
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
    if ((sharedCoded && sharedCount === 0) || geoLayout > 0x0e) throw new Error("v2 layout");
    if (hasMValues && (geoLayout === 0 || geoLayout === 1 || geoLayout === 0x0c)) throw new Error("v2 mval layout");
    if ((uniformType === GEOM_MESH) !== (geoLayout === 0x0e)) throw new Error("v2 mesh layout");

    const sharedBitfields: Uint8Array[] = new Array(sharedCount);
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

    const typesStream = uniformType >= 0 ? null : parseStream02(r, FM_INT, 4, ST_LEN_VARBIN, featureCount);
    const items: ParsedStream[] = [];
    if (((GEO_HAS_GEO >>> geoLayout) & 1) !== 0) items.push(parseStream02(r, FM_INT, 4, ST_LEN_GEOM, featureCount));
    if (((GEO_HAS_PARTS >>> geoLayout) & 1) !== 0) items.push(parseStream02(r, FM_INT, 4, ST_LEN_PARTS, featureCount));
    if (((GEO_HAS_RINGS >>> geoLayout) & 1) !== 0) items.push(parseStream02(r, FM_INT, 4, ST_LEN_RINGS, featureCount));
    if (geoLayout >= 0x0c) {
        items.push(
            parseStream02(r, FM_INT, 4, ST_LEN_TRIS, featureCount),
            parseStream02(r, FM_INT, 4, ST_OFF_TREE_SYM, featureCount),
            parseStream02(r, FM_INT, 4, ST_OFF_TREE_CYC, featureCount),
        );
    }
    const vtxStream = parseStream02(r, FM_VERTEX, 4, ST_DATA_VERTEX, featureCount);
    if (geoLayout === 0x0e && vtxStream.zStep === NO_Z) throw new Error("v2 mesh z");
    items.push(vtxStream);
    if (geoLayout <= 0x0b && (geoLayout & 1) === 1)
        items.push(parseStream02(r, FM_INT, 4, ST_OFF_VERTEX, featureCount));

    const geometry = decodeGeometry02(uniformType, featureCount, typesStream, items);

    let columnCount: number;
    let mValueCount = 0;
    if (!hasMValues) {
        columnCount = readVarintU32(r);
    } else {
        [columnCount, mValueCount] = deinterleaveU64(readVarintU64(r));
        if (mValueCount === 0) throw new Error("v2 empty mvals");
    }
    if (r.e - r.p < columnCount) throw new RangeError("v2 cols eof");

    let ids: MltIdColumn | undefined;
    const properties: MltPropertyColumn[] = [];
    const rawNested: RawNestedColumn[] | null = includeNested ? [] : null;
    const seen = new Set<string>();
    const reg = (n: string): void => {
        if (seen.has(n)) throw new Error("dup col");
        seen.add(n);
    };

    for (let i = 0; i < columnCount; i++) {
        const tb = readU8(r);
        const high = tb & 0xf0;
        const data = tb & 0x0f;

        if (data === 0x0f) {
            if (high !== 0x00 && high !== 0x10) throw new Error("v2 sdict");
            const prefix = readStr(r);
            const cc = readVarintU32(r);
            if (r.e - r.p < cc) throw new RangeError("v2 sdict eof");
            const lengths = parseStream02(r, FM_INT, 4, ST_LEN_DICT, -1);
            let shared: RawSharedDict;
            if (high === 0x00) {
                const blob = parseStream02(r, FM_BYTES, 4, ST_DATA_SHARED, -1);
                shared = { prefix, front: blob.front, lengths, data: blob, children: new Array(cc) };
            } else {
                const symLens = parseStream02(r, FM_INT, 4, ST_LEN_SYM, -1);
                const symTbl = parseStream02(r, FM_BYTES, 4, ST_DATA_FSST, -1);
                const corpus = parseStream02(r, FM_BYTES, 4, ST_DATA_SHARED, -1);
                shared = { prefix, front: corpus.front, lengths, symLens, symTbl, corpus, children: new Array(cc) };
            }
            for (let c = 0; c < cc; c++) {
                const cb = readU8(r);
                if ((cb & 0x0f) !== 0x0b) throw new Error("v2 sdict ch");
                const suffix = readStr(r);
                reg(`${prefix}${suffix}`);
                const cBits = readColPresence02(r, cb & 0xf0, sharedBitfields, featureCount);
                const cCount = cBits === undefined ? featureCount : bitmapPopcount(cBits, featureCount);
                shared.children[c] = {
                    suffix,
                    bits: cBits,
                    data: parseStream02(r, FM_STR, 4, ST_OFF_STRING, cCount, 1),
                };
            }
            properties.push(...decodeSharedDictColumns(shared, featureCount));
            continue;
        }

        const colName = data >= 2 ? readStr(r) : "";
        const bits = readColPresence02(r, high, sharedBitfields, featureCount);
        const dCount = bits === undefined ? featureCount : bitmapPopcount(bits, featureCount);

        if (data <= 1) {
            if (ids !== undefined) throw new Error("dup id");
            const s = parseStream02(r, FM_INT, data === 1 ? 8 : 4, ST_DATA_NONE, dCount);
            ids =
                data === 1
                    ? { kind: "u64", ...expandCol(decodeStreamU64(s, false), bits, featureCount, allocU64, true) }
                    : { kind: "u32", ...expandCol(decodeStreamU32(s, true), bits, featureCount, allocU32, true) };
        } else if (data <= 11) {
            reg(colName);
            properties.push(decodeValueProp02(colName, parseColumnValues02(r, data, dCount), bits, featureCount));
        } else {
            reg(colName);
            const parsed = parseNested02(r, colName, bits, data, dCount);
            if (rawNested !== null) rawNested.push(parsed);
        }
    }

    const mValues: MltMValueColumn[] = new Array(mValueCount);
    const vertexValues: MltMValueColumn[] = [];
    const triangleValues: MltMValueColumn[] = [];
    if (hasMValues) {
        if (r.e - r.p < mValueCount) throw new RangeError("v2 mvals eof");
        const isMesh = geoLayout === 0x0e;
        for (let m = 0; m < mValueCount; m++) {
            const tb = readU8(r);
            const data = tb & 0x0f;
            if (data < 2 || data > 11) throw new Error("v2 mval type");
            let scope: MltMValueScope = "vertex";
            if (isMesh) {
                const sb = readU8(r);
                if (sb === 1) scope = "triangle";
                else if (sb !== 0) throw new Error("v2 mval scope");
            }
            const mName = readStr(r);
            reg(mName);
            const bits = readColPresence02(r, tb & 0xf0, sharedBitfields, featureCount);
            const col = decodeMValue02(mName, scope, bits, parseColumnValues02(r, data, -1), geometry, featureCount);
            mValues[m] = col;
            if (scope === "triangle") triangleValues.push(col);
            else vertexValues.push(col);
        }
    }

    if (r.p !== r.e) throw new Error("v2 trailing");

    const nested: MltNestedColumn[] | undefined =
        rawNested !== null && rawNested.length > 0
            ? rawNested.map((rn) => decodeNestedColumn(rn, featureCount, nestedJsonMode))
            : undefined;

    return {
        name,
        extent,
        version: 2,
        featureCount,
        geometry,
        ...(ids !== undefined ? { ids } : {}),
        properties,
        ...(nested !== undefined ? { nested } : {}),
        vertexValues,
        triangleValues,
        mValues,
    };
}

export interface InternalDecodeOptions extends DecodeTileColumnsOptions {
    nestedJsonMode?: boolean;
}

export function decodeRawTileLayers(data: Uint8Array, options: InternalDecodeOptions = {}): MltColumnLayer[] {
    if (!(data instanceof Uint8Array)) throw new TypeError("expected Uint8Array");
    resetBudget(options.maxBytes);
    const filterSet = options.layers !== undefined ? new Set(options.layers) : null;
    const includeNested = options.includeNested !== false;
    const nestedJsonMode = options.nestedJsonMode === true;

    const r = cursor(data);
    const layers: MltColumnLayer[] = [];

    while (r.p < r.e) {
        const size = readVarintU32(r);
        const tag = readU8(r);
        if (size === 0) throw new Error("zero layer size");
        const body = subCursor(r, size - 1);
        if (tag !== 1 && tag !== 2) continue;

        if (filterSet !== null) {
            const nm = readStr(cursor(body.b, body.p, body.e));
            if (nm.length === 0) throw new Error("layer name");
            if (!filterSet.has(nm)) continue;
        }

        layers.push(
            tag === 1
                ? decodeV1LayerAsColumns(body.b, body.p, body.e)
                : decodeLayer02(body, includeNested, nestedJsonMode),
        );
    }
    return layers;
}

export function decodeSingleLayer02(
    bodyBytes: Uint8Array,
    includeNested = true,
    nestedJsonMode = false,
    maxBytes?: number,
    resetBudgetOnStart = true,
): MltColumnLayer {
    if (resetBudgetOnStart) resetBudget(maxBytes);
    return decodeLayer02(cursor(bodyBytes), includeNested, nestedJsonMode);
}
