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
const INT64_MAX = 2n ** 63n - 1n;

export interface MapEncodingOptions {
    /**
     * Route every integer value through the unsigned dictionary rather than the signed one.
     * The encoder cannot tell a signed from an unsigned source type by looking at a JS value, so by
     * default only values too large for a signed 64-bit stream go to the unsigned dictionary.
     */
    unsignedIntegers?: boolean;
    /** Encode non-integer numbers as 32-bit floats instead of doubles. */
    singlePrecisionFloats?: boolean;
}

/**
 * The pieces of a map column, before they are written out. Mirrors `MapStreams` in the decoder,
 * except that the dictionary is still split by type: the encoder has to choose a stream per type,
 * whereas the decoder only ever sees them concatenated.
 */
interface MapStreams {
    /** Number of tokens each present feature contributes, laid out child-major. */
    lengthStream: number[];
    dictionaries: Dictionaries;
    /** Which features have a value at all, laid out child-major. */
    presenceBits: boolean[];
    /** Dictionary indices interleaved with control tokens. */
    flattenedValues: number[];
}

interface Dictionaries {
    strings: string[];
    /** Integers written as a signed stream, which the decoder reads ahead of the unsigned one. */
    signedIntegers: bigint[];
    unsignedIntegers: bigint[];
    decimals: number[];
    indexByKey: Map<string, number>;
}

