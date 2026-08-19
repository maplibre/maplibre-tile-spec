import { describe, it, expect } from "vitest";
import { decodePropertyColumn } from "./propertyDecoder";
import { decodeMapPropertyColumn } from "./mapPropertyDecoder";
import IntWrapper from "./intWrapper";
import type Vector from "../vector/vector";
import { ColumnScope, ComplexType, ScalarType, type Column, type Field } from "../metadata/tileset/tilesetMetadata";
import { ObjectFlatVector } from "../vector/flat/objectFlatVector";
import { encodeMapPropertyColumn, type MapEncodingOptions, type MapValue } from "../encoding/mapPropertyEncoder";
import { MapControlValue } from "../metadata/tile/mapControlValue";
import { MapMask } from "../metadata/tile/mapMask";
import { PhysicalStreamType } from "../metadata/tile/physicalStreamType";
import { createStream } from "./decodingTestUtils";
import { concatenateBuffers, encodeBooleanRle } from "../encoding/encodingUtils";
import { encodeInt32NoneColumn, encodeUint32Column } from "../encoding/propertyEncoder";
import { encodePlainStrings } from "../encoding/stringEncoder";

function createMapColumnMetadata(name: string, childNames: string[] = []): Column {
    const children: Field[] = childNames.map((childName) => ({
        name: childName,
        nullable: true,
        type: "complexField" as const,
        complexField: {
            physicalType: ComplexType.MAP,
            type: "physicalType" as const,
            children: [],
        },
    }));

    return {
        name,
        nullable: true,
        type: "complexType",
        complexType: {
            physicalType: ComplexType.MAP,
            type: "physicalType",
            children,
        },
    };
}

/** Encodes the columns and decodes them again, returning one vector per column. */
function roundTripVectors(
    childColumns: (MapValue | null)[][],
    options: MapEncodingOptions = {},
    childNames: string[] = [],
): Vector[] {
    const { data, numStreams } = encodeMapPropertyColumn(childColumns, options);
    const columnMetadata = createMapColumnMetadata("a", childNames);
    const offset = new IntWrapper(0);

    const result = decodePropertyColumn(data, offset, columnMetadata, numStreams, childColumns[0].length);
    const vectors = result as Vector[];
    expect(vectors).toHaveLength(childColumns.length);
    for (const vector of vectors) {
        expect(vector).toBeInstanceOf(ObjectFlatVector);
    }

    return vectors;
}

/** Encodes the columns, decodes them again, and returns the per-feature values of each column. */
function roundTrip(
    childColumns: (MapValue | null)[][],
    options: MapEncodingOptions = {},
    childNames: string[] = [],
): unknown[][] {
    const featureCount = childColumns[0].length;
    return roundTripVectors(childColumns, options, childNames).map((vector) =>
        Array.from({ length: featureCount }, (_, i) => vector.getValue(i)),
    );
}

describe("map property column - root values", () => {
    it("round-trips a root scalar string", () => {
        expect(roundTrip([["b"]])).toEqual([["b"]]);
    });

    it("round-trips a root scalar per feature", () => {
        expect(roundTrip([["b", "c", "b"]])).toEqual([["b", "c", "b"]]);
    });

    it("round-trips a root map", () => {
        expect(roundTrip([[{ b: "c" }]])).toEqual([[{ b: "c" }]]);
    });

    it("round-trips a root list", () => {
        expect(roundTrip([[["a", "b", "c"]]])).toEqual([[["a", "b", "c"]]]);
    });

    it("round-trips an empty map", () => {
        expect(roundTrip([[{}]])).toEqual([[{}]]);
    });

    it("round-trips an empty list", () => {
        expect(roundTrip([[[]]])).toEqual([[[]]]);
    });

    it("round-trips mixed root shapes across features", () => {
        expect(roundTrip([["b", { b: "c" }, ["d"]]])).toEqual([["b", { b: "c" }, ["d"]]]);
    });
});

describe("map property column - nesting", () => {
    it("round-trips a map nested in a map", () => {
        expect(roundTrip([[{ a: { b: { c: "d" } } }]])).toEqual([[{ a: { b: { c: "d" } } }]]);
    });

    it("round-trips a list nested in a map", () => {
        expect(roundTrip([[{ a: ["b", "c"] }]])).toEqual([[{ a: ["b", "c"] }]]);
    });

    it("round-trips a list nested in a list", () => {
        expect(roundTrip([[[["a"], ["b", "c"]]]])).toEqual([[[["a"], ["b", "c"]]]]);
    });

    it("round-trips maps and lists interleaved, as the synthetic fixtures do", () => {
        const value: MapValue = { b: [1, "2", { c: [3.5, 4, [5, 6]] }] };
        expect(roundTrip([[value]])).toEqual([[value]]);
    });
});

