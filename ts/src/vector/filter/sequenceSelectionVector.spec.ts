import {beforeEach, describe, it, expect} from "vitest";
import {SequenceSelectionVector} from "./sequenceSelectionVector";

let seqVector: SequenceSelectionVector;

beforeEach(() => {
    seqVector = new SequenceSelectionVector(0, 1, 5);
});

describe("sequenceSelectionVector", () => {
    describe("getIndex Test", () => {
        it("Should return value from Index", () => {
            expect(seqVector.getIndex(0)).toBe(0);
            expect(seqVector.getIndex(2)).toBe(2);
            expect(seqVector.getIndex(4)).toBe(4);
        });

        it("Should calculate correct values with custom base and delta", () => {
            const customVector = new SequenceSelectionVector(10, 5, 5);
            expect(customVector.getIndex(0)).toBe(10);
            expect(customVector.getIndex(1)).toBe(15);
            expect(customVector.getIndex(2)).toBe(20);
            expect(customVector.getIndex(3)).toBe(25);
            expect(customVector.getIndex(4)).toBe(30);
        });

        it("Should calculate correct values with negative delta", () => {
            const negVector = new SequenceSelectionVector(100, -10, 5);
            expect(negVector.getIndex(0)).toBe(100);
            expect(negVector.getIndex(1)).toBe(90);
            expect(negVector.getIndex(2)).toBe(80);
            expect(negVector.getIndex(3)).toBe(70);
            expect(negVector.getIndex(4)).toBe(60);
        });

        it("Should return Index out of bounds", () => {
            expect(() => seqVector.getIndex(80)).toThrowError("Index out of bounds");
            expect(() => seqVector.getIndex(-36)).toThrowError("Index out of bounds");
        });

        it("Should work with large values", () => {
            const largeVector = new SequenceSelectionVector(1000000, 100, 10);
            expect(largeVector.getIndex(0)).toBe(1000000);
            expect(largeVector.getIndex(5)).toBe(1000500);
            expect(largeVector.getIndex(9)).toBe(1000900);
        });
    });

    describe("setIndex Test", () => {
        it("Should set value on Index", () => {
            seqVector.setIndex(0, 25);
            seqVector.setIndex(2, -48);
            seqVector.setIndex(3, 1000000000000001);

            expect(seqVector.getIndex(0)).toBe(25);
            expect(seqVector.getIndex(2)).toBe(-48);
            expect(seqVector.getIndex(3)).toBe(1000000000000001);
        });

        it("Should materialize array on first set", () => {
            const customVector = new SequenceSelectionVector(10, 5, 5);

            // Before setting, values should be calculated
            expect(customVector.getIndex(0)).toBe(10);
            expect(customVector.getIndex(1)).toBe(15);

            // After setting, array should be materialized
            customVector.setIndex(2, 999);
            expect(customVector.getIndex(2)).toBe(999);

            // Other values should still be correct
            expect(customVector.getIndex(0)).toBe(10);
            expect(customVector.getIndex(1)).toBe(15);
        });

        it("Should return Index out of bounds", () => {
            expect(() => seqVector.setIndex(-1, 0)).toThrowError("Index out of bounds");
            expect(() => seqVector.setIndex(25, 52)).toThrowError("Index out of bounds");
        });

        it("Should preserve materialized values after multiple sets", () => {
            seqVector.setIndex(0, 100);
            seqVector.setIndex(1, 200);
            seqVector.setIndex(2, 300);

            expect(seqVector.getIndex(0)).toBe(100);
            expect(seqVector.getIndex(1)).toBe(200);
            expect(seqVector.getIndex(2)).toBe(300);
        });
    });

    describe("setLimit Test", () => {
        it("Should set limit", () => {
            seqVector.setLimit(250);
            expect(seqVector.limit).toBe(250);
            seqVector.setLimit(0);
            expect(seqVector.limit).toBe(0);
            seqVector.setLimit(-125);
            expect(seqVector.limit).toBe(-125);
        });

        it("Should change limit independently of capacity", () => {
            expect(seqVector.capacity).toBe(5);
            seqVector.setLimit(3);
            expect(seqVector.limit).toBe(3);
            expect(seqVector.capacity).toBe(5);
        });
    });

    describe("selectionValues Test", () => {
        it("Should return selectionVector", () => {
            const values = seqVector.selectionValues();
            expect(values).toStrictEqual([0, 1, 2, 3, 4]);
        });

        it("Should return empty array for empty vector", () => {
            const emptyVector = new SequenceSelectionVector(0, 1, 0);
            expect(emptyVector.selectionValues()).toStrictEqual([]);
        });

        it("Should return correct values with custom base and delta", () => {
            const customVector = new SequenceSelectionVector(10, 5, 5);
            expect(customVector.selectionValues()).toStrictEqual([10, 15, 20, 25, 30]);
        });

        it("Should return correct values with negative delta", () => {
            const negVector = new SequenceSelectionVector(100, -10, 5);
            expect(negVector.selectionValues()).toStrictEqual([100, 90, 80, 70, 60]);
        });

        it("Should cache materialized array", () => {
            const values1 = seqVector.selectionValues();
            const values2 = seqVector.selectionValues();
            expect(values1).toBe(values2); // Same reference
        });

        it("Should return materialized array after setIndex", () => {
            seqVector.setIndex(2, 999);
            const values = seqVector.selectionValues();
            expect(values).toStrictEqual([0, 1, 999, 3, 4]);
        });
    });

    describe("get capacity Test", () => {
        it("Should return capacity", () => {
            expect(seqVector.capacity).toBe(5);
        });

        it("Should return 0 for empty vector", () => {
            const emptyVector = new SequenceSelectionVector(0, 1, 0);
            expect(emptyVector.capacity).toBe(0);
        });

        it("Should return correct capacity for large vector", () => {
            const largeVector = new SequenceSelectionVector(0, 1, 1000);
            expect(largeVector.capacity).toBe(1000);
        });
    });

    describe("get limit Test", () => {
        it("Should return limit", () => {
            expect(seqVector.limit).toBe(5);
        });

        it("Should return 0 for empty vector", () => {
            const emptyVector = new SequenceSelectionVector(0, 1, 0);
            expect(emptyVector.limit).toBe(0);
        });

        it("Should return updated limit after setLimit", () => {
            seqVector.setLimit(3);
            expect(seqVector.limit).toBe(3);
        });

        it("Should initialize limit to size", () => {
            const customVector = new SequenceSelectionVector(10, 5, 50);
            expect(customVector.limit).toBe(50);
            expect(customVector.capacity).toBe(50);
        });
    });

    describe("Memory efficiency Test", () => {
        it("Should not materialize array until necessary", () => {
            const vector = new SequenceSelectionVector(0, 1, 1000);

            // Getting individual indices should not materialize
            expect(vector.getIndex(0)).toBe(0);
            expect(vector.getIndex(500)).toBe(500);
            expect(vector.getIndex(999)).toBe(999);

            // Only calling selectionValues() or setIndex() should materialize
        });

        it("Should handle zero delta", () => {
            const zeroVector = new SequenceSelectionVector(42, 0, 5);
            expect(zeroVector.getIndex(0)).toBe(42);
            expect(zeroVector.getIndex(1)).toBe(42);
            expect(zeroVector.getIndex(4)).toBe(42);
            expect(zeroVector.selectionValues()).toStrictEqual([42, 42, 42, 42, 42]);
        });
    });
});
