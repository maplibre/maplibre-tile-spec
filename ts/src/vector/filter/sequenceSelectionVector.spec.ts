import {describe, it, expect} from "vitest";
import {SequenceSelectionVector} from "./sequenceSelectionVector";

describe("sequenceSelectionVector", () => {
    describe("getIndex Test", () => {
        it("Should return sequential values starting from 0", () => {
            const vector = new SequenceSelectionVector(0, 1, 5);
            expect(vector.getIndex(0)).toBe(0);
            expect(vector.getIndex(2)).toBe(2);
            expect(vector.getIndex(4)).toBe(4);
        });

        it("Should calculate values with custom base and delta", () => {
            const vector = new SequenceSelectionVector(10, 5, 5);
            expect(vector.getIndex(0)).toBe(10);
            expect(vector.getIndex(1)).toBe(15);
            expect(vector.getIndex(2)).toBe(20);
        });

        it("Should calculate values with negative delta", () => {
            const vector = new SequenceSelectionVector(100, -10, 5);
            expect(vector.getIndex(0)).toBe(100);
            expect(vector.getIndex(1)).toBe(90);
            expect(vector.getIndex(2)).toBe(80);
        });

        it("Should throw RangeError for out of bounds indices", () => {
            const vector = new SequenceSelectionVector(0, 1, 5);
            expect(() => vector.getIndex(80)).toThrowError("Index out of bounds");
            expect(() => vector.getIndex(-36)).toThrowError("Index out of bounds");
        });
    });

    describe("setIndex Test", () => {
        it("Should update value at specified index", () => {
            const vector = new SequenceSelectionVector(0, 1, 5);
            vector.setIndex(0, 25);
            vector.setIndex(2, -48);
            vector.setIndex(3, 1000000000000001);

            expect(vector.getIndex(0)).toBe(25);
            expect(vector.getIndex(2)).toBe(-48);
            expect(vector.getIndex(3)).toBe(1000000000000001);
        });

        it("Should throw RangeError for out of bounds indices", () => {
            const vector = new SequenceSelectionVector(0, 1, 5);
            expect(() => vector.setIndex(-1, 0)).toThrowError("Index out of bounds");
            expect(() => vector.setIndex(25, 52)).toThrowError("Index out of bounds");
        });
    });

    describe("limit Test", () => {
        it("Should initialize limit to size", () => {
            const vector = new SequenceSelectionVector(0, 1, 5);
            expect(vector.limit).toBe(5);

            const emptyVector = new SequenceSelectionVector(0, 1, 0);
            expect(emptyVector.limit).toBe(0);
        });

        it("Should update limit independently of capacity", () => {
            const vector = new SequenceSelectionVector(0, 1, 10);
            expect(vector.capacity).toBe(10);

            vector.setLimit(3);
            expect(vector.limit).toBe(3);
            expect(vector.capacity).toBe(10);

            vector.setLimit(8);
            expect(vector.limit).toBe(8);
            expect(vector.capacity).toBe(10);
        });

        it("Should throw RangeError for negative limit", () => {
            const vector = new SequenceSelectionVector(0, 1, 5);
            expect(() => vector.setLimit(-1)).toThrowError("Limit out of bounds");
            expect(() => vector.setLimit(100)).toThrowError("Limit out of bounds");
        });

        it("Should allow setting limit to 0", () => {
            const vector = new SequenceSelectionVector(0, 1, 5);
            vector.setLimit(0);
            expect(vector.limit).toBe(0);
        });
    });

    describe("selectionValues Test", () => {
        it("Should return array with sequential values", () => {
            const vector = new SequenceSelectionVector(0, 1, 5);
            const values = vector.selectionValues();
            expect(values).toStrictEqual([0, 1, 2, 3, 4]);
        });

        it("Should return empty array for empty vector", () => {
            const vector = new SequenceSelectionVector(0, 1, 0);
            expect(vector.selectionValues()).toStrictEqual([]);
        });

        it("Should return array with custom base and delta", () => {
            const vector = new SequenceSelectionVector(10, 5, 5);
            expect(vector.selectionValues()).toStrictEqual([10, 15, 20, 25, 30]);
        });

        it("Should return array with negative delta", () => {
            const vector = new SequenceSelectionVector(100, -10, 5);
            expect(vector.selectionValues()).toStrictEqual([100, 90, 80, 70, 60]);
        });

        it("Should reflect modified values", () => {
            const vector = new SequenceSelectionVector(0, 1, 3);
            vector.setIndex(2, 999);
            const values = vector.selectionValues();
            expect(values).toStrictEqual([0, 1, 999]);
        });
    });

    describe("capacity Test", () => {
        it("Should return capacity equal to size", () => {
            const vector = new SequenceSelectionVector(0, 1, 5);
            expect(vector.capacity).toBe(5);
        });

        it("Should return 0 for empty vector", () => {
            const vector = new SequenceSelectionVector(0, 1, 0);
            expect(vector.capacity).toBe(0);
        });

        it("Should remain constant when limit changes", () => {
            const vector = new SequenceSelectionVector(10, 5, 50);
            expect(vector.capacity).toBe(50);

            vector.setLimit(25);
            expect(vector.capacity).toBe(50);
        });
    });
});
