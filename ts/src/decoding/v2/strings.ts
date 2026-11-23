import type { MltStringPropertyColumn } from "../../columns";
import { type ByteReader, allocArray, allocU32, chargeBytes, decodeUtf8 } from "./codecs/binary";
import { decodeFrontCoded, decodeFsstBytes } from "./codecs/fsst";
import { bitmapPopcount, normalizePresentBitmap, scatterPresent } from "./codecs/presence";
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
    | {
          m: 1;
          offsets: ParsedStream;
          lengths: ParsedStream;
          data: ParsedStream;
          front: number;
          n: number;
      }
    | {
          m: 2;
          lengths: ParsedStream;
          symLens: ParsedStream;
          symTbl: ParsedStream;
          corpus: ParsedStream;
          n: number;
      }
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
    | {
          prefix: string;
          front: number;
          fsst: 0;
          lengths: ParsedStream;
          data: ParsedStream;
          children: SharedDictChild[];
      }
    | {
          prefix: string;
          front: number;
          fsst: 1;
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

export function sliceStrings(bytes: Uint8Array, lengths: Uint32Array): string[] {
    const bLen = bytes.length;
    const count = lengths.length;
    chargeBytes(bLen * 2);
    const entries = allocArray<string>(count);
    let pos = 0;

    if (bLen <= 64) {
        let or = 0;
        for (let i = 0; i < bLen; i++) or |= bytes[i];
        if (or < 128) {
            let fullAscii = "";
            for (let i = 0; i < bLen; i++) fullAscii += String.fromCharCode(bytes[i]);
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

export function decodeDictEntries(front: number, rawLengths: Uint32Array, blobBytes: Uint8Array): string[] {
    if (front === 1) {
        const rebuilt = decodeFrontCoded(rawLengths, blobBytes);
        return sliceStrings(rebuilt.bytes, rebuilt.entryLengths);
    }
    return sliceStrings(blobBytes, rawLengths);
}

export function parseStrings02(r: ByteReader, impliedCount: number): RawStringsStreamSet {
    if (r.p >= r.e) throw new RangeError("eof");
    const layout = r.b[r.p] & 3;
    const leadType = layout === 0 ? ST_LEN_VARBIN : layout === 2 ? ST_LEN_DICT : ST_OFF_STRING;
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

export function decodeStringsDense(raw: RawStringsStreamSet, ownedCodes = false): DecodedDenseStrings {
    if (raw.m === 0) {
        return {
            dict: false,
            values: sliceStrings(raw.data.d, decodeStreamU32(raw.lengths, true)),
        };
    }
    if (raw.m === 2) {
        const expanded = decodeFsstBytes(decodeStreamU32(raw.symLens, true), raw.symTbl.d, raw.corpus.d);
        return {
            dict: false,
            values: sliceStrings(expanded, decodeStreamU32(raw.lengths, true)),
        };
    }
    const blob =
        raw.m === 1 ? raw.data.d : decodeFsstBytes(decodeStreamU32(raw.symLens, true), raw.symTbl.d, raw.corpus.d);
    const entries = decodeDictEntries(raw.front | 0, decodeStreamU32(raw.lengths, true), blob);
    const codes = decodeStreamU32(raw.offsets, !ownedCodes);
    const entryLen = entries.length;
    for (let i = 0; i < codes.length; i++) {
        if (codes[i] >= entryLen) throw new RangeError("dict code");
    }
    return { dict: true, entries, codes };
}

/**
 * Charge expanded UTF-16 byte footprint when expanding dictionary codes across rows,
 * preventing repeated references to a large dictionary entry from bypassing `maxBytes`.
 */
export function chargeExpandedDictStrings(entries: readonly string[], codes: Uint32Array): void {
    let totalChars = 0;
    for (let i = 0; i < codes.length; i++) {
        totalChars += entries[codes[i]].length;
    }
    chargeBytes(totalChars * 2);
}

export function materializeStringProperty(
    name: string,
    dense: DecodedDenseStrings,
    presenceBits: Uint8Array | undefined,
    featureCount: number,
): MltStringPropertyColumn {
    const denseLen = isDictDenseStrings(dense) ? dense.codes.length : dense.values.length;

    if (presenceBits === undefined) {
        if (denseLen !== featureCount) throw new Error("str count");
        if (isDictDenseStrings(dense)) {
            chargeExpandedDictStrings(dense.entries, dense.codes);
            const codes = allocU32(featureCount);
            codes.set(dense.codes);
            return { name, kind: "strDict", codes, dictionary: dense.entries };
        }
        return { name, kind: "str", values: dense.values };
    }

    const pop = bitmapPopcount(presenceBits, featureCount);
    if (pop !== denseLen) throw new Error("str popcount");
    const present = normalizePresentBitmap(presenceBits, featureCount);
    if (present === undefined) {
        if (isDictDenseStrings(dense)) {
            chargeExpandedDictStrings(dense.entries, dense.codes);
            const codes = allocU32(featureCount);
            codes.set(dense.codes);
            return { name, kind: "strDict", codes, dictionary: dense.entries };
        }
        return { name, kind: "str", values: dense.values };
    }

    if (isDictDenseStrings(dense)) {
        chargeExpandedDictStrings(dense.entries, dense.codes);
        const codes = allocU32(featureCount);
        if (pop > 0) {
            scatterPresent(presenceBits, featureCount, dense.codes, codes);
        }
        return { name, kind: "strDict", present, codes, dictionary: dense.entries };
    }

    const values = allocArray<string>(featureCount).fill("");
    if (pop > 0) {
        scatterPresent(presenceBits, featureCount, dense.values, values);
    }
    return { name, kind: "str", present, values };
}

export function decodeSharedDictColumns(shared: RawSharedDict, featureCount: number): MltStringPropertyColumn[] {
    const blob =
        "data" in shared
            ? shared.data.d
            : decodeFsstBytes(decodeStreamU32(shared.symLens, true), shared.symTbl.d, shared.corpus.d);
    const entries = decodeDictEntries(shared.front, decodeStreamU32(shared.lengths, true), blob);
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

export function stringCodesForMValue(dense: DecodedDenseStrings): {
    codes: Uint32Array;
    dictionary: string[];
} {
    if (isDictDenseStrings(dense)) {
        chargeExpandedDictStrings(dense.entries, dense.codes);
        return { codes: dense.codes, dictionary: dense.entries };
    }
    const map = new Map<string, number>();
    const dictionary: string[] = [];
    const vals = dense.values;
    const codes = allocU32(vals.length);
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
