import { describe, it, expect } from "vitest";
import { LongFlatVector } from "./longFlatVector";

describe("LongFlatVector", () => {
    it("should construct and return values correctly", () => {
        const data = new BigInt64Array([10n, 20n, 30n, 40n, 50n]);
        const vec = new LongFlatVector("test", data, data.length);

        expect(vec.name).toBe("test");
        expect(vec.size).toBe(5);
        expect(vec.getValue(0)).toBe(10n);
        expect(vec.getValue(2)).toBe(30n);
        expect(vec.getValue(4)).toBe(50n);
    });
});
