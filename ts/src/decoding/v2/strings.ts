import type { MltStringPropertyColumn } from "../../columns";
import { decodeFrontCoded } from "../frontCoding";
import { decodeFsst } from "../fsstDecoder";
import { type ByteReader, allocArray, allocU32, chargeBytes, decodeUtf8 } from "./binary";
import { bitmapPopcount, normalizePresentBitmap, scatterPresent } from "./presence";
import {
    FM_BYTES,
    FM_INT,
    FM_STR,
    type ParsedStream,
    ST_DATA_FSST,
    ST_DATA_NONE,
    ST_DATA_SINGLE,
    ST_LEN_DICT,
    ST_LEN_SYM,
    ST_LEN_VARBIN,
    ST_OFF_STRING,
    decodeStreamU32,
    parseStream02,
} from "./stream";

export type RawStringsStreamSet =
    | { m: 0; lengths: ParsedStream; data: ParsedStream; n: number }
    | { m: 1; offsets: ParsedStream; lengths: ParsedStream; data: ParsedStream; front: number; n: number }
    | { m: 2; lengths: ParsedStream; symLens: ParsedStream; symTbl: ParsedStream; corpus: ParsedStream; n: number }
    | {
          m: 3;
          offsets: ParsedStream;
          lengths: ParsedStream;
          symLens: ParsedStream;
          symTbl: ParsedStream;
          corpus: ParsedStream;
          front: number;
          n: number;
      };

export interface SharedDictChild {
    suffix: string;
    bits: Uint8Array | undefined;
    data: ParsedStream;
}

export type RawSharedDict =
    | { prefix: string; front: number; lengths: ParsedStream; data: ParsedStream; children: SharedDictChild[] }
    | {
          prefix: string;
          front: number;
          lengths: ParsedStream;
          symLens: ParsedStream;
          symTbl: ParsedStream;
          corpus: ParsedStream;
          children: SharedDictChild[];
      };

export type DecodedDenseStrings =
    | { dict: false; values: string[] }
    | { dict: true; entries: string[]; codes: Uint32Array };

export function isDictDenseStrings(d: DecodedDenseStrings): d is { dict: true; entries: string[]; codes: Uint32Array } {
    return d.dict;
}

function decodeFsstBytes(symLens: Uint32Array, symTbl: Uint8Array, corpus: Uint8Array): Uint8Array {
    if (symLens.length > 255) throw new Error("fsst count");
    let symTotal = 0;
    for (let i = 0; i < symLens.length; i++) {
        if (symLens[i] < 1 || symLens[i] > 8) throw new Error("fsst len");
        symTotal += symLens[i];
    }
    if (symTotal > symTbl.length) throw new RangeError("fsst tbl");
    let outLen = 0;
    for (let p = 0; p < corpus.length; p++) {
        const code = corpus[p];
        if (code === 255) {
            if (++p >= corpus.length) throw new Error("fsst esc");
            outLen++;
        } else {
            if (code >= symLens.length) throw new RangeError("fsst code");
            outLen += symLens[code];
        }
    }
    chargeBytes(outLen);
    return decodeFsst(symTbl, symLens, corpus);
}

export function sliceStrings(bytes: Uint8Array, lengths: Uint32Array): string[] {
    const bLen = bytes.length;
    const count = lengths.length;
    chargeBytes(bLen * 2);
    const entries = allocArray<string>(count);
    let pos = 0;
    for (let i = 0; i < count; i++) {
        const next = pos + lengths[i];
        if (next > bLen) throw new RangeError("str eof");
        entries[i] = decodeUtf8(bytes.subarray(pos, next));
        pos = next;
    }
    return entries;
}

function decodeDictEntries(front: number, lengths: Uint32Array, blobBytes: Uint8Array): string[] {
    if (front === 0) return sliceStrings(blobBytes, lengths);
    const { offsets, bytes } = decodeFrontCoded(lengths, blobBytes);
    const count = offsets.length - 1;
    chargeBytes(bytes.length * 2);
    const entries = allocArray<string>(count);
    for (let i = 0; i < count; i++) {
        entries[i] = decodeUtf8(bytes.subarray(offsets[i], offsets[i + 1]));
    }
    return entries;
}

