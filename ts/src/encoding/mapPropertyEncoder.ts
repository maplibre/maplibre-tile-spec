import { PhysicalStreamType } from "../metadata/tile/physicalStreamType";
import { MapMask } from "../metadata/tile/mapMask";
import { MapControlValue } from "../metadata/tile/mapControlValue";
import { createStream } from "../decoding/decodingTestUtils";
import { concatenateBuffers, encodeBooleanRle } from "./encodingUtils";
import { encodePlainStrings } from "./stringEncoder";
import {
    encodeDoubleColumn,
    encodeFloatColumn,
    encodeInt32NoneColumn,
    encodeInt64NoneColumn,
    encodeUint32Column,
    encodeUint64Column,
} from "./propertyEncoder";

/** A value that can appear inside a nested property column. `null` means the property is absent. */
export type MapValue = string | number | bigint | boolean | MapValue[] | { [key: string]: MapValue };

const INT32_MIN = -(2 ** 31);
const INT32_MAX = 2 ** 31 - 1;
const UINT32_MAX = 2 ** 32 - 1;

export interface MapEncodingOptions {
    /**
     * Route integer values through the unsigned dictionary rather than the signed one.
     * The encoder cannot tell a signed from an unsigned source type by looking at a JS value.
     */
    unsignedIntegers?: boolean;
    /** Encode non-integer numbers as 32-bit floats instead of doubles. */
    singlePrecisionFloats?: boolean;
}

/**
 * Encodes nested property (MAP) columns, the inverse of `decodeMapPropertyColumn`.
 *
 * Takes one array of per-feature values per child column — a single column for a standalone map,
 * several when the dictionaries are shared between sibling columns. A `null` entry marks the
 * property as absent for that feature, which is the only form of null the format can express;
 * nulls nested inside a map or list are rejected.
 *
 * Values are collected into one dictionary per type in first-seen order, and the structure is
 * flattened into a token stream of dictionary indices interleaved with control values. Booleans are
 * encoded directly as control values rather than being added to a dictionary.
 *
 * @returns the encoded column and the stream count the decoder must be given.
 */
export function encodeMapColumn(
    childColumns: (MapValue | null)[][],
    options: MapEncodingOptions = {},
): { data: Uint8Array; numStreams: number } {
    if (childColumns.length === 0) {
        throw new Error("A map column needs at least one child column");
    }
    const featureCount = childColumns[0].length;
    if (childColumns.some((column) => column.length !== featureCount)) {
        throw new Error("All child columns must hold the same number of features");
    }

    const dictionaries = collectDictionaries(childColumns);
    // The lookup always hits: collectDictionaries walked the very same values with this same key.
    const indexOf = (value: string | number | bigint): number =>
        dictionaries.indexByKey.get(dictionaryKey(value));

    // Lengths, presence bits and tokens are all laid out child-major.
    const featureValueCounts: number[] = [];
    const presenceBits: boolean[] = [];
    const flattenedValues: number[] = [];
    let hasAbsentValues = false;

    for (const column of childColumns) {
        for (const value of column) {
            const present = value !== null;
            presenceBits.push(present);
            if (!present) {
                hasAbsentValues = true;
                continue;
            }
            const start = flattenedValues.length;
            flattenRootValue(value, flattenedValues, indexOf);
            featureValueCounts.push(flattenedValues.length - start);
        }
    }

    const streams: Uint8Array[] = [];
    let numStreams = 0;
    let mask = 0;

    // The length stream is the only mandatory one.
    streams.push(encodeUint32Column(new Uint32Array(featureValueCounts)));
    numStreams++;

    if (dictionaries.strings.length > 0) {
        mask |= MapMask.STRING;
        // Plain strings are written as a LENGTH and a DATA stream.
        const stringStreamCount = 2;
        streams.push(new Uint8Array([stringStreamCount]));
        streams.push(encodePlainStrings(dictionaries.strings));
        numStreams += stringStreamCount;
    }

    if (dictionaries.integers.length > 0) {
        const wide = dictionaries.integers.some((value) =>
            options.unsignedIntegers ? value > BigInt(UINT32_MAX) : value < BigInt(INT32_MIN) || value > BigInt(INT32_MAX),
        );
        if (options.unsignedIntegers) {
            mask |= wide ? MapMask.UINT64 : MapMask.UINT32;
            streams.push(
                wide
                    ? encodeUint64Column(BigUint64Array.from(dictionaries.integers))
                    : encodeUint32Column(Uint32Array.from(dictionaries.integers, Number)),
            );
        } else {
            mask |= wide ? MapMask.INT64 : MapMask.INT32;
            streams.push(
                wide
                    ? encodeInt64NoneColumn(BigInt64Array.from(dictionaries.integers))
                    : encodeInt32NoneColumn(Int32Array.from(dictionaries.integers, Number)),
            );
        }
        numStreams++;
    }

    if (dictionaries.decimals.length > 0) {
        if (options.singlePrecisionFloats) {
            mask |= MapMask.FLOAT;
            streams.push(encodeFloatColumn(Float32Array.from(dictionaries.decimals)));
        } else {
            mask |= MapMask.DOUBLE;
            streams.push(encodeDoubleColumn(Float64Array.from(dictionaries.decimals)));
        }
        numStreams++;
    }

    if (hasAbsentValues) {
        mask |= MapMask.PRESENCE;
        streams.push(
            createStream(PhysicalStreamType.PRESENT, encodeBooleanRle(presenceBits), { count: presenceBits.length }),
        );
        numStreams++;
    }

    if (flattenedValues.length > 0) {
        streams.push(encodeUint32Column(new Uint32Array(flattenedValues)));
        numStreams++;
    }

    return { data: concatenateBuffers(new Uint8Array([mask]), ...streams), numStreams };
}