describe("map property column - value types", () => {
    it("round-trips booleans, which are encoded as control values", () => {
        expect(roundTrip([[{ t: true, f: false }]])).toEqual([[{ t: true, f: false }]]);
    });

    it("round-trips signed 32-bit integers", () => {
        expect(roundTrip([[[0, -1, 2147483647, -2147483648]]])).toEqual([[[0, -1, 2147483647, -2147483648]]]);
    });

    it("round-trips signed 64-bit integers", () => {
        const values: MapValue = [-(2n ** 63n), 2n ** 62n, 5n];
        expect(roundTrip([[values]])).toEqual([[values]]);
    });

    it("round-trips unsigned 32-bit integers", () => {
        expect(roundTrip([[[0, 4294967295]]], { unsignedIntegers: true })).toEqual([[[0, 4294967295]]]);
    });

    it("round-trips unsigned 64-bit integers", () => {
        const values: MapValue = [0n, 2n ** 64n - 1n];
        expect(roundTrip([[values]], { unsignedIntegers: true })).toEqual([[values]]);
    });

    it("round-trips doubles", () => {
        expect(roundTrip([[[1.5, -2.25, 1e300]]])).toEqual([[[1.5, -2.25, 1e300]]]);
    });

    it("round-trips single-precision floats", () => {
        expect(roundTrip([[[1.5, -2.25]]], { singlePrecisionFloats: true })).toEqual([[[1.5, -2.25]]]);
    });

    it("round-trips non-finite doubles", () => {
        const values: MapValue = [Number.NaN, Number.POSITIVE_INFINITY, Number.NEGATIVE_INFINITY];
        expect(roundTrip([[values]])).toEqual([[values]]);
    });

    it("keeps a string and the integer of the same spelling apart", () => {
        expect(roundTrip([[{ a: "1", b: 1 }]])).toEqual([[{ a: "1", b: 1 }]]);
    });

    it("keeps a __proto__ key as an entry instead of reassigning the prototype", () => {
        // A computed key, so the literal stores an own property rather than setting the prototype.
        const [[decoded]] = roundTrip([[{ ["__proto__"]: "c" }]]);

        expect(Object.getPrototypeOf(decoded)).toBeNull();
        expect(Object.getOwnPropertyDescriptor(decoded as object, "__proto__")?.value).toBe("c");
    });

    it("reuses one dictionary entry for a value repeated across features", () => {
        const { data } = encodeMapPropertyColumn([[{ a: "shared" }, { b: "shared" }]]);
        const occurrences = Buffer.from(data).toString("latin1").split("shared").length - 1;
        expect(occurrences).toBe(1);
    });
});

describe("map property column - absent values", () => {
    it("round-trips a feature whose property is absent", () => {
        expect(roundTrip([[{ b: "c" }, null]])).toEqual([[{ b: "c" }, null]]);
    });

    it("round-trips a column that is absent for every feature", () => {
        expect(roundTrip([[null, null]])).toEqual([[null, null]]);
    });

    it("distinguishes an absent value from an empty map", () => {
        expect(roundTrip([[null, {}]])).toEqual([[null, {}]]);
    });

    it("reports an absent property as missing", () => {
        const [vector] = roundTripVectors([[{ b: "c" }, null]]);

        expect(vector.has(0)).toBe(true);
        expect(vector.getValue(0)).toEqual({ b: "c" });
        expect(vector.has(1)).toBe(false);
        expect(vector.getValue(1)).toBeNull();
    });

    it("reports every feature as present when the column carries no presence stream", () => {
        const [vector] = roundTripVectors([[{ b: "c" }, "d"]]);

        expect(vector.has(0)).toBe(true);
        expect(vector.has(1)).toBe(true);
    });

    it("tracks presence per child column", () => {
        const [one, two] = roundTripVectors([[{ b: "c" }, null], [null, "d"]], {}, ["one", "two"]);

        expect([one.has(0), one.has(1)]).toEqual([true, false]);
        expect([two.has(0), two.has(1)]).toEqual([false, true]);
        expect([two.getValue(0), two.getValue(1)]).toEqual([null, "d"]);
    });
});

