import { describe, it, expect } from "vitest";
import { IntFlatVector } from "./intFlatVector";
import BitVector from "./bitVector";
import { FlatSelectionVector } from "../filter/flatSelectionVector";

function createVector(values: number[], name = "test"): IntFlatVector {
    const data = new Int32Array(values);
    return new IntFlatVector(name, data, values.length);
}

function createNullableVector(values: number[], nullBits: number, name = "test"): IntFlatVector {
    const data = new Int32Array(values);
    const nullability = new Uint8Array([nullBits]);
    const bitVector = new BitVector(nullability, values.length);
    return new IntFlatVector(name, data, bitVector);
}

function getSelectionIndices(selection: { selectionValues: () => number[]; limit: number }): number[] {
    return selection.selectionValues().slice(0, selection.limit);
}

const simpleVector = createVector([10, 20, 30, 40, 50]);
const withDuplicates = createVector([10, 20, 30, 20, 50, 10]);
const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);

describe("IntFlatVector", () => {
    describe("getValue and has", () => {
        it("should get values", () => {
            expect(simpleVector.size).toBe(5);
            expect(simpleVector.getValue(0)).toBe(10);
            expect(simpleVector.getValue(4)).toBe(50);
        });

        it("should handle nullability", () => {
            expect(withNulls.getValue(0)).toBe(10);
            expect(withNulls.getValue(3)).toBe(null);
            expect(withNulls.has(0)).toBe(true);
            expect(withNulls.has(3)).toBe(false);
        });
    });

    describe("filter", () => {
        it("should filter by value", () => {
            expect(getSelectionIndices(simpleVector.filter(30))).toEqual([2]);
            expect(getSelectionIndices(withDuplicates.filter(20))).toEqual([1, 3]);
            expect(getSelectionIndices(withDuplicates.filter(10))).toEqual([0, 5]);
        });

        it("should filter with nullability", () => {
            const result = withNulls.filter(30);
            expect(getSelectionIndices(result)).toEqual([2]);
        });

        it("should return empty when no match", () => {
            expect(simpleVector.filter(999).limit).toBe(0);
        });
    });

    describe("match", () => {
        it("should match multiple values", () => {
            expect(getSelectionIndices(simpleVector.match([10, 50]))).toEqual([0, 4]);
            expect(getSelectionIndices(withDuplicates.match([10, 50]))).toEqual([0, 4, 5]);
        });

        it("should match with nullability", () => {
            expect(getSelectionIndices(withNulls.match([10, 40]))).toEqual([0]);
        });
    });

    describe("filterSelected", () => {
        it("should filter from selection", () => {
            const selection = new FlatSelectionVector([0, 1, 3, 4]);
            withDuplicates.filterSelected(20, selection);
            expect(getSelectionIndices(selection)).toEqual([1, 3]);
        });
    });

    describe("matchSelected", () => {
        it("should match from selection", () => {
            const selection = new FlatSelectionVector([1, 2, 3, 4]);
            simpleVector.matchSelected([20, 40], selection);
            expect(getSelectionIndices(selection)).toEqual([1, 3]);
        });
    });

    describe("greaterThanOrEqualTo", () => {
        it("should filter >= threshold", () => {
            expect(getSelectionIndices(simpleVector.greaterThanOrEqualTo(30))).toEqual([2, 3, 4]);
            expect(getSelectionIndices(withNulls.greaterThanOrEqualTo(30))).toEqual([2, 4]);
        });
    });

    describe("greaterThanOrEqualToSelected", () => {
        it("should filter selected >= threshold", () => {
            const selection = new FlatSelectionVector([0, 2, 3, 4]);
            simpleVector.greaterThanOrEqualToSelected(30, selection);
            expect(getSelectionIndices(selection)).toEqual([2, 3, 4]);
        });
    });

    describe("smallerThanOrEqualTo", () => {
        it("should filter <= threshold", () => {
            expect(getSelectionIndices(simpleVector.smallerThanOrEqualTo(30))).toEqual([0, 1, 2]);
            expect(getSelectionIndices(withNulls.smallerThanOrEqualTo(30))).toEqual([0, 1, 2]);
        });
    });

    describe("smallerThanOrEqualToSelected", () => {
        it("should filter selected <= threshold", () => {
            const selection = new FlatSelectionVector([1, 2, 3, 4]);
            simpleVector.smallerThanOrEqualToSelected(30, selection);
            expect(getSelectionIndices(selection)).toEqual([1, 2]);
        });
    });

    describe("filterNotEqual", () => {
        it("should filter != value", () => {
            expect(getSelectionIndices(withDuplicates.filterNotEqual(20))).toEqual([0, 2, 4, 5]);
        });

        it("should include nulls in not equal", () => {
            // 0b00001011 = 0,1,3 present, 2,4 are null
            const vec = createNullableVector([10, 20, 30, 20, 50], 0b00001011);
            expect(getSelectionIndices(vec.filterNotEqual(20))).toEqual([0, 2, 4]);
        });
    });

    describe("filterNotEqualSelected", () => {
        it("should filter != from selection", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3]);
            withDuplicates.filterNotEqualSelected(20, selection);
            expect(getSelectionIndices(selection)).toEqual([0, 2]);
        });
    });

    describe("presentValues and nullableValues", () => {
        it("should get present values", () => {
            expect(getSelectionIndices(withNulls.presentValues())).toEqual([0, 1, 2, 4]);
        });

        it("should get nullable values", () => {
            expect(getSelectionIndices(withNulls.nullableValues())).toEqual([3]);
        });

        it("should return empty nullableValues when no nullability", () => {
            expect(simpleVector.nullableValues().limit).toBe(0);
        });
    });

    describe("not implemented methods", () => {
        it("should throw for noneMatch", () => {
            expect(() => simpleVector.noneMatch([10])).toThrow("Not implemented yet.");
        });

        it("should throw for noneMatchSelected", () => {
            const sel = new FlatSelectionVector([0]);
            expect(() => simpleVector.noneMatchSelected([10], sel)).toThrow("Not implemented yet.");
        });
    });
});
