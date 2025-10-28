import { describe, it, expect } from "vitest";
import { LongFlatVector } from "./longFlatVector";
import BitVector from "./bitVector";
import { FlatSelectionVector } from "../filter/flatSelectionVector";

// Helper functions
function createVector(values: bigint[], name = "test"): LongFlatVector {
    const data = new BigInt64Array(values);
    return new LongFlatVector(name, data, values.length);
}

function createNullableVector(values: bigint[], nullBits: number, name = "test"): LongFlatVector {
    const data = new BigInt64Array(values);
    const nullability = new Uint8Array([nullBits]);
    const bitVector = new BitVector(nullability, values.length);
    return new LongFlatVector(name, data, bitVector);
}

function getSelectionIndices(selection: { selectionValues: () => number[]; limit: number }): number[] {
    return selection.selectionValues().slice(0, selection.limit);
}

// Predefined test vectors
const simpleVector = createVector([10n, 20n, 30n, 40n, 50n]);
const withDuplicates = createVector([10n, 20n, 30n, 20n, 50n, 10n]);
const withNulls = createNullableVector([10n, 20n, 30n, 40n, 50n], 0b00010111);

describe("LongFlatVector", () => {
    describe("getValue and has", () => {
        it("should get values", () => {
            expect(simpleVector.size).toBe(5);
            expect(simpleVector.getValue(0)).toBe(10n);
            expect(simpleVector.getValue(4)).toBe(50n);
        });

        it("should handle nullability", () => {
            expect(withNulls.getValue(0)).toBe(10n);
            expect(withNulls.getValue(3)).toBe(null);
            expect(withNulls.has(3)).toBe(false);
        });
    });

    describe("filter", () => {
        it("should filter by value", () => {
            expect(getSelectionIndices(simpleVector.filter(30n))).toEqual([2]);
            expect(getSelectionIndices(withDuplicates.filter(20n))).toEqual([1, 3]);
        });

        it("should filter with nullability", () => {
            expect(getSelectionIndices(withNulls.filter(30n))).toEqual([2]);
        });

        it("should return empty when no match", () => {
            expect(simpleVector.filter(999n).limit).toBe(0);
        });
    });

    describe("match", () => {
        it("should match multiple values", () => {
            expect(getSelectionIndices(simpleVector.match([10n, 50n]))).toEqual([0, 4]);
            expect(getSelectionIndices(withDuplicates.match([10n, 50n]))).toEqual([0, 4, 5]);
        });
    });

    describe("filterSelected and matchSelected", () => {
        it("should filter from selection", () => {
            const selection = new FlatSelectionVector([0, 1, 3, 4]);
            withDuplicates.filterSelected(20n, selection);
            expect(getSelectionIndices(selection)).toEqual([1, 3]);
        });

        it("should match from selection", () => {
            const selection = new FlatSelectionVector([1, 2, 3, 4]);
            simpleVector.matchSelected([20n, 40n], selection);
            expect(getSelectionIndices(selection)).toEqual([1, 3]);
        });
    });

    describe("comparison operations", () => {
        it("should filter >= threshold", () => {
            expect(getSelectionIndices(simpleVector.greaterThanOrEqualTo(30n))).toEqual([2, 3, 4]);
            expect(getSelectionIndices(withNulls.greaterThanOrEqualTo(30n))).toEqual([2, 4]);
        });

        it("should filter <= threshold", () => {
            expect(getSelectionIndices(simpleVector.smallerThanOrEqualTo(30n))).toEqual([0, 1, 2]);
        });

        it("should filter selected >= threshold", () => {
            const selection = new FlatSelectionVector([0, 2, 3, 4]);
            simpleVector.greaterThanOrEqualToSelected(30n, selection);
            expect(getSelectionIndices(selection)).toEqual([2, 3, 4]);
        });

        it("should filter selected <= threshold", () => {
            const selection = new FlatSelectionVector([1, 2, 3, 4]);
            simpleVector.smallerThanOrEqualToSelected(30n, selection);
            expect(getSelectionIndices(selection)).toEqual([1, 2]);
        });
    });

    describe("filterNotEqual", () => {
        it("should filter != value", () => {
            expect(getSelectionIndices(withDuplicates.filterNotEqual(20n))).toEqual([0, 2, 4, 5]);
        });

        it("should include nulls in not equal", () => {
            const vec = createNullableVector([10n, 20n, 30n, 20n, 50n], 0b00001011);
            expect(getSelectionIndices(vec.filterNotEqual(20n))).toEqual([0, 2, 4]);
        });

        it("should filter != from selection", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3]);
            withDuplicates.filterNotEqualSelected(20n, selection);
            expect(getSelectionIndices(selection)).toEqual([0, 2]);
        });
    });
});