describe("map property column - shared child columns", () => {
    it("round-trips two child columns sharing one dictionary", () => {
        const decoded = roundTrip(
            [
                [{ b: "c" }, "d"],
                [["e"], null],
            ],
            {},
            ["one", "two"],
        );
        expect(decoded).toEqual([
            [{ b: "c" }, "d"],
            [["e"], null],
        ]);
    });

    it("names child columns by appending the child name to the parent name", () => {
        const { data, numStreams } = encodeMapPropertyColumn([[{ b: "c" }], [{ d: "e" }]]);
        const columnMetadata = createMapColumnMetadata("name:", ["one", "two"]);
        const result = decodePropertyColumn(data, new IntWrapper(0), columnMetadata, numStreams, 1) as Vector[];

        expect(result.map((vector) => vector.name)).toEqual(["name:one", "name:two"]);
    });

    it("uses the column name itself when there are no children", () => {
        const { data, numStreams } = encodeMapPropertyColumn([[{ b: "c" }]]);
        const columnMetadata = createMapColumnMetadata("a");
        const result = decodePropertyColumn(data, new IntWrapper(0), columnMetadata, numStreams, 1) as Vector[];

        expect(result.map((vector) => vector.name)).toEqual(["a"]);
    });
});

describe("map property column - encoder validation", () => {
    it("rejects a null nested inside a value", () => {
        expect(() => encodeMapPropertyColumn([[{ a: null as unknown as MapValue }]])).toThrow(
            /Nested null values cannot be encoded/,
        );
    });

    it("rejects child columns of differing lengths", () => {
        expect(() => encodeMapPropertyColumn([[{ a: "b" }], []])).toThrow(/same number of features/);
    });

    it("rejects an empty set of child columns", () => {
        expect(() => encodeMapPropertyColumn([])).toThrow(/at least one child column/);
    });
});

describe("map property column - decoder validation", () => {
    it("reports a truncated stream set", () => {
        const { data, numStreams } = encodeMapPropertyColumn([[{ b: "c" }]]);
        expect(() => decodePropertyColumn(data, new IntWrapper(0), createMapColumnMetadata("a"), numStreams + 1, 1)) //
            .toThrow(/remaining streams/);
    });

    it("returns empty columns when the column carries no streams", () => {
        const result = decodePropertyColumn(
            new Uint8Array(0),
            new IntWrapper(0),
            createMapColumnMetadata("a"),
            0,
            0,
        ) as Vector[];

        expect(result).toHaveLength(1);
        expect(result[0].size).toBe(0);
    });
});

/**
 * Assembles a map column stream by stream, so a test can hand the decoder a payload the encoder
 * would never produce. The dictionary is `strings` followed by `integers`, matching the order the
 * decoder reads them, so a token of `MapControlValue.COUNT + n` selects the n-th entry.
 */
function encodeRawMapColumn(parts: {
    strings?: string[];
    integers?: number[];
    lengths?: number[];
    tokens?: number[];
    presence?: boolean[];
    presenceStreamType?: PhysicalStreamType;
    stringStreamCount?: number;
}): { data: Uint8Array; numStreams: number } {
    const { strings = [], integers = [], lengths = [], tokens = [], presence, presenceStreamType } = parts;
    const streams: Uint8Array[] = [];
    let numStreams = 0;
    let mask = 0;

    streams.push(encodeUint32Column(new Uint32Array(lengths)));
    numStreams++;

    if (strings.length > 0 || parts.stringStreamCount !== undefined) {
        mask |= MapMask.STRING;
        const stringStreamCount = parts.stringStreamCount ?? 2;
        streams.push(new Uint8Array([stringStreamCount]));
        if (stringStreamCount > 0) streams.push(encodePlainStrings(strings));
        numStreams += stringStreamCount;
    }

    if (integers.length > 0) {
        mask |= MapMask.INT32;
        streams.push(encodeInt32NoneColumn(Int32Array.from(integers)));
        numStreams++;
    }

    if (presence) {
        mask |= MapMask.PRESENCE;
        streams.push(
            createStream(presenceStreamType ?? PhysicalStreamType.PRESENT, encodeBooleanRle(presence), {
                count: presence.length,
            }),
        );
        numStreams++;
    }

    if (tokens.length > 0) {
        streams.push(encodeUint32Column(new Uint32Array(tokens)));
        numStreams++;
    }

    return { data: concatenateBuffers(new Uint8Array([mask]), ...streams), numStreams };
}

function decodeRaw(parts: Parameters<typeof encodeRawMapColumn>[0], columnMetadata?: Column): Vector[] {
    const { data, numStreams } = encodeRawMapColumn(parts);
    const metadata = columnMetadata ?? createMapColumnMetadata("a");
    return decodeMapPropertyColumn(data, new IntWrapper(0), metadata, numStreams);
}

