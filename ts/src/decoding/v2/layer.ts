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
    readStr,
    readU8,
    readVarintU32,
    readVarintU64,
    resetArena,
    resetBudget,
    subCursor,
    trimArena,
} from "./codecs/binary";
import { trimFsstBuffers } from "./codecs/fsst";
import { deinterleaveU64 } from "./codecs/morton";
import {
    type MutableIndexable,
    PRES_BITMAP,
    bitmapPopcount,
    isBitSet,
    normalizePresentBitmap,
    readPresenceBits,
    scatterPresent,
} from "./codecs/presence";
import { GEOM_MESH, decodeGeometry, vertexIndexAt } from "./geometry";
import {
    type RawColumnValues02,
    type RawNestedColumn,
    decodeNestedColumn,
    parseColumnValues02,
    parseNested02,
} from "./nested";
import {
    FM_BYTES,
    FM_INT,
    FM_STR,
    FM_VERTEX,
    HK_BOOL,
    HK_FLOAT,
    HK_INT,
    NO_Z,
    type ParsedStream,
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
    decodeFloatColumnValuesF32,
    decodeFloatColumnValuesF64,
    decodeStreamI8,
    decodeStreamI32,
    decodeStreamI64,
    decodeStreamU8,
    decodeStreamU32,
    decodeStreamU64,
    parseStream01,
    parseStream02,
} from "./stream";
import {
    type RawSharedDict,
    type RawStringsStreamSet,
    type SharedDictChild,
    decodeSharedDictColumns,
    decodeStringsDense,
    materializeStringProperty,
    stringCodesForMValue,
} from "./strings";

function expandScalar<T extends ArrayLike<number> & MutableIndexable<number> & { set(src: ArrayLike<number>): void }>(
    dense: T,
    bits: Uint8Array | undefined,
    featureCount: number,
    alloc: (n: number) => T,
    fromScratch = false,
): { values: T; present?: Uint8Array } {
    if (bits === undefined) {
        if (dense.length !== featureCount) throw new Error("col count");
        if (fromScratch) {
            const out = alloc(featureCount);
            out.set(dense);
            return { values: out };
        }
        return { values: dense };
    }
    const pop = bitmapPopcount(bits, featureCount);
    if (pop !== dense.length) throw new Error("popcount");
    const present = normalizePresentBitmap(bits, featureCount);
    if (present === undefined) {
        const out = alloc(featureCount);
        out.set(dense);
        return { values: out };
    }
    const values = alloc(featureCount);
    if (pop > 0) scatterPresent(bits, featureCount, dense, values);
    return { values, present };
}

function expandBigInt64(
    dense: BigInt64Array,
    bits: Uint8Array | undefined,
    featureCount: number,
): { values: BigInt64Array; present?: Uint8Array } {
    if (bits === undefined) {
        if (dense.length !== featureCount) throw new Error("col64 count");
        const out = allocI64(featureCount);
        out.set(dense);
        return { values: out };
    }
    const pop = bitmapPopcount(bits, featureCount);
    if (pop !== dense.length) throw new Error("popcount64");
    const present = normalizePresentBitmap(bits, featureCount);
    const values = allocI64(featureCount);
    if (present === undefined) {
        values.set(dense);
        return { values };
    }
    if (pop > 0) scatterPresent(bits, featureCount, dense, values);
    return { values, present };
}

function expandBigUint64(
    dense: BigUint64Array,
    bits: Uint8Array | undefined,
    featureCount: number,
): { values: BigUint64Array; present?: Uint8Array } {
    if (bits === undefined) {
        if (dense.length !== featureCount) throw new Error("col64 count");
        const out = allocU64(featureCount);
        out.set(dense);
        return { values: out };
    }
    const pop = bitmapPopcount(bits, featureCount);
    if (pop !== dense.length) throw new Error("popcount64");
    const present = normalizePresentBitmap(bits, featureCount);
    const values = allocU64(featureCount);
    if (present === undefined) {
        values.set(dense);
        return { values };
    }
    if (pop > 0) scatterPresent(bits, featureCount, dense, values);
    return { values, present };
}

