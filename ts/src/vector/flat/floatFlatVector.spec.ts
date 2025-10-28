import { describe, it, expect } from "vitest";
import { FloatFlatVector } from "./floatFlatVector";
import BitVector from "./bitVector";
import { FlatSelectionVector } from "../filter/flatSelectionVector";

function getSelectionIndices(selection: { selectionValues: () => number[]; limit: number }): number[] {
    return selection.selectionValues().slice(0, selection.limit);
}

function createVector(values: number[]): FloatFlatVector {
    return new FloatFlatVector("test", new Float32Array(values), values.length);
}

function createNullableVector(values: number[], nullBits: number): FloatFlatVector {
    const data = new Float32Array(values);
    const bitVector = new BitVector(new Uint8Array([nullBits]), values.length);
    return new FloatFlatVector("test", data, bitVector);
}

const vec = createVector([1.5, 2.5, 3.5, 4.5, 5.5]);

describe("FloatFlatVector", () => {
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