/** How many streams a step wrote, and which mask bits it claims. */
interface WrittenStreams {
    mask: number;
    count: number;
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
export function encodeMapPropertyColumn(
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

    const streams: MapStreams = {
        lengthStream: [],
        dictionaries: collectDictionaries(childColumns, options),
        presenceBits: [],
        flattenedValues: [],
    };

    for (const column of childColumns) {
        encodeChildColumn(column, streams);
    }

    return encodeMapStreams(streams, options);
}

/**
 * Gathers the unique scalars of each type and assigns each one its index.
 *
 * This step has no counterpart in the decoder, which reads the dictionaries straight off the wire
 * in `decodeMapStreams`. Indices run across the concatenated dictionaries in the order the decoder
 * reads them — strings, signed integers, unsigned integers, then floating point — offset by the
 * reserved control values.
 */
function collectDictionaries(childColumns: (MapValue | null)[][], options: MapEncodingOptions): Dictionaries {
    const strings: string[] = [];
    const signedIntegers: bigint[] = [];
    const unsignedIntegers: bigint[] = [];
    const decimals: number[] = [];
    const seen = new Set<string>();

    // Only a value past the signed maximum needs the unsigned dictionary, so a column mixing
    // negative values with such a value ends up with one dictionary of each.
    const collectInteger = (value: bigint): void => {
        if (options.unsignedIntegers || value > INT64_MAX) unsignedIntegers.push(value);
        else signedIntegers.push(value);
    };

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
        else if (typeof value === "bigint") collectInteger(value);
        else if (isIntegral(value)) collectInteger(BigInt(value));
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
    for (const value of signedIntegers) indexByKey.set(dictionaryKey(value), index++);
    for (const value of unsignedIntegers) indexByKey.set(dictionaryKey(value), index++);
    for (const value of decimals) indexByKey.set(dictionaryKey(value), index++);

    return { strings, signedIntegers, unsignedIntegers, decimals, indexByKey };
}

/**
 * Writes one child column's per-feature values, the counterpart of `decodeChildColumn`.
 *
 * Lengths, presence bits and tokens are all laid out child-major, so each child appends to where
 * the previous one left off.
 */
function encodeChildColumn(column: (MapValue | null)[], streams: MapStreams): void {
    for (const value of column) {
        const present = value !== null;
        streams.presenceBits.push(present);
        if (!present) continue;

        const start = streams.flattenedValues.length;
        encodeFeatureValue(value, streams.flattenedValues, streams.dictionaries.indexByKey);
        streams.lengthStream.push(streams.flattenedValues.length - start);
    }
}

/**
 * A feature's payload is written as a bare sequence of map entries, unless it is a scalar — a
 * single token — or a list, which keeps its header. Those two shapes are what let
 * `decodeFeatureValue` tell them apart from map entries.
 */
function encodeFeatureValue(value: MapValue, flattenedValues: number[], indexByKey: Map<string, number>): void {
    if (!Array.isArray(value) && typeof value === "object") {
        encodeMapEntries(value, flattenedValues, indexByKey);
        return;
    }
    encodeValue(value, flattenedValues, indexByKey);
}

function encodeMapEntries(
    value: { [key: string]: MapValue },
    flattenedValues: number[],
    indexByKey: Map<string, number>,
): void {
    for (const [key, entry] of Object.entries(value)) {
        flattenedValues.push(encodeScalarByIndex(key, indexByKey));
        encodeValue(entry, flattenedValues, indexByKey);
    }
}

function encodeValue(value: MapValue, flattenedValues: number[], indexByKey: Map<string, number>): void {
    rejectNestedNull(value);

    if (typeof value === "boolean") {
        flattenedValues.push(value ? MapControlValue.TRUE : MapControlValue.FALSE);
        return;
    }

    if (Array.isArray(value)) {
        const startIndex = flattenedValues.length;
        flattenedValues.push(MapControlValue.START_LIST, 0);
        for (const entry of value) encodeValue(entry, flattenedValues, indexByKey);
        encodeNestedPayloadLength(flattenedValues, startIndex);
        return;
    }

    if (typeof value === "object") {
        const startIndex = flattenedValues.length;
        flattenedValues.push(MapControlValue.START_MAP, 0);
        encodeMapEntries(value, flattenedValues, indexByKey);
        encodeNestedPayloadLength(flattenedValues, startIndex);
        return;
    }

    flattenedValues.push(encodeScalarByIndex(value, indexByKey));
}

/**
 * Backfills the length of a nested payload once its end is known, the counterpart of
 * `decodeNestedPayloadEnd`. The length covers the two header tokens as well, so the decoder can
 * skip the whole payload without walking it.
 */
function encodeNestedPayloadLength(flattenedValues: number[], startIndex: number): void {
    flattenedValues[startIndex + 1] = flattenedValues.length - startIndex;
}

/** The counterpart of `decodeScalarByIndex`: turns a value back into its dictionary token. */
function encodeScalarByIndex(value: string | number | bigint, indexByKey: Map<string, number>): number {
    // The lookup always hits: collectDictionaries walked the very same values with this same key.
    return indexByKey.get(dictionaryKey(value));
}

/** Writes the stream mask and every stream it announces, the counterpart of `decodeMapStreams`. */
function encodeMapStreams(streams: MapStreams, options: MapEncodingOptions): { data: Uint8Array; numStreams: number } {
    const parts: Uint8Array[] = [];
    let mask = 0;
    let numStreams = 0;

    const write = (written: WrittenStreams): void => {
        mask |= written.mask;
        numStreams += written.count;
    };

    // The length stream is the only mandatory one.
    parts.push(encodeUint32Column(new Uint32Array(streams.lengthStream)));
    numStreams++;

    write(encodeStringDictionary(streams.dictionaries.strings, parts));
    write(encodeIntegerDictionaries(streams.dictionaries, parts));
    write(encodeFloatingPointDictionaries(streams.dictionaries.decimals, parts, options));
    write(encodePresenceStream(streams.presenceBits, parts));

    if (streams.flattenedValues.length > 0) {
        parts.push(encodeUint32Column(new Uint32Array(streams.flattenedValues)));
        numStreams++;
    }

    return { data: concatenateBuffers(new Uint8Array([mask]), ...parts), numStreams };
}

/** The counterpart of `decodeStringDictionary`. */
function encodeStringDictionary(strings: string[], parts: Uint8Array[]): WrittenStreams {
    if (strings.length === 0) {
        return { mask: 0, count: 0 };
    }

    // Plain strings are written as a LENGTH and a DATA stream.
    const stringStreamCount = 2;
    parts.push(new Uint8Array([stringStreamCount]));
    parts.push(encodePlainStrings(strings));

    return { mask: MapMask.STRING, count: stringStreamCount };
}

/**
 * Writes the signed dictionary and then the unsigned one, each as a single stream at the narrower
 * width that fits it, the counterpart of `decodeIntegerDictionaries`. Both are written when the
 * values need both, which is the order the decoder reads them in.
 */
function encodeIntegerDictionaries(dictionaries: Dictionaries, parts: Uint8Array[]): WrittenStreams {
    const signed = encodeSignedIntegerDictionary(dictionaries.signedIntegers, parts);
    const unsigned = encodeUnsignedIntegerDictionary(dictionaries.unsignedIntegers, parts);
    return { mask: signed.mask | unsigned.mask, count: signed.count + unsigned.count };
}

function encodeSignedIntegerDictionary(integers: bigint[], parts: Uint8Array[]): WrittenStreams {
    if (integers.length === 0) {
        return { mask: 0, count: 0 };
    }

    const wide = integers.some((value) => value < BigInt(INT32_MIN) || value > BigInt(INT32_MAX));
    parts.push(
        wide
            ? encodeInt64NoneColumn(BigInt64Array.from(integers))
            : encodeInt32NoneColumn(Int32Array.from(integers, Number)),
    );
    return { mask: wide ? MapMask.INT64 : MapMask.INT32, count: 1 };
}

function encodeUnsignedIntegerDictionary(integers: bigint[], parts: Uint8Array[]): WrittenStreams {
    if (integers.length === 0) {
        return { mask: 0, count: 0 };
    }

    const wide = integers.some((value) => value > BigInt(UINT32_MAX));
    parts.push(
        wide ? encodeUint64Column(BigUint64Array.from(integers)) : encodeUint32Column(Uint32Array.from(integers, Number)),
    );
    return { mask: wide ? MapMask.UINT64 : MapMask.UINT32, count: 1 };
}

/** The counterpart of `decodeFloatingPointDictionaries`. */
function encodeFloatingPointDictionaries(
    decimals: number[],
    parts: Uint8Array[],
    options: MapEncodingOptions,
): WrittenStreams {
    if (decimals.length === 0) {
        return { mask: 0, count: 0 };
    }

    if (options.singlePrecisionFloats) {
        parts.push(encodeFloatColumn(Float32Array.from(decimals)));
        return { mask: MapMask.FLOAT, count: 1 };
    }

    parts.push(encodeDoubleColumn(Float64Array.from(decimals)));
    return { mask: MapMask.DOUBLE, count: 1 };
}

/**
 * The counterpart of `decodePresenceStream`. Written only when some feature has no value at all,
 * which is what lets the decoder tell an absent property from an empty map.
 */
function encodePresenceStream(presenceBits: boolean[], parts: Uint8Array[]): WrittenStreams {
    if (!presenceBits.includes(false)) {
        return { mask: 0, count: 0 };
    }

    parts.push(
        createStream(PhysicalStreamType.PRESENT, encodeBooleanRle(presenceBits), { count: presenceBits.length }),
    );
    return { mask: MapMask.PRESENCE, count: 1 };
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
