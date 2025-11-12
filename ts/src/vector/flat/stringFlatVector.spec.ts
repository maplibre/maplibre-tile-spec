import { describe, it, expect } from "vitest";
import { StringFlatVector } from "./stringFlatVector";
import { FlatSelectionVector } from "../filter/flatSelectionVector";
import BitVector from "./bitVector";

/**
 * Helper function to create StringFlatVector from array of strings
 * Builds the Arrow-like columnar format with offsetBuffer and dataBuffer
 */
function createStringFlatVector(values: string[], name = "test"): StringFlatVector {
    const encoder = new TextEncoder();

    const dataBuffer = new Uint8Array(
        values.reduce((total, str) => total + encoder.encode(str).length, 0)
    );

    const offsetBuffer = new Int32Array(values.length + 1);
    offsetBuffer[0] = 0;

    let offset = 0;
    for (let i = 0; i < values.length; i++) {
        const encoded = encoder.encode(values[i]);
        dataBuffer.set(encoded, offset);
        offset += encoded.length;
        offsetBuffer[i + 1] = offset;
    }

    return new StringFlatVector(name, offsetBuffer, dataBuffer, null);
}

/**
 * Helper function to create nullable StringFlatVector
 * @param values - array of string values
 * @param nullBits - bit pattern where 1 = present, 0 = null
 * @param name - optional vector name
 */
function createNullableStringVector(values: string[], nullBits: number, name = "test"): StringFlatVector {
    const encoder = new TextEncoder();

    const dataBuffer = new Uint8Array(
        values.reduce((total, str) => total + encoder.encode(str).length, 0)
    );

    const offsetBuffer = new Int32Array(values.length + 1);
    offsetBuffer[0] = 0;

    let offset = 0;
    for (let i = 0; i < values.length; i++) {
        const encoded = encoder.encode(values[i]);
        dataBuffer.set(encoded, offset);
        offset += encoded.length;
        offsetBuffer[i + 1] = offset;
    }

    const nullability = new Uint8Array([nullBits]);
    const bitVector = new BitVector(nullability, values.length);

    return new StringFlatVector(name, offsetBuffer, dataBuffer, bitVector);
}

function getSelectionIndices(selection: { selectionValues: () => number[]; limit: number }): number[] {
    return selection.selectionValues().slice(0, selection.limit);
}