interface Dictionaries {
    strings: string[];
    integers: bigint[];
    decimals: number[];
    indexByKey: Map<string, number>;
}

/**
 * Walks every value to gather the unique scalars of each type, then assigns each one its index.
 * Indices run across the concatenated dictionaries in the order the decoder reads them —
 * strings, integers, then floating point — offset by the reserved control values.
 */
function collectDictionaries(childColumns: (MapValue | null)[][]): Dictionaries {
    const strings: string[] = [];
    const integers: bigint[] = [];
    const decimals: number[] = [];
    const seen = new Set<string>();

    const collect = (value: MapValue): void => {
        rejectNestedNull(value);
        if (typeof value === "boolean") return;
        if (Array.isArray(value)) {
            for (const entry of value) collect(entry);
            return;
        }
        if (typeof value === "object") {
            for (const [key, entry] of Object.entries(value)) {
                collect(key);
                collect(entry);
            }
            return;
        }

        const key = dictionaryKey(value);
        if (seen.has(key)) return;
        seen.add(key);

        if (typeof value === "string") strings.push(value);
        else if (typeof value === "bigint") integers.push(value);
        else if (isIntegral(value)) integers.push(BigInt(value));
        else decimals.push(value);
    };

    for (const column of childColumns) {
        for (const value of column) {
            if (value === null) continue;
            collect(value);
        }
    }

    const indexByKey = new Map<string, number>();
    let index = MapControlValue.COUNT;
    for (const value of strings) indexByKey.set(dictionaryKey(value), index++);
    for (const value of integers) indexByKey.set(dictionaryKey(value), index++);
    for (const value of decimals) indexByKey.set(dictionaryKey(value), index++);

    return { strings, integers, decimals, indexByKey };
}

/**
 * The format has no token for null: a null is only meaningful as a whole feature value, where it is
 * carried by the presence stream. Reject it anywhere else rather than encoding something lossy.
 */
function rejectNestedNull(value: MapValue): void {
    if (value === null || value === undefined) {
        throw new Error("Nested null values cannot be encoded; null is only valid as a whole feature value");
    }
}

/** Distinguishes values that would otherwise collide, e.g. the string "1" from the integer 1. */
function dictionaryKey(value: string | number | bigint): string {
    if (typeof value === "string") return `s:${value}`;
    if (typeof value === "bigint") return `i:${value}`;
    return isIntegral(value) ? `i:${BigInt(value)}` : `d:${value}`;
}

/**
 * Whether a number should go into an integer dictionary. `Number.isInteger` is not enough: it also
 * accepts magnitudes like 1e300, which no integer stream can hold and which a double represents
 * just as exactly, so anything beyond the safe-integer range belongs with the floating point values.
 */
function isIntegral(value: number): boolean {
    return Number.isSafeInteger(value);
}

/**
 * A root value is stored without a wrapping control token: a map becomes bare entries, a scalar a
 * single token. Only a list keeps its header, which is what lets the decoder tell the two apart.
 */
function flattenRootValue(value: MapValue, out: number[], indexOf: (value: string | number | bigint) => number): void {
    if (!Array.isArray(value) && value !== null && typeof value === "object") {
        flattenMapEntries(value, out, indexOf);
        return;
    }
    flattenValue(value, out, indexOf);
}

function flattenValue(value: MapValue, out: number[], indexOf: (value: string | number | bigint) => number): void {
    rejectNestedNull(value);
    if (typeof value === "boolean") {
        out.push(value ? MapControlValue.TRUE : MapControlValue.FALSE);
        return;
    }
    if (Array.isArray(value)) {
        // The length covers the two header tokens as well, so the decoder can skip the whole payload.
        const start = out.length;
        out.push(MapControlValue.START_LIST, 0);
        for (const entry of value) flattenValue(entry, out, indexOf);
        out[start + 1] = out.length - start;
        return;
    }
    if (typeof value === "object") {
        const start = out.length;
        out.push(MapControlValue.START_MAP, 0);
        flattenMapEntries(value, out, indexOf);
        out[start + 1] = out.length - start;
        return;
    }
    out.push(indexOf(value));
}

function flattenMapEntries(
    value: { [key: string]: MapValue },
    out: number[],
    indexOf: (value: string | number | bigint) => number,
): void {
    for (const [key, entry] of Object.entries(value)) {
        out.push(indexOf(key));
        flattenValue(entry, out, indexOf);
    }
}