const FIRST = MapControlValue.COUNT;

describe("map property column - malformed streams", () => {
    it("rejects a presence stream that is not a PRESENT stream", () => {
        expect(() =>
            decodeRaw({
                strings: ["a"],
                lengths: [1],
                tokens: [FIRST],
                presence: [true],
                presenceStreamType: PhysicalStreamType.DATA,
            }),
        ).toThrow(/Expected PRESENT stream for map column/);
    });

    it("rejects fewer lengths than the presence stream announces", () => {
        expect(() => decodeRaw({ strings: ["a"], lengths: [1], tokens: [FIRST], presence: [true, true] })).toThrow(
            /Merged map counts underflow/,
        );
    });

    it("rejects a feature length running past the token stream", () => {
        expect(() => decodeRaw({ strings: ["a"], lengths: [5], tokens: [FIRST] })).toThrow(
            /Map value stream underflow/,
        );
    });

    it("rejects a payload that leaves tokens unconsumed", () => {
        // A three-token feature holding a list that only covers two of them.
        expect(() =>
            decodeRaw({ strings: ["a"], lengths: [3], tokens: [MapControlValue.START_LIST, 2, FIRST] }),
        ).toThrow(/Unused flattened map values remain/);
    });

    it("rejects a map key that is not a string", () => {
        // Two tokens, so the payload decodes as map entries, with an integer in key position.
        expect(() => decodeRaw({ strings: ["a"], integers: [7], lengths: [2], tokens: [FIRST + 1, FIRST] })).toThrow(
            /Map key dictionary index does not resolve to a string/,
        );
    });

    it("rejects a map entry whose key has no value", () => {
        expect(() => decodeRaw({ strings: ["a", "b"], lengths: [3], tokens: [FIRST, FIRST + 1, FIRST] })).toThrow(
            /Unexpected end of map value stream/,
        );
    });

    it("rejects a nested payload with no length", () => {
        expect(() => decodeRaw({ strings: ["a"], lengths: [2], tokens: [FIRST, MapControlValue.START_MAP] })).toThrow(
            /Missing length for nested map\/list payload/,
        );
    });

    it("rejects a nested payload length below the header size", () => {
        expect(() =>
            decodeRaw({ strings: ["a"], lengths: [3], tokens: [MapControlValue.START_LIST, 0, FIRST] }),
        ).toThrow(/Invalid nested payload length: 0/);
    });

    it("rejects a nested payload running past its container", () => {
        expect(() => decodeRaw({ strings: ["a"], lengths: [2], tokens: [MapControlValue.START_LIST, 99] })).toThrow(
            /Nested payload exceeds containing payload bounds/,
        );
    });

    it("rejects a dictionary index past the end of the dictionary", () => {
        expect(() => decodeRaw({ strings: ["a"], lengths: [1], tokens: [999] })).toThrow(
            /Scalar dictionary index out of range: 999/,
        );
    });

    it("rejects a reserved control token in map-key position", () => {
        // Key tokens go straight to the dictionary, so a control value lands below index zero.
        expect(() => decodeRaw({ strings: ["a"], lengths: [2], tokens: [MapControlValue.FALSE, FIRST] })).toThrow(
            /Scalar dictionary index out of range: 0/,
        );
    });

    it("tolerates a string mask with no string streams", () => {
        const [vector] = decodeRaw({ stringStreamCount: 0, lengths: [0], tokens: [] });
        expect(vector.getValue(0)).toEqual({});
    });
});

describe("map property column - column naming", () => {
    it("falls back to the column name when the metadata is not a complex column", () => {
        const scalarColumn: Column = {
            name: "plain",
            nullable: true,
            columnScope: ColumnScope.FEATURE,
            type: "scalarType",
            scalarType: { longID: false, type: "physicalType", physicalType: ScalarType.STRING },
        };
        const [vector] = decodeRaw({ strings: ["a"], lengths: [1], tokens: [FIRST] }, scalarColumn);

        expect(vector.name).toBe("plain");
    });

    it("treats a child without a name as contributing nothing to the name", () => {
        const columnMetadata = createMapColumnMetadata("parent");
        (columnMetadata.complexType as { children: Field[] }).children = [
            { nullable: true, type: "scalarField", scalarField: { physicalType: ScalarType.STRING } },
        ];
        const [vector] = decodeRaw({ strings: ["a"], lengths: [1], tokens: [FIRST] }, columnMetadata);

        expect(vector.name).toBe("parent");
    });
});
