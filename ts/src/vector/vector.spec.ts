import { describe, it, expect } from "vitest";
import { IntFlatVector } from "./flat/intFlatVector";
import BitVector from "./flat/bitVector";
import { FlatSelectionVector } from "./filter/flatSelectionVector";
import {
    filter,
    filterSelected,
    filterNotEqual,
    filterNotEqualSelected,
    match,
    matchSelected,
    noneMatch,
    noneMatchSelected,
    presentValues,
    presentValuesSelected,
    nullableValues,
    nullableValuesSelected
} from "./utils";


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

// int is used for base testing since it is the simplest datatype. Edge cases are tested separately in the according vector classes
describe("BaseVector tests", () => {
    describe("filter", () => {
        it("should filter matching values", () => {
            const intVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const result = filter(intVector, 20);
            expect(result.selectionValues()).toStrictEqual(new Uint32Array([1]));
        });

        it("should filter duplicate values", () => {
            const withDuplicates = createVector([10, 20, 30, 20, 50, 10]);
            const result = filter(withDuplicates, 20);
            expect(result.selectionValues()).toStrictEqual(new Uint32Array([1, 3]));
        });

        it("should return empty when no match", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const result = filter(simpleVector, 15);
            expect(result.selectionValues()).toStrictEqual(new Uint32Array([]));
        });

        it("should filter with nullability", () => {
            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);
            const result = filter(withNulls, 30);
            expect(result.selectionValues()).toStrictEqual(new Uint32Array([2]));
        });
    });

    describe("filterSelected", () => {
        it("should filter from selection", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const selection = new FlatSelectionVector(new Uint32Array([1, 3, 4, 6, 8]));
            filterSelected(simpleVector, 20, selection);
            expect(selection.selectionValues()).toEqual(new Uint32Array([1]));
        });

        it("should filter from selection with duplicates", () => {
            const withDuplicates = createVector([10, 20, 30, 20, 50, 10]);
            const selection = new FlatSelectionVector(new Uint32Array([0, 1, 3, 4]));
            filterSelected(withDuplicates, 20, selection);
            expect(selection.selectionValues()).toEqual(new Uint32Array([1, 3]));
        });

        it("should filter from selection with nullability", () => {
            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);
            const selection = new FlatSelectionVector(new Uint32Array([0, 2, 3, 4]));
            filterSelected(withNulls, 30, selection);
            expect(selection.selectionValues()).toEqual(new Uint32Array([2]));
        });
    });

    describe("filterNotEqual", () => {
        it("should filter != threshold in simple vector", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const result = filterNotEqual(simpleVector, 50);
            expect(result.selectionValues()).toEqual(new Uint32Array([0, 1, 2, 3, 5, 6, 7, 8]));
        });

        it("should filter != threshold with duplicates", () => {
            const withDuplicates = createVector([10, 20, 30, 20, 50, 10]);
            const result = filterNotEqual(withDuplicates, 20);
            expect(result.selectionValues()).toEqual(new Uint32Array([0, 2, 4, 5]));
        });

        it("should filter != threshold with nullability", () => {
            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);
            const result = filterNotEqual(withNulls, 30);
            expect(result.selectionValues()).toEqual(new Uint32Array([0, 1, 3, 4]));
        });
    });

    describe("filterNotEqualSelected", () => {
        it("should filter != from selection", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const selection = new FlatSelectionVector(new Uint32Array([1, 3, 4, 5, 7]));
            filterNotEqualSelected(simpleVector, 50, selection);
            expect(selection.selectionValues()).toEqual(new Uint32Array([1, 3, 5, 7]));
        });

        it("should filter != from selection with duplicates", () => {
            const withDuplicates = createVector([10, 20, 30, 20, 50, 10]);
            const selection = new FlatSelectionVector(new Uint32Array([1, 2, 3, 4, 5]));
            filterNotEqualSelected(withDuplicates, 20, selection);
            expect(selection.selectionValues()).toEqual(new Uint32Array([2, 4, 5]));
        });

        it("should filter != from selection with nullability", () => {
            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);
            const selection = new FlatSelectionVector(new Uint32Array([0, 2, 3, 4]));
            filterNotEqualSelected(withNulls, 30, selection);
            expect(selection.selectionValues()).toEqual(new Uint32Array([0, 3, 4]));
        });
    });

    describe("match", () => {
        it("should match multiple values in simple vector", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const result = match(simpleVector, [10, 50]);
            expect(result.selectionValues()).toStrictEqual(new Uint32Array([0, 4]));
        });

        it("should match multiple values with duplicates", () => {
            const withDuplicates = createVector([10, 20, 30, 20, 50, 10]);
            const result = match(withDuplicates, [10, 50]);
            expect(result.selectionValues()).toStrictEqual(new Uint32Array([0, 4, 5]));
        });

        it("should match with nullability", () => {
            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);
            const result = match(withNulls, [10, 40]);
            expect(result.selectionValues()).toStrictEqual(new Uint32Array([0]));
        });
    });

    describe("matchSelected", () => {
        it("should match from selection", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const selection = new FlatSelectionVector(new Uint32Array([0, 1, 3, 4, 6]));
            matchSelected(simpleVector, [20, 40], selection);
            expect(selection.selectionValues()).toEqual(new Uint32Array([1, 3]));
        });

        it("should match from selection with duplicates", () => {
            const withDuplicates = createVector([10, 20, 30, 20, 50, 10]);
            const selection = new FlatSelectionVector(new Uint32Array([1, 3, 4, 5]));
            matchSelected(withDuplicates, [20, 50], selection);
            expect(selection.selectionValues()).toEqual(new Uint32Array([1, 3, 4]));
        });

        it("should match from selection with nullability", () => {
            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);
            const selection = new FlatSelectionVector(new Uint32Array([0, 2, 3, 4]));
            matchSelected(withNulls, [10, 50], selection);
            expect(selection.selectionValues()).toEqual(new Uint32Array([0, 4]));
        });
    });

    describe("noneMatch", () => {
        it("should return values not in match array", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const result = noneMatch(simpleVector, [20, 50, 80]);
            expect(result.selectionValues()).toStrictEqual(new Uint32Array([0, 2, 3, 5, 6, 8]));
        });

        it("should handle duplicate values when none match", () => {
            const withDuplicates = createVector([10, 20, 30, 20, 50, 10]);
            const result = noneMatch(withDuplicates, [20, 50]);
            expect(result.selectionValues()).toStrictEqual(new Uint32Array([0, 2, 5]));
        });

        it("should exclude null values and return non-matching", () => {
            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);
            const result = noneMatch(withNulls, [20, 40]);
            expect(result.selectionValues()).toStrictEqual(new Uint32Array([0, 2, 4]));
        });

        it("should return empty when all values match", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30]);
            const result = noneMatch(simpleVector, [10, 20, 30]);
            expect(result.selectionValues()).toStrictEqual(new Uint32Array([]));
        });

        it("should return all present values when no values match", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30]);
            const result = noneMatch(simpleVector, [40, 50, 60]);
            expect(result.selectionValues()).toStrictEqual(new Uint32Array([0, 1, 2]));
        });
    });

    describe("noneMatchSelected", () => {
        it("should filter non-matching values from selection", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const selection = new FlatSelectionVector(new Uint32Array([1, 3, 4, 7, 8]));
            noneMatchSelected(simpleVector, [20, 80], selection);
            expect(selection.selectionValues()).toEqual(new Uint32Array([3, 4, 8]));
        });

        it("should handle duplicates in selection", () => {
            const withDuplicates = createVector([10, 20, 30, 20, 50, 10]);
            const selection = new FlatSelectionVector(new Uint32Array([0, 1, 2, 4, 5]));
            noneMatchSelected(withDuplicates, [20], selection);
            expect(selection.selectionValues()).toEqual(new Uint32Array([0, 2, 4, 5]));
        });

        it("should filter from selection with nullability", () => {
            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);
            const selection = new FlatSelectionVector(new Uint32Array([0, 1, 2, 4]));
            noneMatchSelected(withNulls, [10], selection);
            expect(selection.selectionValues()).toEqual(new Uint32Array([1, 2, 4]));
        });
    });

    describe("presentValues", () => {
        it("should return all indices for vector without nulls", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const result = presentValues(simpleVector);
            expect(result.selectionValues()).toEqual(new Uint32Array([0, 1, 2, 3, 4, 5, 6, 7, 8]));
        });

        it("should return indices of present (non-null) values", () => {
            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);
            const result = presentValues(withNulls);
            expect(result.selectionValues()).toEqual(new Uint32Array([0, 1, 2, 4]));
        });
    });

    describe("presentValuesSelected", () => {
        it("should filter present values from selection", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const selection = new FlatSelectionVector(new Uint32Array([0, 2, 4, 6, 8]));
            presentValuesSelected(simpleVector, selection);
            expect(selection.selectionValues()).toEqual(new Uint32Array([0, 2, 4, 6, 8]));
        });

        it("should filter out null values from selection", () => {
            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);
            const selection = new FlatSelectionVector(new Uint32Array([0, 2, 3, 4]));
            presentValuesSelected(withNulls, selection);
            expect(selection.selectionValues()).toEqual(new Uint32Array([0, 2, 4]));
        });
    });

    describe("nullableValues", () => {
        it("should return empty array for vector without nulls", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const result = nullableValues(simpleVector);
            expect(result.selectionValues()).toEqual(new Uint32Array([]));
        });

        it("should return indices of null values", () => {
            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);
            const result = nullableValues(withNulls);
            expect(result.selectionValues()).toEqual(new Uint32Array([3]));
        });
    });

    describe("nullableValuesSelected", () => {
        it("should return empty for vector without nulls", () => {
            const simpleVector: IntFlatVector = createVector([10, 20, 30, 40, 50, 60, 70, 80, 90]);
            const selection = new FlatSelectionVector(new Uint32Array([0, 2, 4, 6, 8]));
            nullableValuesSelected(simpleVector, selection);
            expect(selection.selectionValues()).toEqual(new Uint32Array([]));
        });

        it("should filter only null values from selection", () => {
            const withNulls = createNullableVector([10, 20, 30, 40, 50], 0b00010111);
            const selection = new FlatSelectionVector(new Uint32Array([0, 2, 3, 4]));
            nullableValuesSelected(withNulls, selection);
            expect(selection.selectionValues()).toEqual(new Uint32Array([3]));
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