function decodeId(
    is64: boolean,
    bits: Uint8Array | undefined,
    stream: ParsedStream,
    featureCount: number,
): MltIdColumn {
    if (is64) {
        return { kind: "u64", ...expandBigUint64(decodeStreamU64(stream, false), bits, featureCount) };
    }
    const u32s = decodeStreamU32(stream, true);
    return { kind: "u32", ...expandScalar(u32s, bits, featureCount, allocU32, true) };
}

function decodeValueProp(
    name: string,
    dt: number,
    s: ParsedStream | RawStringsStreamSet,
    dictStream: ParsedStream | null,
    bits: Uint8Array | undefined,
    featureCount: number,
): MltPropertyColumn {
    if (dt === 11) {
        return materializeStringProperty(name, decodeStringsDense(s as RawStringsStreamSet, false), bits, featureCount);
    }
    const ps = s as ParsedStream;
    const scratch = bits !== undefined;
    switch (dt) {
        case 2:
            return { name, kind: "bool", ...expandScalar(decodeBoolValues(ps), bits, featureCount, allocU8) };
        case 3:
            return { name, kind: "i8", ...expandScalar(decodeStreamI8(ps), bits, featureCount, allocI8) };
        case 4:
            return { name, kind: "u8", ...expandScalar(decodeStreamU8(ps), bits, featureCount, allocU8) };
        case 5:
            return {
                name,
                kind: "i32",
                ...expandScalar(decodeStreamI32(ps, scratch), bits, featureCount, allocI32, scratch),
            };
        case 6:
            return {
                name,
                kind: "u32",
                ...expandScalar(decodeStreamU32(ps, scratch), bits, featureCount, allocU32, scratch),
            };
        case 7:
            return { name, kind: "i64", ...expandBigInt64(decodeStreamI64(ps, false), bits, featureCount) };
        case 8:
            return { name, kind: "u64", ...expandBigUint64(decodeStreamU64(ps, false), bits, featureCount) };
        case 9:
            return {
                name,
                kind: "f32",
                ...expandScalar(decodeFloatColumnValuesF32(ps, dictStream), bits, featureCount, allocF32),
            };
        default:
            return {
                name,
                kind: "f64",
                ...expandScalar(decodeFloatColumnValuesF64(ps, dictStream), bits, featureCount, allocF64),
            };
    }
}

const V1_VALID_MASK = 0x7fff0c1f;

interface Col01Meta {
    typ: number;
    name: string;
    children: Array<{ typ: number; name: string }> | null;
}

type RawProp01 =
    | { dt: number; name: string; bits: Uint8Array | undefined; s: ParsedStream | RawStringsStreamSet }
    | { dt: 30; shared: RawSharedDict };

