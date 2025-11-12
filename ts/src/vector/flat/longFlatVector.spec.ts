import { describe, it, expect, beforeEach } from "vitest";
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

describe("LongFlatVector", () => {
    let simpleVector: LongFlatVector;
    let withDuplicates: LongFlatVector;
    let withNulls: LongFlatVector;
    let emptyVector: LongFlatVector;
    let negativeVector: LongFlatVector;
    let largeValues: LongFlatVector;

    beforeEach(() => {
        simpleVector = createVector([10n, 20n, 30n, 40n, 50n]);
        withDuplicates = createVector([10n, 20n, 30n, 20n, 50n, 10n]);
        withNulls = createNullableVector([10n, 20n, 30n, 40n, 50n], 0b00010111);
        emptyVector = createVector([]);
        negativeVector = createVector([-50n, -30n, -10n, 0n, 10n, 30n, 50n]);
        largeValues = createVector([
            -9223372036854775808n, // MIN_VALUE
            -1000000000000000n,
            0n,
            1000000000000000n,
            9223372036854775807n, // MAX_VALUE
        ]);
    });

    describe("getValueFromBuffer and getValue", () => {
        it("should get values correctly", () => {
            expect(simpleVector.getValue(0)).toBe(10n);
            expect(simpleVector.getValue(4)).toBe(50n);
        });

        it("should return null for null values", () => {
            expect(withNulls.getValue(3)).toBe(null);
        });

        it("should return actual values for non-null indices", () => {
            expect(withNulls.getValue(0)).toBe(10n);
            expect(withNulls.getValue(2)).toBe(30n);
        });

        it("should handle large BigInt values", () => {
            expect(largeValues.getValue(0)).toBe(-9223372036854775808n);
            expect(largeValues.getValue(4)).toBe(9223372036854775807n);
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
            const named = createVector([1n, 2n], "custom");
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
            expect(simpleVector.has(-1)).toBe(false);
        });
    });

    describe("filter", () => {
        it("should filter by exact value", () => {
            expect(getSelectionIndices(simpleVector.filter(30n))).toEqual([2]);
        });

        it("should return multiple indices for duplicate values", () => {
            expect(getSelectionIndices(withDuplicates.filter(20n))).toEqual([1, 3]);
            expect(getSelectionIndices(withDuplicates.filter(10n))).toEqual([0, 5]);
        });

        it("should return empty when no match", () => {
            expect(simpleVector.filter(999n).limit).toBe(0);
        });

        it("should filter first element", () => {
            expect(getSelectionIndices(simpleVector.filter(10n))).toEqual([0]);
        });

        it("should filter last element", () => {
            expect(getSelectionIndices(simpleVector.filter(50n))).toEqual([4]);
        });

        it("should respect nullability", () => {
            expect(getSelectionIndices(withNulls.filter(30n))).toEqual([2]);
            expect(getSelectionIndices(withNulls.filter(40n))).toEqual([]);
        });

        it("should handle empty vector", () => {
            expect(emptyVector.filter(10n).limit).toBe(0);
        });

        it("should handle negative BigInts", () => {
            expect(getSelectionIndices(negativeVector.filter(-30n))).toEqual([1]);
            expect(getSelectionIndices(negativeVector.filter(0n))).toEqual([3]);
        });

        it("should handle large BigInt values", () => {
            expect(getSelectionIndices(largeValues.filter(-9223372036854775808n))).toEqual([0]);
            expect(getSelectionIndices(largeValues.filter(9223372036854775807n))).toEqual([4]);
        });

        it("should return FlatSelectionVector", () => {
            const result = simpleVector.filter(10n);
            expect(result).toBeInstanceOf(FlatSelectionVector);
        });
    });

    describe("match", () => {
        it("should match multiple values", () => {
            expect(getSelectionIndices(simpleVector.match([10n, 50n]))).toEqual([0, 4]);
        });

        it("should match all values in vector", () => {
            expect(getSelectionIndices(simpleVector.match([10n, 20n, 30n, 40n, 50n]))).toEqual([0, 1, 2, 3, 4]);
        });

        it("should handle duplicates in vector", () => {
            expect(getSelectionIndices(withDuplicates.match([10n, 50n]))).toEqual([0, 4, 5]);
            expect(getSelectionIndices(withDuplicates.match([20n]))).toEqual([1, 3]);
        });

        it("should return empty for non-existent values", () => {
            expect(simpleVector.match([999n, 888n]).limit).toBe(0);
        });

        it("should handle empty test array", () => {
            expect(simpleVector.match([]).limit).toBe(0);
        });

        it("should respect nullability", () => {
            expect(getSelectionIndices(withNulls.match([10n, 40n]))).toEqual([0]);
        });

        it("should handle single value in array", () => {
            expect(getSelectionIndices(simpleVector.match([30n]))).toEqual([2]);
        });

        it("should handle duplicate matches in test values", () => {
            const multiVec = createVector([10n, 20n, 10n, 30n, 50n]);
            const result = getSelectionIndices(multiVec.match([10n, 10n, 50n]));
            expect(result).toEqual([0, 0, 2, 2, 4]);
        });

        it("should match in order of appearance", () => {
            expect(getSelectionIndices(simpleVector.match([50n, 10n, 30n]))).toEqual([0, 2, 4]);
        });

        it("should return FlatSelectionVector", () => {
            const result = simpleVector.match([10n, 20n]);
            expect(result).toBeInstanceOf(FlatSelectionVector);
        });
    });

    describe("filterSelected", () => {
        it("should filter from selection", () => {
            const selection = new FlatSelectionVector([0, 1, 3, 4]);
            withDuplicates.filterSelected(20n, selection);
            expect(getSelectionIndices(selection)).toEqual([1, 3]);
        });

        it("should handle no matches", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3]);
            simpleVector.filterSelected(999n, selection);
            expect(getSelectionIndices(selection)).toEqual([]);
            expect(selection.limit).toBe(0);
        });

        it("should update limit correctly", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            simpleVector.filterSelected(30n, selection);
            expect(selection.limit).toBe(1);
        });

        it("should handle single element selection", () => {
            const selection = new FlatSelectionVector([2]);
            simpleVector.filterSelected(30n, selection);
            expect(getSelectionIndices(selection)).toEqual([2]);
        });

        it("should handle empty selection", () => {
            const selection = new FlatSelectionVector([]);
            simpleVector.filterSelected(10n, selection);
            expect(selection.limit).toBe(0);
        });

        it("should respect nullability", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            withNulls.filterSelected(40n, selection);
            expect(getSelectionIndices(selection)).toEqual([]);
        });
    });

    describe("matchSelected", () => {
        it("should match from selection", () => {
            const selection = new FlatSelectionVector([1, 2, 3, 4]);
            simpleVector.matchSelected([20n, 40n], selection);
            expect(getSelectionIndices(selection)).toEqual([1, 3]);
        });

        it("should handle no matches", () => {
            const selection = new FlatSelectionVector([0, 1, 2]);
            simpleVector.matchSelected([999n, 888n], selection);
            expect(getSelectionIndices(selection)).toEqual([]);
        });

        it("should update limit correctly", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            simpleVector.matchSelected([20n, 40n], selection);
            expect(selection.limit).toBe(2);
        });

        it("should preserve selection input order", () => {
            const selection = new FlatSelectionVector([4, 3, 2, 1, 0]);
            simpleVector.matchSelected([10n, 30n], selection);
            expect(getSelectionIndices(selection)).toEqual([2, 0]);
        });

        it("should handle empty test values", () => {
            const selection = new FlatSelectionVector([0, 1, 2]);
            simpleVector.matchSelected([], selection);
            expect(getSelectionIndices(selection)).toEqual([]);
        });

        it("should respect nullability in selected", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            withNulls.matchSelected([30n, 40n], selection);
            expect(getSelectionIndices(selection)).toEqual([2]);
        });
    });

    describe("greaterThanOrEqualTo", () => {
        it("should filter >= threshold", () => {
            expect(getSelectionIndices(simpleVector.greaterThanOrEqualTo(30n))).toEqual([2, 3, 4]);
        });

        it("should include equal values", () => {
            expect(getSelectionIndices(simpleVector.greaterThanOrEqualTo(30n))).toContain(2);
        });

        it("should return all for minimum value", () => {
            expect(getSelectionIndices(simpleVector.greaterThanOrEqualTo(10n))).toEqual([0, 1, 2, 3, 4]);
        });

        it("should return empty for value greater than all", () => {
            expect(simpleVector.greaterThanOrEqualTo(999n).limit).toBe(0);
        });

        it("should handle negative threshold", () => {
            expect(getSelectionIndices(negativeVector.greaterThanOrEqualTo(-10n))).toEqual([2, 3, 4, 5, 6]);
        });

        it("should respect nullability", () => {
            expect(getSelectionIndices(withNulls.greaterThanOrEqualTo(30n))).toEqual([2, 4]);
        });

        it("should handle large BigInt boundaries", () => {
            expect(getSelectionIndices(largeValues.greaterThanOrEqualTo(-9223372036854775808n))).toEqual([0, 1, 2, 3, 4]);
            expect(getSelectionIndices(largeValues.greaterThanOrEqualTo(9223372036854775807n))).toEqual([4]);
        });

        it("should return FlatSelectionVector", () => {
            const result = simpleVector.greaterThanOrEqualTo(30n);
            expect(result).toBeInstanceOf(FlatSelectionVector);
        });
    });

    describe("greaterThanOrEqualToSelected", () => {
        it("should filter selected >= threshold", () => {
            const selection = new FlatSelectionVector([0, 2, 3, 4]);
            simpleVector.greaterThanOrEqualToSelected(30n, selection);
            expect(getSelectionIndices(selection)).toEqual([2, 3, 4]);
        });

        it("should update limit correctly", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            simpleVector.greaterThanOrEqualToSelected(30n, selection);
            expect(selection.limit).toBe(3);
        });

        it("should handle no matches", () => {
            const selection = new FlatSelectionVector([0, 1, 2]);
            simpleVector.greaterThanOrEqualToSelected(999n, selection);
            expect(getSelectionIndices(selection)).toEqual([]);
        });

        it("should preserve selection input order", () => {
            const selection = new FlatSelectionVector([4, 3, 2, 1, 0]);
            simpleVector.greaterThanOrEqualToSelected(30n, selection);
            expect(getSelectionIndices(selection)).toEqual([4, 3, 2]);
        });
    });

    describe("smallerThanOrEqualTo", () => {
        it("should filter <= threshold", () => {
            expect(getSelectionIndices(simpleVector.smallerThanOrEqualTo(30n))).toEqual([0, 1, 2]);
        });

        it("should include equal values", () => {
            expect(getSelectionIndices(simpleVector.smallerThanOrEqualTo(30n))).toContain(2);
        });

        it("should return all for maximum value", () => {
            expect(getSelectionIndices(simpleVector.smallerThanOrEqualTo(50n))).toEqual([0, 1, 2, 3, 4]);
        });

        it("should return empty for value less than all", () => {
            expect(simpleVector.smallerThanOrEqualTo(1n).limit).toBe(0);
        });

        it("should handle negative threshold", () => {
            expect(getSelectionIndices(negativeVector.smallerThanOrEqualTo(-10n))).toEqual([0, 1, 2]);
        });

        it("should respect nullability", () => {
            expect(getSelectionIndices(withNulls.smallerThanOrEqualTo(30n))).toEqual([0, 1, 2]);
        });

        it("should handle large BigInt boundaries", () => {
            expect(getSelectionIndices(largeValues.smallerThanOrEqualTo(-9223372036854775808n))).toEqual([0]);
            expect(getSelectionIndices(largeValues.smallerThanOrEqualTo(9223372036854775807n)).length).toBeGreaterThan(0);
        });

        it("should return FlatSelectionVector", () => {
            const result = simpleVector.smallerThanOrEqualTo(30n);
            expect(result).toBeInstanceOf(FlatSelectionVector);
        });
    });

    describe("smallerThanOrEqualToSelected", () => {
        it("should filter selected <= threshold", () => {
            const selection = new FlatSelectionVector([1, 2, 3, 4]);
            simpleVector.smallerThanOrEqualToSelected(30n, selection);
            expect(getSelectionIndices(selection)).toEqual([1, 2]);
        });

        it("should update limit correctly", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            simpleVector.smallerThanOrEqualToSelected(30n, selection);
            expect(selection.limit).toBe(3);
        });

        it("should handle no matches", () => {
            const selection = new FlatSelectionVector([3, 4]);
            simpleVector.smallerThanOrEqualToSelected(1n, selection);
            expect(getSelectionIndices(selection)).toEqual([]);
        });

        it("should preserve selection input order", () => {
            const selection = new FlatSelectionVector([4, 3, 2, 1, 0]);
            simpleVector.smallerThanOrEqualToSelected(30n, selection);
            expect(getSelectionIndices(selection)).toEqual([2, 1, 0]);
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

        it("should return all when filtering for non-existent value", () => {
            expect(getSelectionIndices(simpleVector.filterNotEqual(999n)).length).toBe(5);
        });

        it("should return empty when all values match", () => {
            const sameVec = createVector([10n, 10n, 10n]);
            expect(getSelectionIndices(sameVec.filterNotEqual(10n))).toEqual([]);
        });

        it("should handle negative values", () => {
            expect(getSelectionIndices(negativeVector.filterNotEqual(0n))).toHaveLength(6);
        });

        it("should return FlatSelectionVector", () => {
            const result = simpleVector.filterNotEqual(10n);
            expect(result).toBeInstanceOf(FlatSelectionVector);
        });

        it("should respect nullability correctly", () => {
            const result = withNulls.filterNotEqual(10n);
            expect(getSelectionIndices(result)).toEqual([1, 2, 3, 4]);
        });

        it("should handle large BigInt values", () => {
            const result = largeValues.filterNotEqual(0n);
            expect(getSelectionIndices(result)).toHaveLength(4);
        });
    });

    describe("filterNotEqualSelected", () => {
        it("should filter != from selection", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3]);
            withDuplicates.filterNotEqualSelected(20n, selection);
            expect(getSelectionIndices(selection)).toEqual([0, 2]);
        });

        it("should update limit correctly", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            simpleVector.filterNotEqualSelected(30n, selection);
            expect(selection.limit).toBe(4);
        });

        it("should handle all match case", () => {
            const selection = new FlatSelectionVector([0, 1, 2]);
            const sameVec = createVector([10n, 10n, 10n]);
            sameVec.filterNotEqualSelected(10n, selection);
            expect(selection.limit).toBe(0);
        });

        it("should include nulls in result", () => {
            const vec = createNullableVector([10n, 20n, 30n, 40n], 0b00001011);
            const selection = new FlatSelectionVector([0, 1, 2, 3]);
            vec.filterNotEqualSelected(20n, selection);
            expect(getSelectionIndices(selection)).toEqual([0, 2, 3]);
        });

        it("should preserve selection order", () => {
            const selection = new FlatSelectionVector([4, 3, 2, 1, 0]);
            simpleVector.filterNotEqualSelected(30n, selection);
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
            const allNull = createNullableVector([10n, 20n, 30n], 0b00000000);
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
            const allNull = createNullableVector([10n, 20n, 30n], 0b00000000);
            expect(getSelectionIndices(allNull.nullableValues())).toEqual([0, 1, 2]);
        });

        it("should return FlatSelectionVector", () => {
            const result = withNulls.nullableValues();
            expect(result).toBeInstanceOf(FlatSelectionVector);
        });
    });

    describe("unimplemented methods", () => {
        it("should throw for noneMatch", () => {
            expect(() => simpleVector.noneMatch([10n])).toThrow("Not implemented yet.");
        });

        it("should throw for noneMatchSelected", () => {
            const sel = new FlatSelectionVector([0]);
            expect(() => simpleVector.noneMatchSelected([10n], sel)).toThrow("Not implemented yet.");
        });
    });

    describe("edge cases and special values", () => {
        it("should handle BigInt64 limits", () => {
            const minInt = -9223372036854775808n;
            const maxInt = 9223372036854775807n;
            expect(getSelectionIndices(largeValues.filter(maxInt))).toEqual([4]);
            expect(getSelectionIndices(largeValues.filter(minInt))).toEqual([0]);
        });

        it("should handle zero", () => {
            const zeroVec = createVector([0n, 1n, 0n, 2n, 0n]);
            expect(getSelectionIndices(zeroVec.filter(0n))).toEqual([0, 2, 4]);
        });

        it("should handle single element", () => {
            const single = createVector([42n]);
            expect(getSelectionIndices(single.filter(42n))).toEqual([0]);
        });

        it("should handle large vectors", () => {
            const large = createVector(Array.from({ length: 1000 }, (_, i) => BigInt(i)));
            expect(getSelectionIndices(large.filter(500n))).toEqual([500]);
            expect(getSelectionIndices(large.greaterThanOrEqualTo(999n)).length).toBe(1);
        });

        it("should handle all same values", () => {
            const same = createVector([10n, 10n, 10n, 10n, 10n]);
            expect(getSelectionIndices(same.filter(10n))).toEqual([0, 1, 2, 3, 4]);
            expect(getSelectionIndices(same.filterNotEqual(10n))).toEqual([]);
        });

        it("should handle mixed comparisons correctly", () => {
            const ge30 = getSelectionIndices(simpleVector.greaterThanOrEqualTo(30n));
            const le40 = getSelectionIndices(simpleVector.smallerThanOrEqualTo(40n));
            const intersection = ge30.filter(idx => le40.includes(idx));
            expect(intersection).toEqual([2, 3]);
        });

        it("should handle very large BigInt arithmetic", () => {
            const huge = createVector([
                -9223372036854775808n,
                -1n,
                9223372036854775807n,
            ]);
            expect(getSelectionIndices(huge.greaterThanOrEqualTo(-1n)).length).toBe(2);
        });
    });

    describe("combined operations", () => {
        it("should apply filter then comparison", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            simpleVector.filterSelected(10n, selection);
            expect(getSelectionIndices(selection)).toEqual([0]);
        });

        it("should apply multiple match operations sequentially", () => {
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            simpleVector.matchSelected([10n, 20n, 30n], selection);
            expect(getSelectionIndices(selection)).toEqual([0, 1, 2]);
        });

        it("should chain presentValues with filter operations", () => {
            const present = getSelectionIndices(withNulls.presentValues());
            expect(present).toEqual([0, 1, 2, 4]);
            const ge30 = getSelectionIndices(withNulls.greaterThanOrEqualTo(30n));
            expect(ge30).toEqual([2, 4]);
        });

        it("should use filterNotEqual to exclude specific values", () => {
            const notTwenty = getSelectionIndices(withDuplicates.filterNotEqual(20n));
            expect(notTwenty).toEqual([0, 2, 4, 5]);
            const notTwentySel = new FlatSelectionVector(notTwenty);
            withDuplicates.matchSelected([10n, 50n], notTwentySel);
            expect(getSelectionIndices(notTwentySel)).toEqual([0, 4, 5]);
        });
    });
});
