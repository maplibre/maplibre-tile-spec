import { describe, it, expect, beforeEach } from "vitest";
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

describe("IntFlatVector", () => {
    let simpleVector: IntFlatVector;
    let withDuplicates: IntFlatVector;
    let withNulls: IntFlatVector;
    let emptyVector: IntFlatVector;
    let negativeVector: IntFlatVector;

    beforeEach(() => {
        simpleVector = createVector([10, 20, 30, 40, 50]);
        withDuplicates = createVector([10, 20, 30, 20, 50, 10]);
        withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111); // indices 0,1,2,4 present; 3 null
        emptyVector = createVector([]);
        negativeVector = createVector([-50, -30, -10, 0, 10, 30, 50]);
    });

    describe("getValueFromBuffer and getValue", () => {
        it("should get values correctly", () => {
            expect(simpleVector.getValue(0)).toBe(10);
            expect(simpleVector.getValue(4)).toBe(50);
        });

        it("should return null for null values", () => {
            expect(withNulls.getValue(3)).toBe(null);
        });

        it("should return actual values for non-null indices", () => {
            expect(withNulls.getValue(0)).toBe(10);
            expect(withNulls.getValue(2)).toBe(30);
        });
    });

    describe("size and properties", () => {
        it("should have correct size", () => {
            expect(simpleVector.size).toBe(5);
            expect(withDuplicates.size).toBe(6);
            expect(emptyVector.size).toBe(0);
        });

        it("should have name", () => {
            expect(simpleVector.name).toBe("test");
            const named = createVector([1, 2], "custom");
            expect(named.name).toBe("custom");
        });
    });

    describe("has - nullability check", () => {
        it("should return true for non-null values", () => {
            expect(withNulls.has(0)).toBe(true);
            expect(withNulls.has(1)).toBe(true);
            expect(withNulls.has(2)).toBe(true);
            expect(withNulls.has(4)).toBe(true);
        });

        it("should return false for null values", () => {
            expect(withNulls.has(3)).toBe(false);
        });

        it("should return true for all in vector without nullability", () => {
            expect(simpleVector.has(0)).toBe(true);
            expect(simpleVector.has(4)).toBe(true);
        });

        it("should return false for out of bounds", () => {
            expect(simpleVector.has(100)).toBe(false);
        });
    });

    describe("filter", () => {
        it("should filter by exact value", () => {
            expect(getSelectionIndices(simpleVector.filter(30))).toEqual([2]);
        });

        it("should return multiple indices for duplicate values", () => {
            expect(getSelectionIndices(withDuplicates.filter(20))).toEqual([1, 3]);
            expect(getSelectionIndices(withDuplicates.filter(10))).toEqual([0, 5]);
        });

        it("should return empty when no match", () => {
            expect(simpleVector.filter(999).limit).toBe(0);
        });

        it("should filter first element", () => {
            expect(getSelectionIndices(simpleVector.filter(10))).toEqual([0]);
        });

        it("should filter last element", () => {
            expect(getSelectionIndices(simpleVector.filter(50))).toEqual([4]);
        });

        it("should respect nullability", () => {
            const result = withNulls.filter(30);
            expect(getSelectionIndices(result)).toEqual([2]);
            expect(getSelectionIndices(withNulls.filter(40))).toEqual([]);
        });

        it("should handle empty vector", () => {
            expect(emptyVector.filter(10).limit).toBe(0);
        });

        it("should handle negative integers", () => {
            expect(getSelectionIndices(negativeVector.filter(-30))).toEqual([1]);
            expect(getSelectionIndices(negativeVector.filter(0))).toEqual([3]);
        });

        it("should return FlatSelectionVector", () => {
            const result = simpleVector.filter(10);
            expect(result).toBeInstanceOf(FlatSelectionVector);
        });
    });

    describe("match", () => {
        it("should match multiple values", () => {
            expect(getSelectionIndices(simpleVector.match([10, 50]))).toEqual([0, 4]);
        });

        it("should match all values in vector", () => {
            expect(getSelectionIndices(simpleVector.match([10, 20, 30, 40, 50]))).toEqual([0, 1, 2, 3, 4]);
        });

        it("should handle duplicates in vector", () => {
            expect(getSelectionIndices(withDuplicates.match([10, 50]))).toEqual([0, 4, 5]);
            expect(getSelectionIndices(withDuplicates.match([20]))).toEqual([1, 3]);
        });

        it("should return empty for non-existent values", () => {
            expect(simpleVector.match([999, 888]).limit).toBe(0);
        });

        it("should handle empty test array", () => {
            expect(simpleVector.match([]).limit).toBe(0);
        });

        it("should respect nullability", () => {
            expect(getSelectionIndices(withNulls.match([10, 40]))).toEqual([0]);
        });

        it("should handle single value in array", () => {
            expect(getSelectionIndices(simpleVector.match([30]))).toEqual([2]);
        });

        it("should handle duplicate matches in test values", () => {
            const multiVec = createVector([10, 20, 10, 30, 50]);
            const result = getSelectionIndices(multiVec.match([10, 10, 50]));
            expect(result).toEqual([0, 0, 2, 2, 4]);
        });

        it("should return FlatSelectionVector", () => {
            const result = simpleVector.match([10, 20]);
            expect(result).toBeInstanceOf(FlatSelectionVector);
        });

        it("should match in order of appearance", () => {
            expect(getSelectionIndices(simpleVector.match([50, 10, 30]))).toEqual([0, 2, 4]);
        });
    });

    describe("filterSelected", () => {
        it("should filter from selection", () => {
            const selection = new FlatSelectionVector([0, 1, 3, 4]);
            withDuplicates.filterSelected(20, selection);
            expect(getSelectionIndices(selection)).toEqual([1, 3]);
        });

        it("should handle no matches", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3]);
            simpleVector.filterSelected(999, selection);
            expect(getSelectionIndices(selection)).toEqual([]);
            expect(selection.limit).toBe(0);
        });

        it("should update limit correctly", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            simpleVector.filterSelected(30, selection);
            expect(selection.limit).toBe(1);
        });

        it("should handle single element selection", () => {
            const selection = new FlatSelectionVector([2]);
            simpleVector.filterSelected(30, selection);
            expect(getSelectionIndices(selection)).toEqual([2]);
        });

        it("should handle empty selection", () => {
            const selection = new FlatSelectionVector([]);
            simpleVector.filterSelected(10, selection);
            expect(selection.limit).toBe(0);
        });

        it("should respect nullability", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            withNulls.filterSelected(40, selection);
            expect(getSelectionIndices(selection)).toEqual([]);
        });
    });

    describe("matchSelected", () => {
        it("should match from selection", () => {
            const selection = new FlatSelectionVector([1, 2, 3, 4]);
            simpleVector.matchSelected([20, 40], selection);
            expect(getSelectionIndices(selection)).toEqual([1, 3]);
        });

        it("should handle no matches", () => {
            const selection = new FlatSelectionVector([0, 1, 2]);
            simpleVector.matchSelected([999, 888], selection);
            expect(getSelectionIndices(selection)).toEqual([]);
        });

        it("should update limit correctly", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            simpleVector.matchSelected([20, 40], selection);
            expect(selection.limit).toBe(2);
        });

        it("should preserve selection input order", () => {
            const selection = new FlatSelectionVector([4, 3, 2, 1, 0]);
            simpleVector.matchSelected([10, 30], selection);
            expect(getSelectionIndices(selection)).toEqual([2, 0]);
        });

        it("should handle empty test values", () => {
            const selection = new FlatSelectionVector([0, 1, 2]);
            simpleVector.matchSelected([], selection);
            expect(getSelectionIndices(selection)).toEqual([]);
        });

        it("should respect nullability in selected", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            withNulls.matchSelected([30, 40], selection);
            expect(getSelectionIndices(selection)).toEqual([2]);
        });
    });

    describe("greaterThanOrEqualTo", () => {
        it("should filter >= threshold", () => {
            expect(getSelectionIndices(simpleVector.greaterThanOrEqualTo(30))).toEqual([2, 3, 4]);
        });

        it("should include equal values", () => {
            expect(getSelectionIndices(simpleVector.greaterThanOrEqualTo(30))).toContain(2);
        });

        it("should return all for minimum value", () => {
            expect(getSelectionIndices(simpleVector.greaterThanOrEqualTo(10))).toEqual([0, 1, 2, 3, 4]);
        });

        it("should return empty for value greater than all", () => {
            expect(simpleVector.greaterThanOrEqualTo(999).limit).toBe(0);
        });

        it("should handle negative threshold", () => {
            expect(getSelectionIndices(negativeVector.greaterThanOrEqualTo(-10))).toEqual([2, 3, 4, 5, 6]);
        });

        it("should respect nullability", () => {
            expect(getSelectionIndices(withNulls.greaterThanOrEqualTo(30))).toEqual([2, 4]);
        });

        it("should return FlatSelectionVector", () => {
            const result = simpleVector.greaterThanOrEqualTo(30);
            expect(result).toBeInstanceOf(FlatSelectionVector);
        });
    });

    describe("greaterThanOrEqualToSelected", () => {
        it("should filter selected >= threshold", () => {
            const selection = new FlatSelectionVector([0, 2, 3, 4]);
            simpleVector.greaterThanOrEqualToSelected(30, selection);
            expect(getSelectionIndices(selection)).toEqual([2, 3, 4]);
        });

        it("should update limit correctly", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            simpleVector.greaterThanOrEqualToSelected(30, selection);
            expect(selection.limit).toBe(3);
        });

        it("should handle no matches", () => {
            const selection = new FlatSelectionVector([0, 1, 2]);
            simpleVector.greaterThanOrEqualToSelected(999, selection);
            expect(getSelectionIndices(selection)).toEqual([]);
        });

        it("should preserve selection input order", () => {
            const selection = new FlatSelectionVector([4, 3, 2, 1, 0]);
            simpleVector.greaterThanOrEqualToSelected(30, selection);
            expect(getSelectionIndices(selection)).toEqual([4, 3, 2]);
        });
    });

    describe("smallerThanOrEqualTo", () => {
        it("should filter <= threshold", () => {
            expect(getSelectionIndices(simpleVector.smallerThanOrEqualTo(30))).toEqual([0, 1, 2]);
        });

        it("should include equal values", () => {
            expect(getSelectionIndices(simpleVector.smallerThanOrEqualTo(30))).toContain(2);
        });

        it("should return all for maximum value", () => {
            expect(getSelectionIndices(simpleVector.smallerThanOrEqualTo(50))).toEqual([0, 1, 2, 3, 4]);
        });

        it("should return empty for value less than all", () => {
            expect(simpleVector.smallerThanOrEqualTo(1).limit).toBe(0);
        });

        it("should handle negative threshold", () => {
            expect(getSelectionIndices(negativeVector.smallerThanOrEqualTo(-10))).toEqual([0, 1, 2]);
        });

        it("should respect nullability", () => {
            expect(getSelectionIndices(withNulls.smallerThanOrEqualTo(30))).toEqual([0, 1, 2]);
        });

        it("should return FlatSelectionVector", () => {
            const result = simpleVector.smallerThanOrEqualTo(30);
            expect(result).toBeInstanceOf(FlatSelectionVector);
        });
    });

    describe("smallerThanOrEqualToSelected", () => {
        it("should filter selected <= threshold", () => {
            const selection = new FlatSelectionVector([1, 2, 3, 4]);
            simpleVector.smallerThanOrEqualToSelected(30, selection);
            expect(getSelectionIndices(selection)).toEqual([1, 2]);
        });

        it("should update limit correctly", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            simpleVector.smallerThanOrEqualToSelected(30, selection);
            expect(selection.limit).toBe(3);
        });

        it("should handle no matches", () => {
            const selection = new FlatSelectionVector([3, 4]);
            simpleVector.smallerThanOrEqualToSelected(1, selection);
            expect(getSelectionIndices(selection)).toEqual([]);
        });

        it("should preserve selection input order", () => {
            const selection = new FlatSelectionVector([4, 3, 2, 1, 0]);
            simpleVector.smallerThanOrEqualToSelected(30, selection);
            expect(getSelectionIndices(selection)).toEqual([2, 1, 0]);
        });
    });

    describe("filterNotEqual", () => {
        it("should filter != value", () => {
            expect(getSelectionIndices(withDuplicates.filterNotEqual(20))).toEqual([0, 2, 4, 5]);
        });

        it("should include nulls in not equal", () => {
            const vec = createNullableVector([10, 20, 30, 20, 50], 0b00001011);
            expect(getSelectionIndices(vec.filterNotEqual(20))).toEqual([0, 2, 4]);
        });

        it("should return all when filtering for non-existent value", () => {
            expect(getSelectionIndices(simpleVector.filterNotEqual(999)).length).toBe(5);
        });

        it("should return empty when all values match", () => {
            const sameVec = createVector([10, 10, 10]);
            expect(getSelectionIndices(sameVec.filterNotEqual(10))).toEqual([]);
        });

        it("should handle negative values", () => {
            expect(getSelectionIndices(negativeVector.filterNotEqual(0))).toHaveLength(6);
        });

        it("should return FlatSelectionVector", () => {
            const result = simpleVector.filterNotEqual(10);
            expect(result).toBeInstanceOf(FlatSelectionVector);
        });

        it("should respect nullability correctly", () => {
            const result = withNulls.filterNotEqual(10);
            expect(getSelectionIndices(result)).toEqual([1, 2, 3, 4]);
        });
    });

    describe("filterNotEqualSelected", () => {
        it("should filter != from selection", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3]);
            withDuplicates.filterNotEqualSelected(20, selection);
            expect(getSelectionIndices(selection)).toEqual([0, 2]);
        });

        it("should update limit correctly", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            simpleVector.filterNotEqualSelected(30, selection);
            expect(selection.limit).toBe(4);
        });

        it("should handle all match case", () => {
            const selection = new FlatSelectionVector([0, 1, 2]);
            const sameVec = createVector([10, 10, 10]);
            sameVec.filterNotEqualSelected(10, selection);
            expect(selection.limit).toBe(0);
        });

        it("should include nulls in result", () => {
            const vec = createNullableVector([10, 20, 30, 40], 0b00001011);
            const selection = new FlatSelectionVector([0, 1, 2, 3]);
            vec.filterNotEqualSelected(20, selection);
            expect(getSelectionIndices(selection)).toEqual([0, 2, 3]);
        });

        it("should preserve selection order", () => {
            const selection = new FlatSelectionVector([4, 3, 2, 1, 0]);
            simpleVector.filterNotEqualSelected(30, selection);
            expect(getSelectionIndices(selection)).toEqual([4, 3, 1, 0]);
        });
    });

    describe("presentValues", () => {
        it("should return indices of non-null values", () => {
            expect(getSelectionIndices(withNulls.presentValues())).toEqual([0, 1, 2, 4]);
        });

        it("should return all indices when no nullability", () => {
            expect(getSelectionIndices(simpleVector.presentValues())).toEqual([0, 1, 2, 3, 4]);
        });

        it("should return empty when all null", () => {
            const allNull = createNullableVector([10, 20, 30], 0b00000000);
            expect(allNull.presentValues().limit).toBe(0);
        });

        it("should return FlatSelectionVector", () => {
            const result = withNulls.presentValues();
            expect(result).toBeInstanceOf(FlatSelectionVector);
        });
    });

    describe("nullableValues", () => {
        it("should return indices of null values", () => {
            expect(getSelectionIndices(withNulls.nullableValues())).toEqual([3]);
        });

        it("should return empty when no nullability", () => {
            expect(simpleVector.nullableValues().limit).toBe(0);
        });

        it("should return all indices when all null", () => {
            const allNull = createNullableVector([10, 20, 30], 0b00000000);
            expect(getSelectionIndices(allNull.nullableValues())).toEqual([0, 1, 2]);
        });

        it("should return FlatSelectionVector", () => {
            const result = withNulls.nullableValues();
            expect(result).toBeInstanceOf(FlatSelectionVector);
        });
    });

    describe("unimplemented methods", () => {
        it("should throw for noneMatch", () => {
            expect(() => simpleVector.noneMatch([10])).toThrow("Not implemented yet.");
        });

        it("should throw for noneMatchSelected", () => {
            const sel = new FlatSelectionVector([0]);
            expect(() => simpleVector.noneMatchSelected([10], sel)).toThrow("Not implemented yet.");
        });
    });

    describe("edge cases and special values", () => {
        it("should handle Int32 limits", () => {
            const maxInt = 2147483647;
            const minInt = -2147483648;
            const limitVec = createVector([minInt, -1, 0, 1, maxInt]);
            expect(getSelectionIndices(limitVec.filter(maxInt))).toEqual([4]);
            expect(getSelectionIndices(limitVec.filter(minInt))).toEqual([0]);
        });

        it("should handle zero", () => {
            const zeroVec = createVector([0, 1, 0, 2, 0]);
            expect(getSelectionIndices(zeroVec.filter(0))).toEqual([0, 2, 4]);
        });

        it("should handle single element", () => {
            const single = createVector([42]);
            expect(getSelectionIndices(single.filter(42))).toEqual([0]);
        });

        it("should handle large vectors", () => {
            const large = createVector(Array.from({ length: 1000 }, (_, i) => i));
            expect(getSelectionIndices(large.filter(500))).toEqual([500]);
            expect(getSelectionIndices(large.greaterThanOrEqualTo(999)).length).toBe(1);
        });

        it("should handle all same values", () => {
            const same = createVector([10, 10, 10, 10, 10]);
            expect(getSelectionIndices(same.filter(10))).toEqual([0, 1, 2, 3, 4]);
            expect(getSelectionIndices(same.filterNotEqual(10))).toEqual([]);
        });

        it("should handle mixed comparisons correctly", () => {
            const ge30 = getSelectionIndices(simpleVector.greaterThanOrEqualTo(30));
            const le40 = getSelectionIndices(simpleVector.smallerThanOrEqualTo(40));
            const intersection = ge30.filter(idx => le40.includes(idx));
            expect(intersection).toEqual([2, 3]);
        });
    });

    describe("combined operations", () => {
        it("should apply filter then comparison", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            simpleVector.filterSelected(10, selection);
            expect(getSelectionIndices(selection)).toEqual([0]);
        });

        it("should apply multiple match operations sequentially", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            simpleVector.matchSelected([10, 20, 30], selection);
            expect(getSelectionIndices(selection)).toEqual([0, 1, 2]);
        });

        it("should chain presentValues with filter operations", () => {
            const present = getSelectionIndices(withNulls.presentValues());
            expect(present).toEqual([0, 1, 2, 4]);
            const ge30 = getSelectionIndices(withNulls.greaterThanOrEqualTo(30));
            expect(ge30).toEqual([2, 4]);
        });

        it("should use filterNotEqual to exclude specific values", () => {
            const notTwenty = getSelectionIndices(withDuplicates.filterNotEqual(20));
            expect(notTwenty).toEqual([0, 2, 4, 5]);
            const notTwentySel = new FlatSelectionVector(notTwenty);
            withDuplicates.matchSelected([10, 50], notTwentySel);
            expect(getSelectionIndices(notTwentySel)).toEqual([0, 4, 5]);
        });
    });
});
