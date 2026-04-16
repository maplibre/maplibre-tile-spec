import { describe, it, expect } from "vitest";
import { Int64FlatVector } from "./int64FlatVector";

describe("Int64FlatVector", () => {
    it("should construct and return values correctly", () => {
        const data = new BigInt64Array([10n, 20n, 30n, 40n, 50n]);
        const vec = new Int64FlatVector("test", data, data.length);

        expect(vec.name).toBe("test");
        expect(vec.size).toBe(5);
        expect(vec.getValue(0)).toBe(10n);
        expect(vec.getValue(2)).toBe(30n);
        expect(vec.getValue(4)).toBe(50n);
    });
});
