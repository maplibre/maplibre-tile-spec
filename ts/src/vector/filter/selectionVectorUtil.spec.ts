import {beforeEach, describe, it, expect} from "vitest";
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
    describe("createSelectionVector Test", () => {
        it("Should create a SequenceSelectionVector with the correct size", () => {
            const sv = createSelectionVector(5);

            expect(sv).toBeInstanceOf(SequenceSelectionVector);
            expect(sv.limit).toBe(5);
            expect(sv.capacity).toBe(5);
        });

        it("Should create a SequenceSelectionVector with sequential values", () => {
            const sv = createSelectionVector(5);

            expect(sv.getIndex(0)).toBe(0);
            expect(sv.getIndex(1)).toBe(1);
            expect(sv.getIndex(2)).toBe(2);
            expect(sv.getIndex(3)).toBe(3);
            expect(sv.getIndex(4)).toBe(4);
        });

        it("Should create an empty SequenceSelectionVector", () => {
            const sv = createSelectionVector(0);

            expect(sv.limit).toBe(0);
            expect(sv.capacity).toBe(0);
        });

        it("Should create a large SequenceSelectionVector", () => {
            const sv = createSelectionVector(1000);

            expect(sv.limit).toBe(1000);
            expect(sv.capacity).toBe(1000);
            expect(sv.getIndex(999)).toBe(999);
        });
    });

    describe("createNullableSelectionVector Test", () => {
        it("Should create a FlatSelectionVector with non-null indices", () => {
            const buffer = new Uint8Array([0b00001011]); // bits 0, 1, 3 are set
            const bitVector = new BitVector(buffer, 8);
            const sv = createNullableSelectionVector(8, bitVector);

            expect(sv).toBeInstanceOf(FlatSelectionVector);
            expect(sv.limit).toBe(3);
            expect(sv.getIndex(0)).toBe(0);
            expect(sv.getIndex(1)).toBe(1);
            expect(sv.getIndex(2)).toBe(3);
        });

        it("Should create an empty FlatSelectionVector when all bits are false", () => {
            const buffer = new Uint8Array([0b00000000]);
            const bitVector = new BitVector(buffer, 8);
            const sv = createNullableSelectionVector(8, bitVector);

            expect(sv.limit).toBe(0);
            expect(sv.capacity).toBe(0);
        });

        it("Should create a full FlatSelectionVector when all bits are true", () => {
            const buffer = new Uint8Array([0b11111111]);
            const bitVector = new BitVector(buffer, 8);
            const sv = createNullableSelectionVector(8, bitVector);

            expect(sv.limit).toBe(8);
            expect(sv.getIndex(0)).toBe(0);
            expect(sv.getIndex(7)).toBe(7);
        });

        it("Should handle multiple bytes in BitVector", () => {
            const buffer = new Uint8Array([0b10101010, 0b01010101]);
            const bitVector = new BitVector(buffer, 16);
            const sv = createNullableSelectionVector(16, bitVector);

            expect(sv.limit).toBe(8);
            const values = sv.selectionValues();
            expect(values).toContain(1);
            expect(values).toContain(3);
            expect(values).toContain(5);
            expect(values).toContain(7);
            expect(values).toContain(8);
            expect(values).toContain(10);
            expect(values).toContain(12);
            expect(values).toContain(14);
        });
    });

    describe("updateSelectionVector Test", () => {
        let selectionVector: FlatSelectionVector;

        beforeEach(() => {
            selectionVector = new FlatSelectionVector([0, 1, 2, 3, 4, 5, 6, 7]);
        });

        it("Should filter out indices where nullability is false", () => {
            const buffer = new Uint8Array([0b00001011]); // bits 0, 1, 3 are set
            const bitVector = new BitVector(buffer, 8);

            updateSelectionVector(selectionVector, bitVector);

            expect(selectionVector.limit).toBe(3);
            expect(selectionVector.getIndex(0)).toBe(0);
            expect(selectionVector.getIndex(1)).toBe(1);
            expect(selectionVector.getIndex(2)).toBe(3);
        });

        it("Should keep all indices when nullability buffer is not provided", () => {
            updateSelectionVector(selectionVector, null);

            expect(selectionVector.limit).toBe(8);
            expect(selectionVector.getIndex(0)).toBe(0);
            expect(selectionVector.getIndex(7)).toBe(7);
        });

        it("Should set limit to 0 when all bits are false", () => {
            const buffer = new Uint8Array([0b00000000]);
            const bitVector = new BitVector(buffer, 8);

            updateSelectionVector(selectionVector, bitVector);

            expect(selectionVector.limit).toBe(0);
        });

        it("Should keep all indices when all bits are true", () => {
            const buffer = new Uint8Array([0b11111111]);
            const bitVector = new BitVector(buffer, 8);

            updateSelectionVector(selectionVector, bitVector);

            expect(selectionVector.limit).toBe(8);
            expect(selectionVector.getIndex(0)).toBe(0);
            expect(selectionVector.getIndex(7)).toBe(7);
        });

        it("Should handle SequenceSelectionVector", () => {
            const seqVector = new SequenceSelectionVector(0, 1, 8);
            const buffer = new Uint8Array([0b00001111]); // bits 0, 1, 2, 3 are set
            const bitVector = new BitVector(buffer, 8);

            updateSelectionVector(seqVector, bitVector);

            expect(seqVector.limit).toBe(4);
        });
    });

    describe("updateNullableSelectionVector Test", () => {
        let selectionVector: FlatSelectionVector;

        beforeEach(() => {
            selectionVector = new FlatSelectionVector([0, 1, 2, 3, 4, 5, 6, 7]);
        });

        it("Should filter out indices where nullability is false", () => {
            const buffer = new Uint8Array([0b00001011]); // bits 0, 1, 3 are set
            const bitVector = new BitVector(buffer, 8);

            updateNullableSelectionVector(selectionVector, bitVector);

            expect(selectionVector.limit).toBe(3);
            expect(selectionVector.getIndex(0)).toBe(0);
            expect(selectionVector.getIndex(1)).toBe(1);
            expect(selectionVector.getIndex(2)).toBe(3);
        });

        it("Should keep all indices when nullability buffer is not provided", () => {
            updateNullableSelectionVector(selectionVector, null);

            expect(selectionVector.limit).toBe(8);
            expect(selectionVector.getIndex(0)).toBe(0);
            expect(selectionVector.getIndex(7)).toBe(7);
        });

        it("Should set limit to 0 when all bits are false", () => {
            const buffer = new Uint8Array([0b00000000]);
            const bitVector = new BitVector(buffer, 8);

            updateNullableSelectionVector(selectionVector, bitVector);

            expect(selectionVector.limit).toBe(0);
        });

        it("Should keep all indices when all bits are true", () => {
            const buffer = new Uint8Array([0b11111111]);
            const bitVector = new BitVector(buffer, 8);

            updateNullableSelectionVector(selectionVector, bitVector);

            expect(selectionVector.limit).toBe(8);
            expect(selectionVector.getIndex(0)).toBe(0);
            expect(selectionVector.getIndex(7)).toBe(7);
        });

        it("Should handle SequenceSelectionVector", () => {
            const seqVector = new SequenceSelectionVector(0, 1, 8);
            const buffer = new Uint8Array([0b00001111]); // bits 0, 1, 2, 3 are set
            const bitVector = new BitVector(buffer, 8);

            updateNullableSelectionVector(seqVector, bitVector);

            expect(seqVector.limit).toBe(4);
        });

        it("Should handle complex filtering pattern", () => {
            const buffer = new Uint8Array([0b10101010]);
            const bitVector = new BitVector(buffer, 8);

            updateNullableSelectionVector(selectionVector, bitVector);

            expect(selectionVector.limit).toBe(4);
            expect(selectionVector.getIndex(0)).toBe(1);
            expect(selectionVector.getIndex(1)).toBe(3);
            expect(selectionVector.getIndex(2)).toBe(5);
            expect(selectionVector.getIndex(3)).toBe(7);
        });
    });
});
