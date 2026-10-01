import { describe, it, expect } from "vitest";
import {
    createSelectionVector,
    createNullableSelectionVector,
    updateNullableSelectionVector,
} from "./selectionVectorUtils";
import { FlatSelectionVector } from "./flatSelectionVector";
import { SequenceSelectionVector } from "./sequenceSelectionVector";
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
        it("Should return all indices when nullabilityBuffer is not provided", () => {
            const sv = createNullableSelectionVector(5);
            expect(sv).toBeInstanceOf(FlatSelectionVector);
            expect(sv.limit).toBe(5);
        });

        it("Should return empty vector when size is 0 and nullabilityBuffer is not provided", () => {
            const sv = createNullableSelectionVector(0);
            expect(sv).toBeInstanceOf(FlatSelectionVector);
            expect(sv.limit).toBe(0);
        });

        it("Should return empty vector when size is 0 with BitVector provided", () => {
            const buffer = new Uint8Array([0b11111111]);
            const bitVector = new BitVector(buffer, 8);
            const sv = createNullableSelectionVector(0, bitVector);
            expect(sv).toBeInstanceOf(FlatSelectionVector);
            expect(sv.limit).toBe(0);
        });

        it("Should create FlatSelectionVector with only set bits", () => {
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
        describe("with FlatSelectionVector", () => {
            it("Should return new instance when filtering with BitVector", () => {
                const selectionVector = new FlatSelectionVector([0, 1, 2, 3, 4, 5, 6, 7]);
                const buffer = new Uint8Array([0b00001011]);
                const bitVector = new BitVector(buffer, 8);
                const result = updateNullableSelectionVector(selectionVector, bitVector);

                expect(result).toBeInstanceOf(FlatSelectionVector);
                expect(result.limit).toBe(3);
                expect(result).not.toBe(selectionVector);
            });

            it("Should return same instance when BitVector is null", () => {
                const selectionVector = new FlatSelectionVector([0, 1, 2, 3, 4, 5, 6, 7]);
                const result = updateNullableSelectionVector(selectionVector, null);
                expect(result).toStrictEqual(selectionVector);
            });

            it("Should return all indices when nullabilityBuffer is undefined", () => {
                const selectionVector = new FlatSelectionVector([0, 2, 4, 6]);
                const result = updateNullableSelectionVector(selectionVector, undefined);
                expect(result).toBeInstanceOf(FlatSelectionVector);
                expect(result.limit).toBe(4);
            });

            it("Should keep all indices when all bits are set", () => {
                const selectionVector = new FlatSelectionVector([0, 2, 4, 6]);
                const buffer = new Uint8Array([0b01010101]);
                const bitVector = new BitVector(buffer, 8);
                const result = updateNullableSelectionVector(selectionVector, bitVector);
                expect(result).toBeInstanceOf(FlatSelectionVector);
                expect(result.limit).toBe(4);
            });

            it("Should filter out null indices from selection vector", () => {
                const selectionVector = new FlatSelectionVector([0, 2, 4, 6]);
                const buffer = new Uint8Array([0b01000101]); // bits at 0, 2, 6
                const bitVector = new BitVector(buffer, 8);
                const result = updateNullableSelectionVector(selectionVector, bitVector);
                expect(result).toBeInstanceOf(FlatSelectionVector);
                expect(result.limit).toBe(3);
            });

            it("Should return empty vector when all selected indices are null", () => {
                const selectionVector = new FlatSelectionVector([1, 3, 5]);
                const buffer = new Uint8Array([0b01010101]); // bits at 0, 2, 4, 6 (not 1, 3, 5)
                const bitVector = new BitVector(buffer, 8);
                const result = updateNullableSelectionVector(selectionVector, bitVector);
                expect(result).toBeInstanceOf(FlatSelectionVector);
                expect(result.limit).toBe(0);
            });

            it("Should handle empty FlatSelectionVector", () => {
                const selectionVector = new FlatSelectionVector([]);
                const buffer = new Uint8Array([0b11111111]);
                const bitVector = new BitVector(buffer, 8);
                const result = updateNullableSelectionVector(selectionVector, bitVector);
                expect(result).toBeInstanceOf(FlatSelectionVector);
                expect(result.limit).toBe(0);
            });

            it("Should filter large index values from selection vector", () => {
                const selectionVector = new FlatSelectionVector([0, 8, 16]);
                const buffer = new Uint8Array([0b00000001, 0b00000000, 0b00000000]);
                const bitVector = new BitVector(buffer, 24);
                const result = updateNullableSelectionVector(selectionVector, bitVector);
                expect(result).toBeInstanceOf(FlatSelectionVector);
                expect(result.limit).toBe(1);
            });
        });

        describe("with SequenceSelectionVector", () => {
            it("Should filter SequenceSelectionVector with all bits set", () => {
                const selectionVector = new SequenceSelectionVector(0, 2, 4); // [0, 2, 4, 6]
                const buffer = new Uint8Array([0b01010101]);
                const bitVector = new BitVector(buffer, 8);
                const result = updateNullableSelectionVector(selectionVector, bitVector);
                expect(result).toBeInstanceOf(FlatSelectionVector);
                expect(result.limit).toBe(4);
            });

            it("Should partially filter SequenceSelectionVector", () => {
                const selectionVector = new SequenceSelectionVector(0, 2, 4); // [0, 2, 4, 6]
                const buffer = new Uint8Array([0b00010001]); // bits at 0, 4
                const bitVector = new BitVector(buffer, 8);
                const result = updateNullableSelectionVector(selectionVector, bitVector);
                expect(result).toBeInstanceOf(FlatSelectionVector);
                expect(result.limit).toBe(2);
            });

            it("Should preserve all SequenceSelectionVector values when nullabilityBuffer is undefined", () => {
                const selectionVector = new SequenceSelectionVector(1, 3, 3); // [1, 4, 7]
                const result = updateNullableSelectionVector(selectionVector, undefined);
                expect(result).toBeInstanceOf(FlatSelectionVector);
                expect(result.limit).toBe(3);
            });

            it("Should create FlatSelectionVector when BitVector is null", () => {
                const selectionVector = new SequenceSelectionVector(0, 2, 4); // [0, 2, 4, 6]
                const result = updateNullableSelectionVector(selectionVector, null);

                expect(result).toBeInstanceOf(FlatSelectionVector);
                expect(result.limit).toBe(4);
            });
        });
    });
});
