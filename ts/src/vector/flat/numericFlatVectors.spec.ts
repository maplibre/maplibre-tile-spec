import { describe, it, expect } from "vitest";
import { DoubleFlatVector } from "./doubleFlatVector";
import { FloatFlatVector } from "./floatFlatVector";
import BitVector from "./bitVector";
import { FlatSelectionVector } from "../filter/flatSelectionVector";

function getSelectionIndices(selection: { selectionValues: () => number[]; limit: number }): number[] {
    return selection.selectionValues().slice(0, selection.limit);
}

describe("DoubleFlatVector", () => {
    function createVector(values: number[]): DoubleFlatVector {
        return new DoubleFlatVector("test", new Float64Array(values), values.length);
    }

    function createNullableVector(values: number[], nullBits: number): DoubleFlatVector {
        const data = new Float64Array(values);
        const bitVector = new BitVector(new Uint8Array([nullBits]), values.length);
        return new DoubleFlatVector("test", data, bitVector);
    }

    const vec = createVector([10.5, 20.5, 30.5, 40.5, 50.5]);

    it("should filter by value", () => {
        expect(getSelectionIndices(vec.filter(30.5))).toEqual([2]);
        expect(vec.filter(999.9).limit).toBe(0);
    });

    it("should match values", () => {
        expect(getSelectionIndices(vec.match([10.5, 50.5]))).toEqual([0, 4]);
    });

    it("should filter selected", () => {
        const selection = new FlatSelectionVector([0, 1, 2]);
        vec.filterSelected(20.5, selection);
        expect(getSelectionIndices(selection)).toEqual([1]);
    });

    it("should match selected", () => {
        const selection = new FlatSelectionVector([1, 2, 3]);
        vec.matchSelected([20.5, 40.5], selection);
        expect(getSelectionIndices(selection)).toEqual([1, 3]);
    });

    it("should filter >= threshold", () => {
        expect(getSelectionIndices(vec.greaterThanOrEqualTo(30.5))).toEqual([2, 3, 4]);
    });

    it("should filter <= threshold", () => {
        expect(getSelectionIndices(vec.smallerThanOrEqualTo(30.5))).toEqual([0, 1, 2]);
    });

    it("should filter selected >= threshold", () => {
        const selection = new FlatSelectionVector([1, 2, 3, 4]);
        vec.greaterThanOrEqualToSelected(30.5, selection);
        expect(getSelectionIndices(selection)).toEqual([2, 3, 4]);
    });

    it("should filter selected <= threshold", () => {
        const selection = new FlatSelectionVector([0, 1, 2, 3]);
        vec.smallerThanOrEqualToSelected(20.5, selection);
        expect(getSelectionIndices(selection)).toEqual([0, 1]);
    });

    it("should handle nullability", () => {
        const nullVec = createNullableVector([10.5, 20.5, 30.5, 40.5], 0b0111); // 0,1,2 present
        expect(getSelectionIndices(nullVec.filter(40.5))).toEqual([]);
        expect(getSelectionIndices(nullVec.filter(20.5))).toEqual([1]);
    });

    it("should throw for not implemented methods", () => {
        expect(() => vec.noneMatch([10.5])).toThrow("Not implemented yet.");
        expect(() => vec.filterNotEqual(10.5)).toThrow("Not implemented yet.");
        expect(() => vec.filterNotEqualSelected(10.5, new FlatSelectionVector([0]))).toThrow("Not implemented yet.");
    });
});

describe("FloatFlatVector", () => {
    function createVector(values: number[]): FloatFlatVector {
        return new FloatFlatVector("test", new Float32Array(values), values.length);
    }

    function createNullableVector(values: number[], nullBits: number): FloatFlatVector {
        const data = new Float32Array(values);
        const bitVector = new BitVector(new Uint8Array([nullBits]), values.length);
        return new FloatFlatVector("test", data, bitVector);
    }

    const vec = createVector([1.5, 2.5, 3.5, 4.5, 5.5]);

    it("should filter by value", () => {
        expect(getSelectionIndices(vec.filter(3.5))).toEqual([2]);
    });

    it("should match values", () => {
        expect(getSelectionIndices(vec.match([1.5, 5.5]))).toEqual([0, 4]);
    });

    it("should filter selected", () => {
        const selection = new FlatSelectionVector([0, 1, 2]);
        vec.filterSelected(2.5, selection);
        expect(getSelectionIndices(selection)).toEqual([1]);
    });

    it("should match selected", () => {
        const selection = new FlatSelectionVector([1, 2, 3]);
        vec.matchSelected([2.5, 4.5], selection);
        expect(getSelectionIndices(selection)).toEqual([1, 3]);
    });

    it("should filter >= and <= thresholds", () => {
        expect(getSelectionIndices(vec.greaterThanOrEqualTo(3.5))).toEqual([2, 3, 4]);
        expect(getSelectionIndices(vec.smallerThanOrEqualTo(3.5))).toEqual([0, 1, 2]);
    });

    it("should handle selected comparisons", () => {
        const sel1 = new FlatSelectionVector([1, 2, 3, 4]);
        vec.greaterThanOrEqualToSelected(3.5, sel1);
        expect(getSelectionIndices(sel1)).toEqual([2, 3, 4]);

        const sel2 = new FlatSelectionVector([0, 1, 2, 3]);
        vec.smallerThanOrEqualToSelected(2.5, sel2);
        expect(getSelectionIndices(sel2)).toEqual([0, 1]);
    });

    it("should handle nullability", () => {
        const nullVec = createNullableVector([1.5, 2.5, 3.5, 4.5], 0b0111);
        expect(getSelectionIndices(nullVec.filter(4.5))).toEqual([]);
        expect(getSelectionIndices(nullVec.greaterThanOrEqualTo(3.5))).toEqual([2]);
    });

    it("should throw for not implemented methods", () => {
        expect(() => vec.noneMatch([1.5])).toThrow("Not implemented yet.");
        expect(() => vec.filterNotEqual(1.5)).toThrow("Not implemented yet.");
    });
});
