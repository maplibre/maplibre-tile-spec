import { describe, expect, it } from "vitest";
import { Int64SequenceVector } from "./int64SequenceVector";

describe("Int64SequenceVector", () => {
    it("generates a signed sequence base + index * delta", () => {
        const vec = new Int64SequenceVector("test", 10n, 5n, 5, true); // 10n, 15n, 20n, 25n, 30n
        expect(vec.getValue(0)).toBe(10n);
        expect(vec.getValue(2)).toBe(20n);
        expect(vec.getValue(4)).toBe(30n);
    });

    it("keeps a negative base when signed", () => {
        const vec = new Int64SequenceVector("test", -5n, 1n, 4, true);
        expect([0, 1, 2, 3].map((i) => vec.getValue(i))).toEqual([-5n, -4n, -3n, -2n]);
    });

    it("reinterprets the max base as unsigned", () => {
        // The decoder hands over the zig-zag bit pattern as a signed bigint (-1n).
        const vec = new Int64SequenceVector("test", -1n, 0n, 1, false);
        expect(vec.getValue(0)).toBe(18446744073709551615n);
    });

    it("reinterprets the high-bit base as unsigned", () => {
        // -(2^63) is the signed bit pattern for 2^63.
        const vec = new Int64SequenceVector("test", -(2n ** 63n), 0n, 1, false);
        expect(vec.getValue(0)).toBe(9223372036854775808n);
    });

    it("walks an unsigned sequence up to the max value", () => {
        // base -3n is the signed bit pattern for 2^64 - 3.
        const vec = new Int64SequenceVector("test", -3n, 1n, 3, false);
        expect([0, 1, 2].map((i) => vec.getValue(i))).toEqual([
            18446744073709551613n,
            18446744073709551614n,
            18446744073709551615n,
        ]);
    });

    it("exposes name and size", () => {
        const vec = new Int64SequenceVector("ids", 0n, 1n, 7, false);
        expect(vec.name).toBe("ids");
        expect(vec.size).toBe(7);
    });
});
