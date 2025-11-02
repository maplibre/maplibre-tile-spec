import { describe, it, expect } from "vitest";
import { FloatFlatVector } from "./floatFlatVector";

describe("FloatFlatVector", () => {
    it("should construct and return values correctly", () => {
        const data = new Float32Array([1.5, 2.5, 3.5, 4.5, 5.5]);
        const vec = new FloatFlatVector("test", data, data.length);

        expect(vec.name).toBe("test");
        expect(vec.size).toBe(5);
        expect(vec.getValue(0)).toBe(1.5);
        expect(vec.getValue(2)).toBe(3.5);
        expect(vec.getValue(4)).toBe(5.5);
    });
});
