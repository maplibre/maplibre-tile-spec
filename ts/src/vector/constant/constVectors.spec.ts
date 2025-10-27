import { describe, it, expect } from "vitest";
import { IntConstVector } from "./intConstVector";
import { LongConstVector } from "./longConstVector";
import BitVector from "../flat/bitVector";
import { FlatSelectionVector } from "../filter/flatSelectionVector";

function getSelectionIndices(selection: { selectionValues: () => number[]; limit: number }): number[] {
    return selection.selectionValues().slice(0, selection.limit);
}

describe("IntConstVector", () => {
    it("should return constant value", () => {
        const vec = new IntConstVector("test", 42, 10);
        expect(vec.size).toBe(10);
        expect(vec.getValue(0)).toBe(42);
        expect(vec.getValue(5)).toBe(42);
        expect(vec.getValue(9)).toBe(42);
    });

    it("should filter matching constant", () => {
        const vec = new IntConstVector("test", 42, 10);
        const result = vec.filter(42);
        expect(result.limit).toBe(10);
        expect(getSelectionIndices(result)).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    });

    it("should return empty when filtering non-matching value", () => {
        const vec = new IntConstVector("test", 42, 10);
        const result = vec.filter(99);
        expect(result.limit).toBe(0);
    });

    it("should match when value in array", () => {
        const vec = new IntConstVector("test", 42, 5);
        expect(vec.match([10, 42, 99]).limit).toBe(5);
        expect(vec.match([10, 99]).limit).toBe(0);
    });

    it("should filter selected", () => {
        const vec = new IntConstVector("test", 42, 10);
        const selection = new FlatSelectionVector([2, 3, 4, 5]);
        vec.filterSelected(42, selection);
        expect(getSelectionIndices(selection)).toEqual([2, 3, 4, 5]);

        const selection2 = new FlatSelectionVector([2, 3, 4]);
        vec.filterSelected(99, selection2);
        expect(selection2.limit).toBe(0);
    });

    it("should match selected", () => {
        const vec = new IntConstVector("test", 42, 10);
        const selection = new FlatSelectionVector([2, 3, 4]);
        vec.matchSelected([10, 42], selection);
        expect(getSelectionIndices(selection)).toEqual([2, 3, 4]);
    });

    it("should handle comparison operations", () => {
        const vec = new IntConstVector("test", 50, 5);
        expect(vec.greaterThanOrEqualTo(50).limit).toBe(5);
        expect(vec.greaterThanOrEqualTo(51).limit).toBe(0);
        expect(vec.smallerThanOrEqualTo(50).limit).toBe(5);
        expect(vec.smallerThanOrEqualTo(49).limit).toBe(0);
    });

    it("should handle comparison selected", () => {
        const vec = new IntConstVector("test", 50, 10);
        const sel1 = new FlatSelectionVector([1, 2, 3]);
        vec.greaterThanOrEqualToSelected(50, sel1);
        expect(getSelectionIndices(sel1)).toEqual([1, 2, 3]);

        const sel2 = new FlatSelectionVector([1, 2, 3]);
        vec.greaterThanOrEqualToSelected(51, sel2);
        expect(sel2.limit).toBe(0);
    });

    it("should filter not equal", () => {
        const vec = new IntConstVector("test", 42, 5);
        expect(vec.filterNotEqual(99).limit).toBe(5);
        expect(vec.filterNotEqual(42).limit).toBe(0);
    });

    it("should filter not equal selected", () => {
        const vec = new IntConstVector("test", 42, 10);
        const selection = new FlatSelectionVector([2, 3, 4]);
        vec.filterNotEqualSelected(99, selection);
        expect(getSelectionIndices(selection)).toEqual([2, 3, 4]);

        const selection2 = new FlatSelectionVector([2, 3, 4]);
        vec.filterNotEqualSelected(42, selection2);
        expect(selection2.limit).toBe(0);
    });

    it("should handle nullability", () => {
        const bitVector = new BitVector(new Uint8Array([0b01010101]), 8); // 0,2,4,6 present
        const vec = new IntConstVector("test", 42, bitVector);
        const result = vec.filter(42);
        expect(result.limit).toBe(4);
        expect(getSelectionIndices(result)).toEqual([0, 2, 4, 6]);
    });
});

describe("LongConstVector", () => {
    it("should return constant bigint value", () => {
        const vec = new LongConstVector("test", 42n, 10);
        expect(vec.getValue(0)).toBe(42n);
        expect(vec.getValue(9)).toBe(42n);
    });

    it("should filter matching constant", () => {
        const vec = new LongConstVector("test", 42n, 5);
        expect(vec.filter(42n).limit).toBe(5);
        expect(vec.filter(99n).limit).toBe(0);
    });

    it("should match values", () => {
        const vec = new LongConstVector("test", 42n, 5);
        expect(vec.match([10n, 42n]).limit).toBe(5);
        expect(vec.match([10n, 99n]).limit).toBe(0);
    });

    it("should handle comparison operations", () => {
        const vec = new LongConstVector("test", 50n, 5);
        expect(vec.greaterThanOrEqualTo(50n).limit).toBe(5);
        expect(vec.greaterThanOrEqualTo(51n).limit).toBe(0);
        expect(vec.smallerThanOrEqualTo(50n).limit).toBe(5);
        expect(vec.smallerThanOrEqualTo(49n).limit).toBe(0);
    });

    it("should filter not equal", () => {
        const vec = new LongConstVector("test", 42n, 5);
        expect(vec.filterNotEqual(99n).limit).toBe(5);
        expect(vec.filterNotEqual(42n).limit).toBe(0);
    });
});