describe("StringFlatVector", () => {

    describe("getValue", () => {
        it("should get values correctly from buffer", () => {
            const vector = createStringFlatVector(["hello", "world", "test"]);
            expect(vector.getValue(0)).toBe("hello");
            expect(vector.getValue(1)).toBe("world");
            expect(vector.getValue(2)).toBe("test");
        });

        it("should return null for null values in nullable vector", () => {
            const withNulls = createNullableStringVector(["test", "data", "value"], 0b00000101); // indices 0,2 present; 1 null
            expect(withNulls.getValue(1)).toBe(null);
        });

        it("should return actual values for non-null indices", () => {
            const withNulls = createNullableStringVector(["test", "data", "value"], 0b00000101);
            expect(withNulls.getValue(0)).toBe("test");
            expect(withNulls.getValue(2)).toBe("value");
        });

        it("should handle empty strings", () => {
            const vector = createStringFlatVector(["", "test", ""]);
            expect(vector.getValue(0)).toBe("");
            expect(vector.getValue(2)).toBe("");
        });

        it("should handle unicode strings", () => {
            const vector = createStringFlatVector(["hello", "世界", "🌍"]);
            expect(vector.getValue(0)).toBe("hello");
            expect(vector.getValue(1)).toBe("世界");
            expect(vector.getValue(2)).toBe("🌍");
        });
    });

    describe("size and name properties", () => {
        it("should have correct size", () => {
            const vector = createStringFlatVector(["a", "b", "c"]);
            expect(vector.size).toBe(3);
        });

        it("should handle empty vector", () => {
            const empty = createStringFlatVector([]);
            expect(empty.size).toBe(0);
        });

        it("should have correct name", () => {
            const vector = createStringFlatVector(["test"], "myVector");
            expect(vector.name).toBe("myVector");
        });

        it("should use default name when provided", () => {
            const vector = createStringFlatVector(["a", "b"], "custom");
            expect(vector.name).toBe("custom");
        });
    });

    describe("has - nullability check", () => {
        it("should return true for non-null values", () => {
            const withNulls = createNullableStringVector(["a", "b", "c", "d", "e"], 0b00010111); // indices 0,1,2,4 present; 3 null
            expect(withNulls.has(0)).toBe(true);
            expect(withNulls.has(1)).toBe(true);
            expect(withNulls.has(2)).toBe(true);
            expect(withNulls.has(4)).toBe(true);
        });

        it("should return false for null values", () => {
            const withNulls = createNullableStringVector(["a", "b", "c", "d", "e"], 0b00010111);
            expect(withNulls.has(3)).toBe(false);
        });

        it("should return true for all in vector without nullability", () => {
            const vector = createStringFlatVector(["a", "b", "c"]);
            expect(vector.has(0)).toBe(true);
            expect(vector.has(1)).toBe(true);
            expect(vector.has(2)).toBe(true);
        });

        it("should return false for out of bounds indices", () => {
            const vector = createStringFlatVector(["a", "b", "c"]);
            expect(vector.has(100)).toBe(false);
            expect(vector.has(-1)).toBe(false);
        });

        it("should handle index at size boundary", () => {
            const vector = createStringFlatVector(["a", "b", "c"]);
            expect(vector.has(3)).toBe(false);
        });
    });

    describe("filter", () => {
        it("should filter by exact value and return matching indices", () => {
            const vector = createStringFlatVector(["test", "data", "test"]);
            const result = vector.filter("test");
            expect(result.selectionValues()).toEqual([0, 2]);
        });

        it("should return FlatSelectionVector instance", () => {
            const vector = createStringFlatVector(["test", "data"]);
            const result = vector.filter("test");
            expect(result).toBeInstanceOf(FlatSelectionVector);
        });

        it("should return empty selection when no matches exist", () => {
            const vector = createStringFlatVector(["test", "data", "hello"]);
            const result = vector.filter("nonexistent");
            expect(result.selectionValues()).toEqual([]);
            expect(result.limit).toBe(0);
        });

        it("should be case-sensitive", () => {
            const vector = createStringFlatVector(["Test", "test", "TEST"]);
            expect(vector.filter("test").selectionValues()).toEqual([1]);
            expect(vector.filter("Test").selectionValues()).toEqual([0]);
        });

        it("should respect nullability and exclude null values", () => {
            const withNulls = createNullableStringVector(["test", "data", "test", "test"], 0b00001101); // indices 0,2,3 present; 1 null
            const result = withNulls.filter("test");
            expect(result.selectionValues()).toEqual([0, 2, 3]);
        });
    });

    describe("filterSelected", () => {
        it("should filter from existing selection", () => {
            const vector = createStringFlatVector(["test", "data", "test", "hello", "test"]);
            const selection = new FlatSelectionVector([0, 1, 2, 4]);
            vector.filterSelected("test", selection);
            expect(getSelectionIndices(selection)).toEqual([0, 2, 4]);
        });

        it("should update limit correctly", () => {
            const vector = createStringFlatVector(["a", "b", "a", "c", "a"]);
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            vector.filterSelected("a", selection);
            expect(selection.limit).toBe(3);
            expect(getSelectionIndices(selection)).toEqual([0, 2, 4]);
        });

        it("should return empty when no matches in selection", () => {
            const vector = createStringFlatVector(["test", "data", "hello"]);
            const selection = new FlatSelectionVector([0, 1, 2]);
            vector.filterSelected("nonexistent", selection);
            expect(getSelectionIndices(selection)).toEqual([]);
            expect(selection.limit).toBe(0);
        });

        it("should handle empty selection", () => {
            const vector = createStringFlatVector(["test", "data", "hello"]);
            const selection = new FlatSelectionVector([]);
            vector.filterSelected("test", selection);
            expect(selection.limit).toBe(0);
        });

        it("should handle single element selection", () => {
            const vector = createStringFlatVector(["test", "data", "hello"]);
            const selection = new FlatSelectionVector([1]);
            vector.filterSelected("data", selection);
            expect(getSelectionIndices(selection)).toEqual([1]);
        });
    });

    describe("match", () => {
        it("should match single predicate and return all matching indices", () => {
            const vector = createStringFlatVector(["test", "data", "test", "hello"]);
            const result = vector.match(["test"]);
            expect(result.selectionValues()).toEqual([0, 2]);
        });

        it("should match multiple predicates", () => {
            const vector = createStringFlatVector(["test", "data", "test", "hello", "data"]);
            const result = vector.match(["test", "hello"]);
            expect(result.selectionValues()).toEqual([0, 2, 3]);
        });

        it("should return FlatSelectionVector instance", () => {
            const vector = createStringFlatVector(["test", "data"]);
            const result = vector.match(["test"]);
            expect(result).toBeInstanceOf(FlatSelectionVector);
        });

        it("should return empty selection when no matches", () => {
            const vector = createStringFlatVector(["test", "data"]);
            const result = vector.match(["nonexistent", "missing"]);
            expect(result.selectionValues()).toEqual([]);
            expect(result.limit).toBe(0);
        });

        it("should respect nullability", () => {
            const withNulls = createNullableStringVector(["test", "data", "test", "hello"], 0b00001101); // indices 0,2,3 present; 1 null
            const result = withNulls.match(["test", "data"]);
            expect(result.selectionValues()).toEqual([0, 2]);
        });
    });

    describe("matchSelected", () => {
        it("should match from existing selection", () => {
            const vector = createStringFlatVector(["a", "b", "c", "d", "e"]);
            const selection = new FlatSelectionVector([1, 2, 3, 4]);
            vector.matchSelected(["b", "d"], selection);
            expect(getSelectionIndices(selection)).toEqual([1, 3]);
        });

        it("should update limit correctly", () => {
            const vector = createStringFlatVector(["a", "b", "c", "d", "e"]);
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            vector.matchSelected(["b", "d"], selection);
            expect(selection.limit).toBe(2);
        });

        it("should return empty when no matches in selection", () => {
            const vector = createStringFlatVector(["test", "data", "hello"]);
            const selection = new FlatSelectionVector([0, 1, 2]);
            vector.matchSelected(["nonexistent", "missing"], selection);
            expect(getSelectionIndices(selection)).toEqual([]);
        });

        it("should preserve selection input order", () => {
            const vector = createStringFlatVector(["a", "b", "c", "d", "e"]);
            const selection = new FlatSelectionVector([4, 3, 2, 1, 0]);
            vector.matchSelected(["a", "c"], selection);
            expect(getSelectionIndices(selection)).toEqual([2, 0]);
        });

        it("should handle empty test values array", () => {
            const vector = createStringFlatVector(["test", "data", "hello"]);
            const selection = new FlatSelectionVector([0, 1, 2]);
            vector.matchSelected([], selection);
            expect(getSelectionIndices(selection)).toEqual([]);
        });
    });

    describe("filterNotEqual", () => {
        it("should filter all values not equal to target", () => {
            const vector = createStringFlatVector(["test", "data", "test", "hello"]);
            const result = vector.filterNotEqual("test");
            expect(result.selectionValues()).toEqual([1, 3]);
        });

        it("should return FlatSelectionVector instance", () => {
            const vector = createStringFlatVector(["test", "data"]);
            const result = vector.filterNotEqual("test");
            expect(result).toBeInstanceOf(FlatSelectionVector);
        });

        it("should return all indices when filtering non-existent value", () => {
            const vector = createStringFlatVector(["a", "b", "c"]);
            const result = vector.filterNotEqual("nonexistent");
            expect(result.selectionValues().length).toBe(3);
            expect(result.selectionValues()).toEqual([0, 1, 2]);
        });

        it("should return empty when all values match", () => {
            const vector = createStringFlatVector(["same", "same", "same"]);
            const result = vector.filterNotEqual("same");
            expect(result.selectionValues()).toEqual([]);
        });

        it("should respect nullability", () => {
            const withNulls = createNullableStringVector(["test", "data", "test", "hello"], 0b00001101); // indices 0,2,3 present; 1 null
            const result = withNulls.filterNotEqual("test");
            expect(result.selectionValues()).toEqual([3]);
        });
    });

    describe("filterNotEqualSelected", () => {
        it("should filter not equal from selection", () => {
            const vector = createStringFlatVector(["test", "data", "test", "hello"]);
            const selection = new FlatSelectionVector([0, 1, 2, 3]);
            vector.filterNotEqualSelected("test", selection);
            expect(getSelectionIndices(selection)).toEqual([1, 3]);
        });

        it("should update limit correctly", () => {
            const vector = createStringFlatVector(["a", "b", "a", "c", "d"]);
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            vector.filterNotEqualSelected("a", selection);
            expect(selection.limit).toBe(3);
        });

        it("should return empty when all in selection match", () => {
            const vector = createStringFlatVector(["same", "same", "same"]);
            const selection = new FlatSelectionVector([0, 1, 2]);
            vector.filterNotEqualSelected("same", selection);
            expect(selection.limit).toBe(0);
        });

        it("should preserve selection order", () => {
            const vector = createStringFlatVector(["a", "b", "c", "d", "e"]);
            const selection = new FlatSelectionVector([4, 3, 2, 1, 0]);
            vector.filterNotEqualSelected("c", selection);
            expect(getSelectionIndices(selection)).toEqual([4, 3, 1, 0]);
        });

        it("should handle empty selection", () => {
            const vector = createStringFlatVector(["test", "data"]);
            const selection = new FlatSelectionVector([]);
            vector.filterNotEqualSelected("test", selection);
            expect(selection.limit).toBe(0);
        });
    });

    describe("noneMatch", () => {
        it("should return indices that match none of the predicates", () => {
            const vector = createStringFlatVector(["test", "data", "hello", "world"]);
            const result = vector.noneMatch(["test", "hello"]);
            expect(result.selectionValues()).toEqual([1, 3]);
        });

        it("should return FlatSelectionVector instance", () => {
            const vector = createStringFlatVector(["test", "data"]);
            const result = vector.noneMatch(["test"]);
            expect(result).toBeInstanceOf(FlatSelectionVector);
        });

        it("should return all indices when no predicates match anything", () => {
            const vector = createStringFlatVector(["a", "b", "c"]);
            const result = vector.noneMatch(["x", "y", "z"]);
            expect(result.selectionValues().length).toBe(3);
        });

        it("should return empty when all values match at least one predicate", () => {
            const vector = createStringFlatVector(["a", "b", "c"]);
            const result = vector.noneMatch(["a", "b", "c"]);
            expect(result.selectionValues()).toEqual([]);
        });

        it("should respect nullability", () => {
            const withNulls = createNullableStringVector(["test", "data", "hello", "world"], 0b00001101); // indices 0,2,3 present; 1 null
            const result = withNulls.noneMatch(["test", "hello"]);
            expect(result.selectionValues()).toEqual([3]);
        });
    });

    describe("noneMatchSelected", () => {
        it("should filter selection to values matching none of predicates", () => {
            const vector = createStringFlatVector(["test", "data", "hello", "world", "foo"]);
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            vector.noneMatchSelected(["test", "hello"], selection);
            expect(getSelectionIndices(selection)).toEqual([1, 3, 4]);
        });

        it("should update limit correctly", () => {
            const vector = createStringFlatVector(["a", "b", "c", "d", "e"]);
            const selection = new FlatSelectionVector([0, 1, 2, 3, 4]);
            vector.noneMatchSelected(["a", "c"], selection);
            expect(selection.limit).toBe(3);
        });

        it("should return empty when all in selection match predicates", () => {
            const vector = createStringFlatVector(["a", "b", "c"]);
            const selection = new FlatSelectionVector([0, 1, 2]);
            vector.noneMatchSelected(["a", "b", "c"], selection);
            expect(getSelectionIndices(selection)).toEqual([]);
        });

        it("should preserve selection order", () => {
            const vector = createStringFlatVector(["a", "b", "c", "d", "e"]);
            const selection = new FlatSelectionVector([4, 3, 2, 1, 0]);
            vector.noneMatchSelected(["a", "c"], selection);
            expect(getSelectionIndices(selection)).toEqual([4, 3, 1]);
        });

        it("should handle empty test values", () => {
            const vector = createStringFlatVector(["test", "data"]);
            const selection = new FlatSelectionVector([0, 1]);
            vector.noneMatchSelected([], selection);
            expect(getSelectionIndices(selection)).toEqual([0, 1]);
        });
    });

    describe("presentValues", () => {
        it("should return all indices for vector without nullability", () => {
            const vector = createStringFlatVector(["a", "b", "c"]);
            const result = vector.presentValues();
            expect(result.selectionValues()).toEqual([0, 1, 2]);
        });

        it("should return only non-null indices", () => {
            const withNulls = createNullableStringVector(["a", "b", "c", "d", "e"], 0b00010111); // indices 0,1,2,4 present; 3 null
            const result = withNulls.presentValues();
            expect(result.selectionValues()).toEqual([0, 1, 2, 4]);
        });

        it("should return FlatSelectionVector instance", () => {
            const vector = createStringFlatVector(["test"]);
            const result = vector.presentValues();
            expect(result).toBeInstanceOf(FlatSelectionVector);
        });

        it("should return empty for all-null vector", () => {
            const allNull = createNullableStringVector(["a", "b", "c"], 0b00000000);
            const result = allNull.presentValues();
            expect(result.limit).toBe(0);
        });

        it("should handle empty vector", () => {
            const empty = createStringFlatVector([]);
            const result = empty.presentValues();
            expect(result.selectionValues()).toEqual([]);
        });
    });

    describe("nullableValues", () => {
        it("should return empty for vector without nullability", () => {
            const vector = createStringFlatVector(["a", "b", "c"]);
            const result = vector.nullableValues();
            expect(result.limit).toBe(0);
        });

        it("should return only null indices", () => {
            const withNulls = createNullableStringVector(["a", "b", "c", "d", "e"], 0b00010111); // indices 0,1,2,4 present; 3 null
            const result = withNulls.nullableValues();
            expect(result.selectionValues()).toEqual([3]);
        });

        it("should return FlatSelectionVector instance", () => {
            const withNulls = createNullableStringVector(["a", "b"], 0b00000001);
            const result = withNulls.nullableValues();
            expect(result).toBeInstanceOf(FlatSelectionVector);
        });

        it("should return all indices for all-null vector", () => {
            const allNull = createNullableStringVector(["a", "b", "c"], 0b00000000);
            const result = allNull.nullableValues();
            expect(result.selectionValues()).toEqual([0, 1, 2]);
        });

        it("should handle multiple null values", () => {
            const withNulls = createNullableStringVector(["a", "b", "c", "d", "e"], 0b00010101); // indices 0,2,4 present; 1,3 null
            const result = withNulls.nullableValues();
            expect(result.selectionValues()).toEqual([1, 3]);
        });
    });

    describe("greaterThanOrEqualTo", () => {
        it("should throw not implemented error", () => {
            const vector = createStringFlatVector(["a", "b", "c"]);
            expect(() => vector.greaterThanOrEqualTo("b")).toThrow("Not implemented yet.");
        });
    });

    describe("greaterThanOrEqualToSelected", () => {
        it("should throw not implemented error", () => {
            const vector = createStringFlatVector(["a", "b", "c"]);
            const selection = new FlatSelectionVector([0, 1, 2]);
            expect(() => vector.greaterThanOrEqualToSelected("b", selection)).toThrow("Not implemented yet.");
        });
    });

    describe("smallerThanOrEqualTo", () => {
        it("should throw not implemented error", () => {
            const vector = createStringFlatVector(["a", "b", "c"]);
            expect(() => vector.smallerThanOrEqualTo("b")).toThrow("Not implemented yet.");
        });
    });

    describe("smallerThanOrEqualToSelected", () => {
        it("should throw not implemented error", () => {
            const vector = createStringFlatVector(["a", "b", "c"]);
            const selection = new FlatSelectionVector([0, 1, 2]);
            expect(() => vector.smallerThanOrEqualToSelected("b", selection)).toThrow("Not implemented yet.");
        });
    });
});
