import { describe, it, expect } from "vitest";
import { IntSequenceVector } from "./intSequenceVector";

function getSelectionIndices(selection: { selectionValues: () => number[]; limit: number }): number[] {
    return selection.selectionValues().slice(0, selection.limit);
}

describe("IntSequenceVector", () => {
    it("should generate sequence values", () => {
        const vec = new IntSequenceVector("test", 10, 5, 5); // 10, 15, 20, 25, 30
        expect(vec.size).toBe(5);
        expect(vec.getValue(0)).toBe(10);
        expect(vec.getValue(1)).toBe(15);
        expect(vec.getValue(2)).toBe(20);
        expect(vec.getValue(4)).toBe(30);
    });

    it("should filter sequence value", () => {
        const vec = new IntSequenceVector("test", 100, 10, 10); // 100, 110, 120...
        const result = vec.filter(120);
        expect(result.limit).toBe(1);
        expect(getSelectionIndices(result)[0]).toBe(2);
    });

    it("should return empty for non-sequence value", () => {
        const vec = new IntSequenceVector("test", 100, 10, 10);
        const result = vec.filter(125); // Not in sequence
        expect(result.limit).toBe(0);
    });

    it("should throw for not implemented methods", () => {
        const vec = new IntSequenceVector("test", 10, 5, 5);
        expect(() => vec.match([10, 15])).toThrow("Not implemented yet.");
        expect(() => vec.greaterThanOrEqualTo(15)).toThrow("Not implemented yet.");
    });
});