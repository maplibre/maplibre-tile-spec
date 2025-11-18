import { describe, it, expect } from "vitest";
import { IntFlatVector } from "./flat/intFlatVector";
import BitVector from "./flat/bitVector";
import { FlatSelectionVector } from "./filter/flatSelectionVector";


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

/*
 * the use of the .slice method has to be discussed (is it intentional not in flatSelectionVector or can it be added there)
 * for now the flatSelectionVector stays the same and the tests use the .slice method for validation
 */

// int is used for base testing since it is the simplest datatype. Edge cases are tested separately in the according vector classes
describe("BaseVector tests", () => {
    describe("filter", () => {
        it("should filter matching values", () => {
            const intVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const result = intVector.filter(20);
            expect(result.selectionValues()).toStrictEqual([1]);
        });

        it("should filter duplicate values", () => {
            const withDuplicates = createVector([10, 20, 30, 20, 50, 10]);
            const result = withDuplicates.filter(20);
            expect(result.selectionValues()).toStrictEqual([1, 3]);
        });

        it("should return empty when no match", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const result = simpleVector.filter(15);
            expect(result.selectionValues()).toStrictEqual([]);
        });

        it("should filter with nullability", () => {
            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);
            const result = withNulls.filter(30);
            expect(result.selectionValues()).toStrictEqual([2]);
        });
    });

    describe("filterSelected", () => {
        it("should filter from selection", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const selection = new FlatSelectionVector([1, 3, 4, 6, 8]);
            simpleVector.filterSelected(20, selection);
            expect(selection.selectionValues().slice(0, selection.limit)).toEqual([1]);
        });

        it("should filter from selection with duplicates", () => {
            const withDuplicates = createVector([10, 20, 30, 20, 50, 10]);
            const selection = new FlatSelectionVector([0, 1, 3, 4]);
            withDuplicates.filterSelected(20, selection);
            expect(selection.selectionValues().slice(0, selection.limit)).toEqual([1, 3]);
        });

        it("should filter from selection with nullability", () => {
            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);
            const selection = new FlatSelectionVector([0, 2, 3, 4]);
            withNulls.filterSelected(30, selection);
            expect(selection.selectionValues().slice(0, selection.limit)).toEqual([2]);
        });
    });

    describe("filterNotEqual", () => {
        it("should filter != threshold in simple vector", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const result = simpleVector.filterNotEqual(50);
            expect(result.selectionValues()).toEqual([0, 1, 2, 3, 5, 6, 7, 8]);
        });

        it("should filter != threshold with duplicates", () => {
            const withDuplicates = createVector([10, 20, 30, 20, 50, 10]);
            const result = withDuplicates.filterNotEqual(20);
            expect(result.selectionValues()).toEqual([0, 2, 4, 5]);
        });

        it("should filter != threshold with nullability", () => {
            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);
            const result = withNulls.filterNotEqual(30);
            expect(result.selectionValues()).toEqual([0, 1, 3, 4]);
        });
    });

    describe("filterNotEqualSelected", () => {
        it("should filter != from selection", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const selection = new FlatSelectionVector([1, 3, 4, 5, 7]);
            simpleVector.filterNotEqualSelected(50, selection);
            expect(selection.selectionValues().slice(0, selection.limit)).toEqual([1, 3, 5, 7]);
        });

        it("should filter != from selection with duplicates", () => {
            const withDuplicates = createVector([10, 20, 30, 20, 50, 10]);
            const selection = new FlatSelectionVector([1, 2, 3, 4, 5]);
            withDuplicates.filterNotEqualSelected(20, selection);
            expect(selection.selectionValues().slice(0, selection.limit)).toEqual([2, 4, 5]);
        });

        it("should filter != from selection with nullability", () => {
            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);
            const selection = new FlatSelectionVector([0, 2, 3, 4]);
            withNulls.filterNotEqualSelected(30, selection);
            expect(selection.selectionValues().slice(0, selection.limit)).toEqual([0, 3, 4]);
        });
    });

    describe("match", () => {
        it("should match multiple values in simple vector", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const result = simpleVector.match([10, 50]);
            expect(result.selectionValues()).toStrictEqual([0, 4]);
        });

        it("should match multiple values with duplicates", () => {
            const withDuplicates = createVector([10, 20, 30, 20, 50, 10]);
            const result = withDuplicates.match([10, 50]);
            expect(result.selectionValues()).toStrictEqual([0, 4, 5]);
        });

        it("should match with nullability", () => {
            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);
            const result = withNulls.match([10, 40]);
            expect(result.selectionValues()).toStrictEqual([0]);
        });
    });

    describe("matchSelected", () => {
        it("should match from selection", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const selection = new FlatSelectionVector([0, 1, 3, 4, 6]);
            simpleVector.matchSelected([20, 40], selection);
            expect(selection.selectionValues().slice(0, selection.limit)).toEqual([1, 3]);
        });

        it("should match from selection with duplicates", () => {
            const withDuplicates = createVector([10, 20, 30, 20, 50, 10]);
            const selection = new FlatSelectionVector([1, 3, 4, 5]);
            withDuplicates.matchSelected([20, 50], selection);
            expect(selection.selectionValues().slice(0, selection.limit)).toEqual([1, 3, 4]);
        });

        it("should match from selection with nullability", () => {
            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);
            const selection = new FlatSelectionVector([0, 2, 3, 4]);
            withNulls.matchSelected([10, 50], selection);
            expect(selection.selectionValues().slice(0, selection.limit)).toEqual([0, 4]);
        });
    });

    describe("noneMatch", () => {
        it("should return values not in match array", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const result = simpleVector.noneMatch([20, 50, 80]);
            expect(result.selectionValues()).toStrictEqual([0, 2, 3, 5, 6, 8]);
        });

        it("should handle duplicate values when none match", () => {
            const withDuplicates = createVector([10, 20, 30, 20, 50, 10]);
            const result = withDuplicates.noneMatch([20, 50]);
            expect(result.selectionValues()).toStrictEqual([0, 2, 5]);
        });

        it("should exclude null values and return non-matching", () => {
            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);
            const result = withNulls.noneMatch([20, 40]);
            expect(result.selectionValues()).toStrictEqual([0, 2, 4]);
        });

        it("should return empty when all values match", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30]);
            const result = simpleVector.noneMatch([10, 20, 30]);
            expect(result.selectionValues()).toStrictEqual([]);
        });

        it("should return all present values when no values match", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30]);
            const result = simpleVector.noneMatch([40, 50, 60]);
            expect(result.selectionValues()).toStrictEqual([0, 1, 2]);
        });
    });

    describe("noneMatchSelected", () => {
        it("should filter non-matching values from selection", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const selection = new FlatSelectionVector([1, 3, 4, 7, 8]);
            simpleVector.noneMatchSelected([20, 80], selection);
            expect(selection.selectionValues().slice(0, selection.limit)).toEqual([3, 4, 8]);
        });

        it("should handle duplicates in selection", () => {
            const withDuplicates = createVector([10, 20, 30, 20, 50, 10]);
            const selection = new FlatSelectionVector([0, 1, 2, 4, 5]);
            withDuplicates.noneMatchSelected([20], selection);
            expect(selection.selectionValues().slice(0, selection.limit)).toEqual([0, 2, 4, 5]);
        });

        it("should filter from selection with nullability", () => {
            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);
            const selection = new FlatSelectionVector([0, 1, 2, 4]);
            withNulls.noneMatchSelected([10], selection);
            expect(selection.selectionValues().slice(0, selection.limit)).toEqual([1, 2, 4]);
        });
    });

    describe("greaterThanOrEqualTo", () => {
        it("should filter >= threshold in simple vector", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const result = simpleVector.greaterThanOrEqualTo(70);
            expect(result.selectionValues()).toEqual([6, 7, 8]);
        });

        it("should filter >= threshold with duplicates", () => {
            const withDuplicates = createVector([10, 20, 30, 20, 50, 10]);
            const result = withDuplicates.greaterThanOrEqualTo(20);
            expect(result.selectionValues()).toEqual([1, 2, 3, 4]);
        });

        it("should filter >= threshold with nullability", () => {
            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);
            const result = withNulls.greaterThanOrEqualTo(30);
            expect(result.selectionValues()).toEqual([2, 4]);
        });
    });

    describe("greaterThanOrEqualToSelected", () => {
        it("should filter >= from selection", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const selection = new FlatSelectionVector([0, 1, 3, 4, 6]);
            simpleVector.greaterThanOrEqualToSelected(40, selection);
            expect(selection.selectionValues().slice(0, selection.limit)).toEqual([3, 4, 6]);
        });

        it("should filter >= from selection with duplicates", () => {
            const withDuplicates = createVector([10, 20, 30, 20, 50, 10]);
            const selection = new FlatSelectionVector([1, 2, 3, 4, 5]);
            withDuplicates.greaterThanOrEqualToSelected(20, selection);
            expect(selection.selectionValues().slice(0, selection.limit)).toEqual([1, 2, 3, 4]);
        });

        it("should filter >= from selection with nullability", () => {
            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);
            const selection = new FlatSelectionVector([1, 2, 3, 4]);
            withNulls.greaterThanOrEqualToSelected(30, selection);
            expect(selection.selectionValues().slice(0, selection.limit)).toEqual([2, 4]);
        });
    });

    describe("smallerThanOrEqualTo", () => {
        it("should filter <= threshold in simple vector", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const result = simpleVector.smallerThanOrEqualTo(50);
            expect(result.selectionValues()).toEqual([0, 1, 2, 3, 4]);
        });

        it("should filter <= threshold with duplicates", () => {
            const withDuplicates = createVector([10, 20, 30, 20, 50, 10]);
            const result = withDuplicates.smallerThanOrEqualTo(30);
            expect(result.selectionValues()).toEqual([0, 1, 2, 3, 5]);
        });

        it("should filter <= threshold with nullability", () => {
            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);
            const result = withNulls.smallerThanOrEqualTo(30);
            expect(result.selectionValues()).toEqual([0, 1, 2]);
        });
    });

    describe("smallerThanOrEqualToSelected", () => {
        it("should filter <= from selection", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const selection = new FlatSelectionVector([0, 2, 4, 6, 8]);
            simpleVector.smallerThanOrEqualToSelected(50, selection);
            expect(selection.selectionValues().slice(0, selection.limit)).toEqual([0, 2, 4]);
        });

        it("should filter <= from selection with duplicates", () => {
            const withDuplicates = createVector([10, 20, 30, 20, 50, 10]);
            const selection = new FlatSelectionVector([0, 1, 2, 4, 5]);
            withDuplicates.smallerThanOrEqualToSelected(30, selection);
            expect(selection.selectionValues().slice(0, selection.limit)).toEqual([0, 1, 2, 5]);
        });

        it("should filter <= from selection with nullability", () => {
            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);
            const selection = new FlatSelectionVector([0, 2, 3, 4]);
            withNulls.smallerThanOrEqualToSelected(30, selection);
            expect(selection.selectionValues().slice(0, selection.limit)).toEqual([0, 2]);
        });
    });

    describe("presentValues", () => {
        it("should return all indices for vector without nulls", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const result = simpleVector.presentValues();
            expect(result.selectionValues()).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 8]);
        });

        it("should return indices of present (non-null) values", () => {
            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);
            const result = withNulls.presentValues();
            expect(result.selectionValues()).toEqual([0, 1, 2, 4]);
        });
    });

    describe("presentValuesSelected", () => {
        it("should filter present values from selection", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const selection = new FlatSelectionVector([0, 2, 4, 6, 8]);
            simpleVector.presentValuesSelected(selection);
            expect(selection.selectionValues().slice(0, selection.limit)).toEqual([0, 2, 4, 6, 8]);
        });

        it("should filter out null values from selection", () => {
            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);
            const selection = new FlatSelectionVector([0, 2, 3, 4]);
            withNulls.presentValuesSelected(selection);
            expect(selection.selectionValues().slice(0, selection.limit)).toEqual([0, 2, 4]);
        });
    });

    describe("nullableValues", () => {
        it("should return empty array for vector without nulls", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const result = simpleVector.nullableValues();
            expect(result.selectionValues()).toEqual([]);
        });

        it("should return indices of null values", () => {
            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);
            const result = withNulls.nullableValues();
            expect(result.selectionValues()).toEqual([3]);
        });
    });

    describe("nullableValuesSelected", () => {
        it("should return empty for vector without nulls", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const selection = new FlatSelectionVector([0, 2, 4, 6, 8]);
            simpleVector.nullableValuesSelected(selection);
            expect(selection.selectionValues().slice(0, selection.limit)).toEqual([]);
        });

        it("should filter only null values from selection", () => {
            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);
            const selection = new FlatSelectionVector([0, 2, 3, 4]);
            withNulls.nullableValuesSelected(selection);
            expect(selection.selectionValues().slice(0, selection.limit)).toEqual([3]);
        });
    });

    describe("get name", () => {
        it("should return the vector name", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90], "test_name");
            expect(simpleVector.name).toStrictEqual("test_name");

            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111, "test_name");
            expect(withNulls.name).toStrictEqual("test_name");
        })
    });

    describe("has with invalid index", () => {
        it("should return false if index is out of bounds", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            expect(simpleVector.has(-1)).toBe(false);
            expect(simpleVector.has(100)).toBe(false);
        })
    });

    describe("getValue", () => {
        it("should throw error if index is out of bounds", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            expect( () => simpleVector.getValue(-1)).toThrowError("Index out of bounds");
            expect( () => simpleVector.getValue(100)).toThrowError("Index out of bounds");
        });
        it("should return null for null value at valid index", () => {
            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);
            expect(withNulls.getValue(3)).toBe(null);  // index 3 is null
        });
    });
});