function decodeLayer01(r: ByteReader): MltColumnLayer {
    resetArena();
    const name = readStr(r);
    if (name.length === 0) throw new Error("layer name");
    const extent = readVarintU32(r);
    if (extent === 0) throw new Error("extent");
    const colCount = readVarintU32(r);
    if (r.e - r.p < colCount) throw new RangeError("v1 cols eof");

    const colsMeta: Col01Meta[] = new Array(colCount);
    let geomCols = 0;
    let idCols = 0;

    for (let i = 0; i < colCount; i++) {
        const typ = readU8(r);
        if (typ > 30 || ((V1_VALID_MASK >>> typ) & 1) === 0) {
            throw new Error("v1 col");
        }
        const colName = typ >= 10 ? readStr(r) : "";
        let children: Array<{ typ: number; name: string }> | null = null;
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

    let rawId: { is64: boolean; bits: Uint8Array | undefined; s: ParsedStream } | null = null;
    let geomTypesStream: ParsedStream | null = null;
    let geomItems: ParsedStream[] | null = null;
    const rawProps: RawProp01[] = [];

    const optBits = (typ: number): Uint8Array | undefined =>
        (typ & 1) === 0 ? undefined : decodeBoolBitmap(parseStream01(r, HK_BOOL, true));

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
            rawProps.push({ dt: 2, name: colName, bits, s: parseStream01(r, HK_BOOL, true) });
        } else if (typ <= 23) {
            const dtMap = [5, 5, 6, 6, 7, 7, 8, 8];
            const bits = optBits(typ);
            rawProps.push({ dt: dtMap[typ - 16], name: colName, bits, s: parseStream01(r, HK_INT) });
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
            const s: ParsedStream[] = new Array(sc);
            for (let k = 0; k < sc; k++) s[k] = parseStream01(r, HK_INT);
            const strRaw: RawStringsStreamSet =
                sc === 2
                    ? { m: 0, lengths: s[0], data: s[1], n: s[0].n }
                    : sc === 3
                      ? { m: 1, lengths: s[0], offsets: s[1], data: s[2], front: 0, n: s[1].n }
                      : sc === 4
                        ? { m: 2, symLens: s[0], symTbl: s[1], lengths: s[2], corpus: s[3], n: s[2].n }
                        : {
                              m: 3,
                              symLens: s[0],
                              symTbl: s[1],
                              lengths: s[2],
                              corpus: s[3],
                              offsets: s[4],
                              front: 0,
                              n: s[4].n,
                          };
            rawProps.push({ dt: 11, name: colName, bits, s: strRaw });
        } else {
            const streamCount = readVarintU32(r);
            const ds: ParsedStream[] = [];
            while (ds.length < streamCount) {
                const st = parseStream01(r, HK_INT);
                ds.push(st);
                if (st.t === ST_DATA_SINGLE || st.t === ST_DATA_SHARED) break;
                if (ds.length >= 5) throw new Error("v1 sdict");
            }
            const colChildren = col.children ?? [];
            const children: SharedDictChild[] = new Array(colChildren.length);
            let optCount = 0;
            for (let c = 0; c < colChildren.length; c++) {
                const ch = colChildren[c];
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
            const shared: RawSharedDict =
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
    const properties: MltPropertyColumn[] = [];
    for (let i = 0; i < rawProps.length; i++) {
        const p = rawProps[i];
        if ("shared" in p) {
            properties.push(...decodeSharedDictColumns(p.shared, featureCount));
        } else {
            properties.push(decodeValueProp(p.name, p.dt, p.s, null, p.bits, featureCount));
        }
    }

    const ids = rawId !== null ? decodeId(rawId.is64, rawId.bits, rawId.s, featureCount) : undefined;
    return {
        name,
        extent,
        version: 1,
        featureCount,
        geometry,
        ...(ids !== undefined ? { ids } : {}),
        properties,
        vertexValues: [],
        triangleValues: [],
        mValues: [],
    };
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
            if (scope === "triangle") {
                cursorPos += triOff ? triOff[f + 1] - triOff[f] : 0;
            } else {
                cursorPos += vertexIndexAt(geometry, f + 1) - vertexIndexAt(geometry, f);
            }
        }
        offsets[f + 1] = cursorPos;
    }

    const base = {
        name,
        scope,
        offsets,
        ...(present !== undefined ? { present } : {}),
    };

    if (cv.dt === 11) {
        const { codes, dictionary } = stringCodesForMValue(decodeStringsDense(cv.s, true));
        if (codes.length !== cursorPos) throw new Error("mval len");
        return { ...base, kind: "strDict", codes, dictionary };
    }
    switch (cv.dt) {
        case 2: {
            const values = decodeBoolValues(cv.s);
            if (values.length !== cursorPos) throw new Error("mval len");
            return { ...base, kind: "bool", values };
        }
        case 3: {
            const values = decodeStreamI8(cv.s);
            if (values.length !== cursorPos) throw new Error("mval len");
            return { ...base, kind: "i8", values };
        }
        case 4: {
            const values = decodeStreamU8(cv.s);
            if (values.length !== cursorPos) throw new Error("mval len");
            return { ...base, kind: "u8", values };
        }
        case 5: {
            const values = decodeStreamI32(cv.s, false);
            if (values.length !== cursorPos) throw new Error("mval len");
            return { ...base, kind: "i32", values };
        }
        case 6: {
            const values = decodeStreamU32(cv.s, false);
            if (values.length !== cursorPos) throw new Error("mval len");
            return { ...base, kind: "u32", values };
        }
        case 7: {
            const values = decodeStreamI64(cv.s, true);
            if (values.length !== cursorPos) throw new Error("mval len");
            return { ...base, kind: "i64", values };
        }
        case 8: {
            const values = decodeStreamU64(cv.s, true);
            if (values.length !== cursorPos) throw new Error("mval len");
            return { ...base, kind: "u64", values };
        }
        case 9: {
            const values = decodeFloatColumnValuesF32(cv.s, cv.d);
            if (values.length !== cursorPos) throw new Error("mval len");
            return { ...base, kind: "f32", values };
        }
        default: {
            const values = decodeFloatColumnValuesF64(cv.s, cv.d);
            if (values.length !== cursorPos) throw new Error("mval len");
            return { ...base, kind: "f64", values };
        }
    }
}

