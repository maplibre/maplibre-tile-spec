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
            expect(vector.getIndex(3)).toBe(25);
            expect(vector.getIndex(4)).toBe(30);
        });

        it("Should calculate values with negative delta", () => {
            const vector = new SequenceSelectionVector(100, -10, 5);
            expect(vector.getIndex(0)).toBe(100);
            expect(vector.getIndex(1)).toBe(90);
            expect(vector.getIndex(2)).toBe(80);
            expect(vector.getIndex(3)).toBe(70);
            expect(vector.getIndex(4)).toBe(60);
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

    describe("setLimit Test", () => {
        it("Should update limit", () => {
            const vector = new SequenceSelectionVector(0, 1, 5);
            vector.setLimit(250);
            expect(vector.limit).toBe(250);
            vector.setLimit(0);
            expect(vector.limit).toBe(0);
            vector.setLimit(-125);
            expect(vector.limit).toBe(-125);
        });

        it("Should change limit independently of capacity", () => {
            const vector = new SequenceSelectionVector(0, 1, 5);
            expect(vector.capacity).toBe(5);
            vector.setLimit(3);
            expect(vector.limit).toBe(3);
            expect(vector.capacity).toBe(5);
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
            const vector = new SequenceSelectionVector(0, 1, 5);
            vector.setIndex(2, 999);
            const values = vector.selectionValues();
            expect(values).toStrictEqual([0, 1, 999, 3, 4]);
        });
    });

    describe("get capacity Test", () => {
        it("Should return capacity", () => {
            const vector = new SequenceSelectionVector(0, 1, 5);
            expect(vector.capacity).toBe(5);
        });

        it("Should return 0 for empty vector", () => {
            const vector = new SequenceSelectionVector(0, 1, 0);
            expect(vector.capacity).toBe(0);
        });
    });

    describe("get limit Test", () => {
        it("Should return initial limit equal to size", () => {
            const vector = new SequenceSelectionVector(0, 1, 5);
            expect(vector.limit).toBe(5);
        });

        it("Should return 0 for empty vector", () => {
            const vector = new SequenceSelectionVector(0, 1, 0);
            expect(vector.limit).toBe(0);
        });

        it("Should return updated limit after setLimit", () => {
            const vector = new SequenceSelectionVector(0, 1, 5);
            vector.setLimit(3);
            expect(vector.limit).toBe(3);
        });

        it("Should initialize limit to size", () => {
            const vector = new SequenceSelectionVector(10, 5, 50);
            expect(vector.limit).toBe(50);
            expect(vector.capacity).toBe(50);
        });
    });
});
