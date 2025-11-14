import { describe, it, expect, beforeEach } from "vitest";
import { DoubleFlatVector } from "./doubleFlatVector";
import BitVector from "./bitVector";
import { FlatSelectionVector } from "../filter/flatSelectionVector";

function getSelectionIndices(selection: { selectionValues: () => number[]; limit: number }): number[] {
    return selection.selectionValues().slice(0, selection.limit);
}

function createVector(values: number[]): DoubleFlatVector {
    return new DoubleFlatVector("test", new Float64Array(values), values.length);
}

function createNullableVector(values: number[], nullBits: number): DoubleFlatVector {
    const data = new Float64Array(values);
    const bitVector = new BitVector(new Uint8Array([nullBits]), values.length);
    return new DoubleFlatVector("test", data, bitVector);
}

describe("DoubleFlatVector", () => {
    let vec: DoubleFlatVector;

    beforeEach(() => {
        vec = createVector([10.5, 20.5, 30.5, 40.5, 50.5]);
    });

    describe("filter", () => {
        it("should filter by exact value", () => {
            expect(getSelectionIndices(vec.filter(30.5))).toEqual([2]);
        });

        it("should return empty result for non-existent value", () => {
            expect(vec.filter(999.9).limit).toBe(0);
        });

        it("should return multiple indices for same value", () => {
            const multiVec = createVector([10.5, 20.5, 10.5, 30.5, 10.5]);
            expect(getSelectionIndices(multiVec.filter(10.5))).toEqual([0, 2, 4]);
        });

        it("should filter first element", () => {
            expect(getSelectionIndices(vec.filter(10.5))).toEqual([0]);
        });

        it("should filter last element", () => {
            expect(getSelectionIndices(vec.filter(50.5))).toEqual([4]);
        });

        it("should return FlatSelectionVector", () => {
            const result = vec.filter(10.5);
            expect(result).toBeInstanceOf(FlatSelectionVector);
        });

        it("should handle empty vector", () => {
            const emptyVec = createVector([]);
            expect(emptyVec.filter(10.5).limit).toBe(0);
        });

        it("should handle negative numbers", () => {
            const negVec = createVector([-10.5, -20.5, -30.5]);
            expect(getSelectionIndices(negVec.filter(-20.5))).toEqual([1]);
        });

        it("should handle decimal precision", () => {
            const precisionVec = createVector([10.123456789, 20.5, 10.123456789]);
            expect(getSelectionIndices(precisionVec.filter(10.123456789))).toEqual([0, 2]);
        });
    });

    describe("match", () => {
        it("should match multiple values", () => {
            expect(getSelectionIndices(vec.match([10.5, 50.5]))).toEqual([0, 4]);
        });

        it("should return empty for non-existent values", () => {
            expect(vec.match([999.9, 888.8]).limit).toBe(0);
        });

        it("should match single value in array", () => {
            expect(getSelectionIndices(vec.match([30.5]))).toEqual([2]);
        });

        it("should match all values", () => {
            expect(getSelectionIndices(vec.match([10.5, 20.5, 30.5, 40.5, 50.5]))).toEqual([0, 1, 2, 3, 4]);
        });

        it("should handle duplicate matches in test values - duplicates are included in result", () => {
            const multiVec = createVector([10.5, 20.5, 10.5, 30.5, 50.5]);
            const result = getSelectionIndices(multiVec.match([10.5, 10.5, 50.5]));
            expect(result).toEqual([0, 0, 2, 2, 4]);
        });

        it("should handle empty test array", () => {
            expect(vec.match([]).limit).toBe(0);
        });

        it("should match in order of appearance", () => {
            expect(getSelectionIndices(vec.match([50.5, 10.5, 30.5]))).toEqual([0, 2, 4]);
        });

        it("should return FlatSelectionVector", () => {
            const result = vec.match([10.5, 20.5]);
            expect(result).toBeInstanceOf(FlatSelectionVector);
        });
    });

    describe("filterSelected", () => {
        it("should filter selected from subset", () => {
            const selection = new FlatSelectionVector([0, 1, 2]);
            vec.filterSelected(20.5, selection);
            expect(getSelectionIndices(selection)).toEqual([1]);
        });

        it("should remove non-matching indices", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            vec.filterSelected(30.5, selection);
            expect(getSelectionIndices(selection)).toEqual([2]);
        });

        it("should handle when no values match", () => {
            const selection = new FlatSelectionVector([0, 1, 2]);
            vec.filterSelected(999.9, selection);
            expect(getSelectionIndices(selection)).toEqual([]);
            expect(selection.limit).toBe(0);
        });

        it("should preserve order", () => {
            const selection = new FlatSelectionVector([4, 3, 2, 1, 0]);
            vec.filterSelected(10.5, selection);
            expect(getSelectionIndices(selection)).toEqual([0]);
        });

        it("should handle single element selection", () => {
            const selection = new FlatSelectionVector([2]);
            vec.filterSelected(30.5, selection);
            expect(getSelectionIndices(selection)).toEqual([2]);
        });

        it("should update limit correctly", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            vec.filterSelected(20.5, selection);
            expect(selection.limit).toBe(1);
        });
    });

    describe("matchSelected", () => {
        it("should match selected from subset", () => {
            const selection = new FlatSelectionVector([1, 2, 3]);
            vec.matchSelected([20.5, 40.5], selection);
            expect(getSelectionIndices(selection)).toEqual([1, 3]);
        });

        it("should handle no matches", () => {
            const selection = new FlatSelectionVector([0, 1, 2]);
            vec.matchSelected([999.9, 888.8], selection);
            expect(getSelectionIndices(selection)).toEqual([]);
        });

        it("should match multiple values in selection", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            vec.matchSelected([10.5, 30.5, 50.5], selection);
            expect(getSelectionIndices(selection)).toEqual([0, 2, 4]);
        });

        it("should preserve selection order", () => {
            const selection = new FlatSelectionVector([4, 3, 2, 1, 0]);
            vec.matchSelected([10.5, 30.5], selection);
            expect(getSelectionIndices(selection)).toEqual([2, 0]);
        });

        it("should update limit correctly", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            vec.matchSelected([20.5, 40.5], selection);
            expect(selection.limit).toBe(2);
        });

        it("should handle empty test values", () => {
            const selection = new FlatSelectionVector([0, 1, 2]);
            vec.matchSelected([], selection);
            expect(getSelectionIndices(selection)).toEqual([]);
        });
    });

    describe("greaterThanOrEqualTo", () => {
        it("should filter >= threshold", () => {
            expect(getSelectionIndices(vec.greaterThanOrEqualTo(30.5))).toEqual([2, 3, 4]);
        });

        it("should include equal values", () => {
            expect(getSelectionIndices(vec.greaterThanOrEqualTo(30.5))).toContain(2);
        });

        it("should return all for minimum value", () => {
            expect(getSelectionIndices(vec.greaterThanOrEqualTo(10.5))).toEqual([0, 1, 2, 3, 4]);
        });

        it("should return empty for value greater than all", () => {
            expect(vec.greaterThanOrEqualTo(999.9).limit).toBe(0);
        });

        it("should handle negative threshold", () => {
            const negVec = createVector([-10.5, 0, 10.5, 20.5]);
            expect(getSelectionIndices(negVec.greaterThanOrEqualTo(-5.5))).toEqual([1, 2, 3]);
        });

        it("should return FlatSelectionVector", () => {
            const result = vec.greaterThanOrEqualTo(30.5);
            expect(result).toBeInstanceOf(FlatSelectionVector);
        });
    });

    describe("smallerThanOrEqualTo", () => {
        it("should filter <= threshold", () => {
            expect(getSelectionIndices(vec.smallerThanOrEqualTo(30.5))).toEqual([0, 1, 2]);
        });

        it("should include equal values", () => {
            expect(getSelectionIndices(vec.smallerThanOrEqualTo(30.5))).toContain(2);
        });

        it("should return all for maximum value", () => {
            expect(getSelectionIndices(vec.smallerThanOrEqualTo(50.5))).toEqual([0, 1, 2, 3, 4]);
        });

        it("should return empty for value less than all", () => {
            expect(vec.smallerThanOrEqualTo(1.0).limit).toBe(0);
        });

        it("should handle negative threshold", () => {
            const negVec = createVector([-30.5, -10.5, 0, 10.5]);
            expect(getSelectionIndices(negVec.smallerThanOrEqualTo(-10.5))).toEqual([0, 1]);
        });

        it("should return FlatSelectionVector", () => {
            const result = vec.smallerThanOrEqualTo(30.5);
            expect(result).toBeInstanceOf(FlatSelectionVector);
        });
    });

    describe("greaterThanOrEqualToSelected", () => {
        it("should filter selected >= threshold", () => {
            const selection = new FlatSelectionVector([1, 2, 3, 4]);
            vec.greaterThanOrEqualToSelected(30.5, selection);
            expect(getSelectionIndices(selection)).toEqual([2, 3, 4]);
        });

        it("should update limit correctly", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            vec.greaterThanOrEqualToSelected(30.5, selection);
            expect(selection.limit).toBe(3);
        });

        it("should handle no matches", () => {
            const selection = new FlatSelectionVector([0, 1, 2]);
            vec.greaterThanOrEqualToSelected(999.9, selection);
            expect(getSelectionIndices(selection)).toEqual([]);
        });

        it("should preserve order of indices", () => {
            const selection = new FlatSelectionVector([4, 3, 2, 1, 0]);
            vec.greaterThanOrEqualToSelected(30.5, selection);
            expect(getSelectionIndices(selection)).toEqual([4, 3, 2]);
        });
    });

    describe("smallerThanOrEqualToSelected", () => {
        it("should filter selected <= threshold", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3]);
            vec.smallerThanOrEqualToSelected(20.5, selection);
            expect(getSelectionIndices(selection)).toEqual([0, 1]);
        });

        it("should update limit correctly", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            vec.smallerThanOrEqualToSelected(20.5, selection);
            expect(selection.limit).toBe(2);
        });

        it("should handle no matches", () => {
            const selection = new FlatSelectionVector([3, 4]);
            vec.smallerThanOrEqualToSelected(1.0, selection);
            expect(getSelectionIndices(selection)).toEqual([]);
        });

        it("should preserve order of indices", () => {
            const selection = new FlatSelectionVector([4, 3, 2, 1, 0]);
            vec.smallerThanOrEqualToSelected(20.5, selection);
            expect(getSelectionIndices(selection)).toEqual([1, 0]);
        });
    });

    describe("nullability handling", () => {
        it("should exclude null values in filter", () => {
            const nullVec = createNullableVector([10.5, 20.5, 30.5, 40.5], 0b0111);
            expect(getSelectionIndices(nullVec.filter(40.5))).toEqual([]);
        });

        it("should include non-null matching values", () => {
            const nullVec = createNullableVector([10.5, 20.5, 30.5, 40.5], 0b0111);
            expect(getSelectionIndices(nullVec.filter(20.5))).toEqual([1]);
        });

        it("should exclude null values in match", () => {
            const nullVec = createNullableVector([10.5, 20.5, 30.5, 40.5], 0b0111);
            expect(getSelectionIndices(nullVec.match([30.5, 40.5]))).toEqual([2]);
        });

        it("should exclude null values in comparison filters", () => {
            const nullVec = createNullableVector([10.5, 20.5, 30.5, 40.5], 0b0111);
            expect(getSelectionIndices(nullVec.greaterThanOrEqualTo(20.5))).toEqual([1, 2]);
        });

        it("should handle all null vector", () => {
            const allNullVec = createNullableVector([10.5, 20.5, 30.5], 0b0000);
            expect(allNullVec.filter(10.5).limit).toBe(0);
        });

        it("should handle all non-null vector", () => {
            const allValidVec = createNullableVector([10.5, 20.5, 30.5], 0b1111);
            expect(getSelectionIndices(allValidVec.filter(20.5))).toEqual([1]);
        });

        it("should exclude null in filterSelected", () => {
            const nullVec = createNullableVector([10.5, 20.5, 30.5, 40.5], 0b0111);
            const selection = new FlatSelectionVector([0, 1, 2, 3]);
            nullVec.filterSelected(20.5, selection);
            expect(getSelectionIndices(selection)).toEqual([1]);
        });

        it("should exclude null in greaterThanOrEqualToSelected", () => {
            const nullVec = createNullableVector([10.5, 20.5, 30.5, 40.5], 0b0111);
            const selection = new FlatSelectionVector([1, 2, 3]);
            nullVec.greaterThanOrEqualToSelected(20.5, selection);
            expect(getSelectionIndices(selection)).toEqual([1, 2]);
        });
    });

    describe("unimplemented methods", () => {
        it("should throw for noneMatch", () => {
            expect(() => vec.noneMatch([10.5])).toThrow("Not implemented yet.");
        });

        it("should throw for noneMatchSelected", () => {
            const selection = new FlatSelectionVector([0, 1]);
            expect(() => vec.noneMatchSelected([10.5], selection)).toThrow("Not implemented yet.");
        });

        it("should throw for filterNotEqual", () => {
            expect(() => vec.filterNotEqual(10.5)).toThrow("Not implemented yet.");
        });

        it("should throw for filterNotEqualSelected", () => {
            const selection = new FlatSelectionVector([0]);
            expect(() => vec.filterNotEqualSelected(10.5, selection)).toThrow("Not implemented yet.");
        });
    });

    describe("edge cases and special values", () => {
        it("should handle very large numbers", () => {
            const largeVec = createVector([1e10, 2e10, 3e10]);
            expect(getSelectionIndices(largeVec.filter(2e10))).toEqual([1]);
        });

        it("should handle very small numbers", () => {
            const smallVec = createVector([1e-10, 2e-10, 3e-10]);
            expect(getSelectionIndices(smallVec.filter(2e-10))).toEqual([1]);
        });

        it("should handle mixed positive and negative", () => {
            const mixedVec = createVector([-30.5, -10.5, 0, 10.5, 30.5]);
            expect(getSelectionIndices(mixedVec.filter(0))).toEqual([2]);
            expect(getSelectionIndices(mixedVec.greaterThanOrEqualTo(0))).toEqual([2, 3, 4]);
            expect(getSelectionIndices(mixedVec.smallerThanOrEqualTo(0))).toEqual([0, 1, 2]);
        });

        it("should handle single element vector", () => {
            const singleVec = createVector([42.5]);
            expect(getSelectionIndices(singleVec.filter(42.5))).toEqual([0]);
            expect(getSelectionIndices(singleVec.greaterThanOrEqualTo(40.0))).toEqual([0]);
        });

        it("should handle duplicate values in vector", () => {
            const dupVec = createVector([10.5, 10.5, 10.5, 20.5, 20.5]);
            expect(getSelectionIndices(dupVec.filter(10.5))).toEqual([0, 1, 2]);
            expect(getSelectionIndices(dupVec.match([10.5, 20.5]))).toEqual([0, 1, 2, 3, 4]);
        });

        it("should handle Infinity values", () => {
            const infVec = createVector([10.5, Infinity, 30.5, -Infinity]);
            expect(getSelectionIndices(infVec.greaterThanOrEqualTo(Infinity))).toEqual([1]);
            expect(getSelectionIndices(infVec.smallerThanOrEqualTo(-Infinity))).toEqual([3]);
        });

        it("should handle NaN values", () => {
            const nanVec = createVector([10.5, NaN, 30.5]);
            expect(nanVec.filter(NaN).limit).toBe(0);
        });
    });

    describe("combined operations", () => {
        it("should apply filter then comparison", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            vec.filterSelected(10.5, selection);
            expect(getSelectionIndices(selection)).toEqual([0]);
        });

        it("should apply multiple match operations sequentially", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            vec.matchSelected([10.5, 20.5, 30.5], selection);
            expect(getSelectionIndices(selection)).toEqual([0, 1, 2]);
        });

        it("should handle chained comparisons", () => {
            const ge30 = getSelectionIndices(vec.greaterThanOrEqualTo(30.5));
            const le40 = getSelectionIndices(vec.smallerThanOrEqualTo(40.5));
            const intersection = ge30.filter(idx => le40.includes(idx));
            expect(intersection).toEqual([2, 3]);
        });
    });
});
