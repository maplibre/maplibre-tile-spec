import {describe, expect, it} from "vitest";
import {LongSequenceVector} from "./longSequenceVector";

function getSelectionIndices(selection: { selectionValues: () => number[]; limit: number }): number[] {
    return selection.selectionValues().slice(0, selection.limit);
}

describe("LongSequenceVector", () => {
    it("should generate bigint sequence values", () => {
        const vec = new LongSequenceVector("test", 10n, 5n, 5); // 10n, 15n, 20n, 25n, 30n
        expect(vec.getValue(0)).toBe(10n);
        expect(vec.getValue(2)).toBe(20n);
        expect(vec.getValue(4)).toBe(30n);
    });

    it("should filter bigint sequence value", () => {
        const vec = new LongSequenceVector("test", 100n, 10n, 10);
        const result = vec.filter(120n);
        expect(result.limit).toBe(1);
        expect(getSelectionIndices(result)[0]).toBe(2);
    });
});
