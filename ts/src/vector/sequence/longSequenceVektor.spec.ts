import { describe, expect, it } from "vitest";
import { LongSequenceVector } from "./longSequenceVector";

describe("LongSequenceVector", () => {
    it("should generate bigint sequence values", () => {
        const vec = new LongSequenceVector("test", 10n, 5n, 5); // 10n, 15n, 20n, 25n, 30n
        expect(vec.getValue(0)).toBe(10n);
        expect(vec.getValue(2)).toBe(20n);
        expect(vec.getValue(4)).toBe(30n);
    });
});