export function parseStrings02(r: ByteReader, impliedCount: number): RawStringsStreamSet {
    if (r.p >= r.e) throw new RangeError("eof");
    const layout = r.b[r.p] & 3;
    const leadType = layout === 0 ? ST_LEN_VARBIN : layout === 2 ? ST_LEN_DICT : ST_OFF_STRING;
    const leading = parseStream02(r, FM_STR, 4, leadType, impliedCount, layout);
    const next = leading.n;
    if (layout === 0)
        return { m: 0, lengths: leading, data: parseStream02(r, FM_BYTES, 4, ST_DATA_NONE, next), n: next };
    if (layout === 1) {
        const lengths = parseStream02(r, FM_INT, 4, ST_LEN_DICT, next);
        const data = parseStream02(r, FM_BYTES, 4, ST_DATA_SINGLE, next);
        return { m: 1, offsets: leading, lengths, data, front: data.front, n: next };
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
    return { m: 3, offsets: leading, lengths, symLens, symTbl, corpus, front: corpus.front, n: next };
}

export function decodeStringsDense(raw: RawStringsStreamSet, ownedCodes = false): DecodedDenseStrings {
    if (raw.m === 0) return { dict: false, values: sliceStrings(raw.data.d, decodeStreamU32(raw.lengths, true)) };
    if (raw.m === 2) {
        const expanded = decodeFsstBytes(decodeStreamU32(raw.symLens, true), raw.symTbl.d, raw.corpus.d);
        return { dict: false, values: sliceStrings(expanded, decodeStreamU32(raw.lengths, true)) };
    }
    const blob =
        raw.m === 1 ? raw.data.d : decodeFsstBytes(decodeStreamU32(raw.symLens, true), raw.symTbl.d, raw.corpus.d);
    const entries = decodeDictEntries(raw.front, decodeStreamU32(raw.lengths, true), blob);
    const codes = decodeStreamU32(raw.offsets, !ownedCodes);
    for (let i = 0; i < codes.length; i++) if (codes[i] >= entries.length) throw new RangeError("dict code");
    return { dict: true, entries, codes };
}

export function chargeExpandedDictStrings(entries: readonly string[], codes: Uint32Array): void {
    let totalChars = 0;
    for (let i = 0; i < codes.length; i++) totalChars += entries[codes[i]].length;
    chargeBytes(totalChars * 2);
}

export function expandDenseStrings(sd: DecodedDenseStrings): string[] {
    if (!isDictDenseStrings(sd)) return sd.values;
    chargeExpandedDictStrings(sd.entries, sd.codes);
    const out = allocArray<string>(sd.codes.length);
    for (let i = 0; i < sd.codes.length; i++) out[i] = sd.entries[sd.codes[i]];
    return out;
}

export function materializeStringProperty(
    name: string,
    dense: DecodedDenseStrings,
    presenceBits: Uint8Array | undefined,
    featureCount: number,
): MltStringPropertyColumn {
    const denseLen = isDictDenseStrings(dense) ? dense.codes.length : dense.values.length;
    const pop = presenceBits === undefined ? featureCount : bitmapPopcount(presenceBits, featureCount);
    if (pop !== denseLen) throw new Error("str count");
    const present = normalizePresentBitmap(presenceBits, featureCount);

    if (isDictDenseStrings(dense)) {
        chargeExpandedDictStrings(dense.entries, dense.codes);
        const codes = allocU32(featureCount);
        if (present === undefined || presenceBits === undefined) {
            codes.set(dense.codes);
            return { name, kind: "strDict", codes, dictionary: dense.entries };
        }
        if (pop > 0) scatterPresent(presenceBits, featureCount, dense.codes, codes);
        return { name, kind: "strDict", present, codes, dictionary: dense.entries };
    }
    if (present === undefined || presenceBits === undefined) {
        return { name, kind: "str", values: dense.values };
    }
    const values = allocArray<string>(featureCount).fill("");
    if (pop > 0) scatterPresent(presenceBits, featureCount, dense.values, values);
    return { name, kind: "str", present, values };
}

export function decodeSharedDictColumns(shared: RawSharedDict, featureCount: number): MltStringPropertyColumn[] {
    const blob =
        "data" in shared
            ? shared.data.d
            : decodeFsstBytes(decodeStreamU32(shared.symLens, true), shared.symTbl.d, shared.corpus.d);
    const entries = decodeDictEntries(shared.front, decodeStreamU32(shared.lengths, true), blob);
    return shared.children.map((ch) => {
        const codes = decodeStreamU32(ch.data, true);
        for (let i = 0; i < codes.length; i++) if (codes[i] >= entries.length) throw new RangeError("sdict code");
        return materializeStringProperty(
            `${shared.prefix}${ch.suffix}`,
            { dict: true, entries, codes },
            ch.bits,
            featureCount,
        );
    });
}

export function stringCodesForMValue(dense: DecodedDenseStrings): { codes: Uint32Array; dictionary: string[] } {
    if (isDictDenseStrings(dense)) {
        chargeExpandedDictStrings(dense.entries, dense.codes);
        return { codes: dense.codes, dictionary: dense.entries };
    }
    const map = new Map<string, number>();
    const dictionary: string[] = [];
    const codes = allocU32(dense.values.length);
    for (let i = 0; i < dense.values.length; i++) {
        const s = dense.values[i];
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
