import { describe, it, expect } from "vitest";
import { LongConstVector } from "./longConstVector";
import BitVector from "../flat/bitVector";
import { FlatSelectionVector } from "../filter/flatSelectionVector";

function getSelectionIndices(selection: { selectionValues: () => number[]; limit: number }): number[] {
    return selection.selectionValues().slice(0, selection.limit);
}

describe("LongConstVector", () => {
    it("should return constant value", () => {
        const bitVector = new BitVector(new Uint8Array([0xFF, 0xFF]), 10);
        const vec = new LongConstVector("test", 42n, bitVector);
        expect(vec.size).toBe(10);
        expect(vec.getValue(0)).toBe(42n);
        expect(vec.getValue(5)).toBe(42n);
        expect(vec.getValue(9)).toBe(42n);
    });

    it("should filter matching constant", () => {
        const bitVector = new BitVector(new Uint8Array([0xFF, 0xFF]), 10);
        const vec = new LongConstVector("test", 42n, bitVector);
        const result = vec.filter(42n);
        expect(result.limit).toBe(10);
        expect(getSelectionIndices(result)).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    });

    it("should return empty when filtering non-matching value", () => {
        const bitVector = new BitVector(new Uint8Array([0xFF, 0xFF]), 10);
        const vec = new LongConstVector("test", 42n, bitVector);
        const result = vec.filter(99n);
        expect(result.limit).toBe(0);
    });

    it("should match when value in array", () => {
        const bitVector = new BitVector(new Uint8Array([0xFF]), 5);
        const vec = new LongConstVector("test", 42n, bitVector);
        expect(vec.match([10n, 42n, 99n]).limit).toBe(5);
        expect(vec.match([10n, 99n]).limit).toBe(0);
    });

    it("should filter selected", () => {
        const bitVector = new BitVector(new Uint8Array([0xFF, 0xFF]), 10);
        const vec = new LongConstVector("test", 42n, bitVector);
        const selection = new FlatSelectionVector([2, 3, 4, 5]);
        vec.filterSelected(42n, selection);
        expect(getSelectionIndices(selection)).toEqual([2, 3, 4, 5]);

        const selection2 = new FlatSelectionVector([2, 3, 4]);
        vec.filterSelected(99n, selection2);
        expect(selection2.limit).toBe(0);
    });

    it("should match selected", () => {
        const bitVector = new BitVector(new Uint8Array([0xFF, 0xFF]), 10);
        const vec = new LongConstVector("test", 42n, bitVector);
        const selection = new FlatSelectionVector([2, 3, 4]);
        vec.matchSelected([10n, 42n], selection);
        expect(getSelectionIndices(selection)).toEqual([2, 3, 4]);
    });

    it("should handle comparison operations", () => {
        const bitVector = new BitVector(new Uint8Array([0xFF]), 5);
        const vec = new LongConstVector("test", 50n, bitVector);
        expect(vec.greaterThanOrEqualTo(50n).limit).toBe(5);
        expect(vec.greaterThanOrEqualTo(51n).limit).toBe(0);
        expect(vec.smallerThanOrEqualTo(50n).limit).toBe(5);
        expect(vec.smallerThanOrEqualTo(49n).limit).toBe(0);
    });

    it("should handle comparison selected", () => {
        const bitVector = new BitVector(new Uint8Array([0xFF, 0xFF]), 10);
        const vec = new LongConstVector("test", 50n, bitVector);
        const sel1 = new FlatSelectionVector([1, 2, 3]);
        vec.greaterThanOrEqualToSelected(50n, sel1);
        expect(getSelectionIndices(sel1)).toEqual([1, 2, 3]);

        const sel2 = new FlatSelectionVector([1, 2, 3]);
        vec.greaterThanOrEqualToSelected(51n, sel2);
        expect(sel2.limit).toBe(0);
    });

    it("should filter not equal", () => {
        const bitVector = new BitVector(new Uint8Array([0xFF]), 5);
        const vec = new LongConstVector("test", 42n, bitVector);
        expect(vec.filterNotEqual(99n).limit).toBe(5);
        expect(vec.filterNotEqual(42n).limit).toBe(0);
    });

    it("should filter not equal selected", () => {
        const bitVector = new BitVector(new Uint8Array([0xFF, 0xFF]), 10);
        const vec = new LongConstVector("test", 42n, bitVector);
        const selection = new FlatSelectionVector([2, 3, 4]);
        vec.filterNotEqualSelected(99n, selection);
        expect(getSelectionIndices(selection)).toEqual([2, 3, 4]);

        const selection2 = new FlatSelectionVector([2, 3, 4]);
        vec.filterNotEqualSelected(42n, selection2);
        expect(selection2.limit).toBe(0);
    });

    it("should handle nullability", () => {
        const bitVector = new BitVector(new Uint8Array([0b01010101]), 8); // 0,2,4,6 present
        const vec = new LongConstVector("test", 42n, bitVector);
        const result = vec.filter(42n);
        expect(result.limit).toBe(4);
        expect(getSelectionIndices(result)).toEqual([0, 2, 4, 6]);
    });
});
