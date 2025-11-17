import {describe, it, expect} from "vitest";
import {
    createSelectionVector,
    createNullableSelectionVector,
    updateSelectionVector,
    updateNullableSelectionVector
} from "./selectionVectorUtils";
import {FlatSelectionVector} from "./flatSelectionVector";
import {SequenceSelectionVector} from "./sequenceSelectionVector";
import BitVector from "../flat/bitVector";

describe("selectionVectorUtils", () => {
    describe("createSelectionVector", () => {
        it("Should create a SequenceSelectionVector with correct size", () => {
            const sv = createSelectionVector(5);
            expect(sv).toBeInstanceOf(SequenceSelectionVector);
            expect(sv.limit).toBe(5);
            expect(sv.capacity).toBe(5);
        });

        it("Should create an empty SequenceSelectionVector", () => {
            const sv = createSelectionVector(0);
            expect(sv.limit).toBe(0);
            expect(sv.capacity).toBe(0);
        });
    });

    describe("createNullableSelectionVector", () => {
        it("Should create FlatSelectionVector with only non-null indices", () => {
            const buffer = new Uint8Array([0b00001011]); // bits 0, 1, 3 are set
            const bitVector = new BitVector(buffer, 8);
            const sv = createNullableSelectionVector(8, bitVector);

            expect(sv).toBeInstanceOf(FlatSelectionVector);
            expect(sv.limit).toBe(3);
            expect(sv.selectionValues()).toStrictEqual([0, 1, 3]);
        });

        it("Should create empty vector when all bits are false", () => {
            const buffer = new Uint8Array([0b00000000]);
            const bitVector = new BitVector(buffer, 8);
            const sv = createNullableSelectionVector(8, bitVector);

            expect(sv.limit).toBe(0);
            expect(sv.capacity).toBe(0);
        });

        it("Should create full vector when all bits are true", () => {
            const buffer = new Uint8Array([0b11111111]);
            const bitVector = new BitVector(buffer, 8);
            const sv = createNullableSelectionVector(8, bitVector);

            expect(sv.limit).toBe(8);
            expect(sv.selectionValues()).toStrictEqual([0, 1, 2, 3, 4, 5, 6, 7]);
        });

        it("Should handle multiple bytes in BitVector", () => {
            const buffer = new Uint8Array([0b10101010, 0b01010101]);
            const bitVector = new BitVector(buffer, 16);
            const sv = createNullableSelectionVector(16, bitVector);

            expect(sv.limit).toBe(8);
            expect(sv.selectionValues()).toStrictEqual([1, 3, 5, 7, 8, 10, 12, 14]);
        });
    });

    describe("updateSelectionVector", () => {
        it("Should create new vector with filtered indices", () => {
            const selectionVector = new FlatSelectionVector([0, 1, 2, 3, 4, 5, 6, 7]);
            const buffer = new Uint8Array([0b00001011]); // bits 0, 1, 3 are set
            const bitVector = new BitVector(buffer, 8);

            const result = updateSelectionVector(selectionVector, bitVector);

            expect(result).toBeInstanceOf(FlatSelectionVector);
            expect(result.limit).toBe(3);
            expect(result.selectionValues()).toStrictEqual([0, 1, 3]);

            // Original should remain unchanged
            expect(selectionVector.limit).toBe(8);
        });

        it("Should keep all indices when nullability buffer is null", () => {
            const selectionVector = new FlatSelectionVector([0, 1, 2, 3, 4, 5, 6, 7]);

            const result = updateSelectionVector(selectionVector, null);

            expect(result.limit).toBe(8);
            expect(result.selectionValues()).toStrictEqual([0, 1, 2, 3, 4, 5, 6, 7]);
        });

        it("Should create empty vector when all bits are false", () => {
            const selectionVector = new FlatSelectionVector([0, 1, 2, 3, 4, 5, 6, 7]);
            const buffer = new Uint8Array([0b00000000]);
            const bitVector = new BitVector(buffer, 8);

            const result = updateSelectionVector(selectionVector, bitVector);

            expect(result.limit).toBe(0);
            expect(result.selectionValues()).toStrictEqual([]);
        });

        it("Should work with SequenceSelectionVector", () => {
            const seqVector = new SequenceSelectionVector(0, 1, 8);
            const buffer = new Uint8Array([0b00001111]); // bits 0, 1, 2, 3 are set
            const bitVector = new BitVector(buffer, 8);

            const result = updateSelectionVector(seqVector, bitVector);

            expect(result).toBeInstanceOf(FlatSelectionVector);
            expect(result.limit).toBe(4);
            expect(result.selectionValues()).toStrictEqual([0, 1, 2, 3]);
        });
    });

    describe("updateNullableSelectionVector", () => {
        it("Should create new vector with filtered indices", () => {
            const selectionVector = new FlatSelectionVector([0, 1, 2, 3, 4, 5, 6, 7]);
            const buffer = new Uint8Array([0b00001011]); // bits 0, 1, 3 are set
            const bitVector = new BitVector(buffer, 8);

            const result = updateNullableSelectionVector(selectionVector, bitVector);

            expect(result).toBeInstanceOf(FlatSelectionVector);
            expect(result.limit).toBe(3);
            expect(result.selectionValues()).toStrictEqual([0, 1, 3]);

            // Original should remain unchanged
            expect(selectionVector.limit).toBe(8);
        });

        it("Should keep all indices when nullability buffer is null", () => {
            const selectionVector = new FlatSelectionVector([0, 1, 2, 3, 4, 5, 6, 7]);

            const result = updateNullableSelectionVector(selectionVector, null);

            expect(result.limit).toBe(8);
            expect(result.selectionValues()).toStrictEqual([0, 1, 2, 3, 4, 5, 6, 7]);
        });

        it("Should handle complex filtering pattern", () => {
            const selectionVector = new FlatSelectionVector([0, 1, 2, 3, 4, 5, 6, 7]);
            const buffer = new Uint8Array([0b10101010]);
            const bitVector = new BitVector(buffer, 8);

            const result = updateNullableSelectionVector(selectionVector, bitVector);

            expect(result.limit).toBe(4);
            expect(result.selectionValues()).toStrictEqual([1, 3, 5, 7]);
        });
    });
});
