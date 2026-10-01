import { describe, expect, it } from "vitest";
import { Int32SequenceVector } from "./int32SequenceVector";

describe("Int32SequenceVector", () => {
    it("generates a signed sequence base + index * delta", () => {
        const vec = new Int32SequenceVector("test", 10, 5, 5, true); // 10, 15, 20, 25, 30
        expect(vec.getValue(0)).toBe(10);
        expect(vec.getValue(2)).toBe(20);
        expect(vec.getValue(4)).toBe(30);
    });

    it("keeps a negative base when signed", () => {
        const vec = new Int32SequenceVector("test", -5, 1, 4, true);
        expect([0, 1, 2, 3].map((i) => vec.getValue(i))).toEqual([-5, -4, -3, -2]);
    });

    it("reinterprets the max base as unsigned", () => {
        // The decoder hands over the zig-zag bit pattern as a signed number (-1).
        const vec = new Int32SequenceVector("test", -1, 0, 1, false);
        expect(vec.getValue(0)).toBe(4294967295);
    });

    it("reinterprets the high-bit base as unsigned", () => {
        // -2147483648 is the signed bit pattern for 2^31.
        const vec = new Int32SequenceVector("test", -2147483648, 0, 1, false);
        expect(vec.getValue(0)).toBe(2147483648);
    });

    it("walks an unsigned sequence up to the max value", () => {
        // base -3 is the signed bit pattern for 4294967293.
        const vec = new Int32SequenceVector("test", -3, 1, 3, false);
        expect([0, 1, 2].map((i) => vec.getValue(i))).toEqual([4294967293, 4294967294, 4294967295]);
    });

    it("exposes name and size", () => {
        const vec = new Int32SequenceVector("ids", 0, 1, 7, false);
        expect(vec.name).toBe("ids");
        expect(vec.size).toBe(7);
    });
});