const GEO_HAS_GEO = 0x6ccc;
const GEO_HAS_PARTS = 0x2ff0;
const GEO_HAS_RINGS = 0x2f00;

function decodeLayer02(r: ByteReader, includeNested: boolean, nestedJsonMode: boolean): MltColumnLayer {
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
    if (hasMValues && (geoLayout === 0 || geoLayout === 1 || geoLayout === 0x0c)) {
        throw new Error("v2 mval layout");
    }
    if ((uniformType === GEOM_MESH) !== (geoLayout === 0x0e)) {
        throw new Error("v2 mesh layout");
    }

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
    const vtxStream = parseStream02(r, FM_VERTEX, 4, ST_DATA_VERTEX, featureCount);
    if (geoLayout === 0x0e && vtxStream.zStep === NO_Z) {
        throw new Error("v2 mesh z");
    }
    items.push(vtxStream);
    if (geoLayout <= 0x0b && (geoLayout & 1) === 1) {
        items.push(parseStream02(r, FM_INT, 4, ST_OFF_VERTEX, featureCount));
    }

    const geometry = decodeGeometry(uniformType, featureCount, typesStream, true, items);
    resetArena();

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
        resetArena();
        const tb = readU8(r);
        const high = tb & 0xf0;
        const data = tb & 0x0f;

        if (data === 0x0f) {
            if (high !== 0x00 && high !== 0x10) throw new Error("v2 sdict");
            const prefix = readStr(r);
            const cc = readVarintU32(r);
            if (r.e - r.p < cc) throw new RangeError("v2 sdict eof");
            let shared: RawSharedDict;
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
                const cBits = readColPresence02(r, cb & 0xf0, sharedBitfields, featureCount);
                const cCount = cBits === undefined ? featureCount : bitmapPopcount(cBits, featureCount);
                const cData = parseStream02(r, FM_STR, 4, ST_OFF_STRING, cCount, 1);
                shared.children[c] = { suffix, bits: cBits, data: cData };
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
            ids = decodeId(data === 1, bits, s, featureCount);
        } else if (data <= 11) {
            reg(colName);
            const cv = parseColumnValues02(r, data, dCount);
            properties.push(decodeValueProp(colName, data, cv.s, cv.d, bits, featureCount));
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
            resetArena();
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
            const cv = parseColumnValues02(r, data, -1);
            const col = decodeMValue02(mName, scope, bits, cv, geometry, featureCount);
            mValues[m] = col;
            if (scope === "triangle") triangleValues.push(col);
            else vertexValues.push(col);
        }
    }

    if (r.p !== r.e) throw new Error("v2 trailing");

    let nested: MltNestedColumn[] | undefined;
    if (rawNested !== null && rawNested.length > 0) {
        nested = rawNested.map((rn) => decodeNestedColumn(rn, featureCount, nestedJsonMode));
    }

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
    if (!(data instanceof Uint8Array)) {
        throw new TypeError("expected Uint8Array");
    }
    resetBudget(options.maxBytes);
    const filterSet = options.layers !== undefined ? new Set(options.layers) : null;
    const includeNested = options.includeNested !== false;
    const nestedJsonMode = options.nestedJsonMode === true;

    const r = cursor(data);
    const layers: MltColumnLayer[] = [];

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

            layers.push(tag === 1 ? decodeLayer01(body) : decodeLayer02(body, includeNested, nestedJsonMode));
        }
    } finally {
        trimArena();
        trimFsstBuffers();
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
    if (resetBudgetOnStart) {
        resetBudget(maxBytes);
    }
    try {
        return decodeLayer02(cursor(bodyBytes), includeNested, nestedJsonMode);
    } finally {
        trimArena();
        trimFsstBuffers();
    }
}
