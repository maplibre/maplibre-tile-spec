import {describe, it, expect} from "vitest";
import {
    createSelectionVector,
    createNullableSelectionVector,
    updateNullableSelectionVector
} from "./selectionVectorUtils";
import {FlatSelectionVector} from "./flatSelectionVector";
import {SequenceSelectionVector} from "./sequenceSelectionVector";
import BitVector from "../flat/bitVector";

describe("selectionVectorUtils", () => {
    describe("createSelectionVector", () => {
        it("Should create a SequenceSelectionVector with given size", () => {
            const sv = createSelectionVector(5);
            expect(sv).toBeInstanceOf(SequenceSelectionVector);
            expect(sv.limit).toBe(5);
        });

        it("Should handle zero size", () => {
            const sv = createSelectionVector(0);
            expect(sv).toBeInstanceOf(SequenceSelectionVector);
            expect(sv.limit).toBe(0);
        });
    });

    describe("createNullableSelectionVector", () => {
        it("Should create FlatSelectionVector when filtering by BitVector", () => {
            const buffer = new Uint8Array([0b00001011]); // bits 0, 1, 3 are set
            const bitVector = new BitVector(buffer, 8);
            const sv = createNullableSelectionVector(8, bitVector);

            expect(sv).toBeInstanceOf(FlatSelectionVector);
            expect(sv.limit).toBe(3);
        });

        it("Should create empty vector when no bits are set", () => {
            const buffer = new Uint8Array([0b00000000]);
            const bitVector = new BitVector(buffer, 8);
            const sv = createNullableSelectionVector(8, bitVector);

            expect(sv).toBeInstanceOf(FlatSelectionVector);
            expect(sv.limit).toBe(0);
        });

        it("Should handle multiple bytes in BitVector", () => {
            const buffer = new Uint8Array([0b10101010, 0b01010101]);
            const bitVector = new BitVector(buffer, 16);
            const sv = createNullableSelectionVector(16, bitVector);

            expect(sv).toBeInstanceOf(FlatSelectionVector);
            expect(sv.limit).toBe(8);
        });
    });

    describe("updateNullableSelectionVector", () => {
        it("Should return FlatSelectionVector when filtering with BitVector", () => {
            const selectionVector = new FlatSelectionVector([0, 1, 2, 3, 4, 5, 6, 7]);
            const buffer = new Uint8Array([0b00001011]);
            const bitVector = new BitVector(buffer, 8);
            const result = updateNullableSelectionVector(selectionVector, bitVector);

            expect(result).toBeInstanceOf(FlatSelectionVector);
            expect(result.limit).toBe(3);
            expect(result).not.toBe(selectionVector); // Should be new instance
        });

        it("Should return same vector when BitVector is null", () => {
            const selectionVector = new FlatSelectionVector([0, 1, 2, 3, 4, 5, 6, 7]);
            const result = updateNullableSelectionVector(selectionVector, null);

            expect(result).toStrictEqual(selectionVector);
        });
    });
});
