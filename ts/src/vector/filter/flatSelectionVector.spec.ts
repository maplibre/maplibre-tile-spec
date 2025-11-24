import {describe, it, expect} from "vitest";
import {FlatSelectionVector} from "./flatSelectionVector";

describe("flatSelectionVector", () => {
    describe("Basic functionality", () => {
        it("Should store and retrieve indices", () => {
            const fsVector = new FlatSelectionVector(new Uint32Array([0, 1, 4294967295, 28, 36]));
            expect(fsVector.getIndex(0)).toBe(0);
            expect(fsVector.getIndex(2)).toBe(4294967295);
            expect(fsVector.getIndex(3)).toBe(28);

            fsVector.setIndex(2, 48);
            expect(fsVector.getIndex(2)).toBe(48);
        });

        it("Should throw RangeError for out of bounds access", () => {
            const fsVector = new FlatSelectionVector(new Uint32Array([0, 1, 2]));
            expect(() => fsVector.getIndex(10)).toThrowError("Index out of bounds");
            expect(() => fsVector.getIndex(-1)).toThrowError("Index out of bounds");
            expect(() => fsVector.setIndex(-1, 0)).toThrowError("Index out of bounds");
            expect(() => fsVector.setIndex(10, 0)).toThrowError("Index out of bounds");
        });
    });

    describe("Array wrapper behavior", () => {
        it("Should return reference to underlying array", () => {
            const vector = new Uint32Array([0, 1, 2, 3, 4]);
            const fsVector = new FlatSelectionVector(vector);
            expect(fsVector.selectionValues()).toStrictEqual(vector);
        });

        it("Should use array length as default limit and capacity", () => {
            const fsVector = new FlatSelectionVector(new Uint32Array([1, 2, 3, 4, 5]));
            expect(fsVector.limit).toBe(5);
            expect(fsVector.capacity).toBe(5);
        });

        it("Should allow custom limit independent of array length", () => {
            const fsVector = new FlatSelectionVector(new Uint32Array([1, 2, 3, 4, 5]), 3);
            expect(fsVector.limit).toBe(3);
            expect(fsVector.capacity).toBe(5);
        });
    });
    describe("set Limit Tests", () => {
       it("should set Limit", () => {
           const fsVector = new FlatSelectionVector(new Uint32Array([1, 2, 3, 4, 5]), 3);
           fsVector.setLimit(2);
           expect(fsVector.limit).toBe(2)
       });
        it("should throw out of bounds error", () => {
            const fsVector = new FlatSelectionVector(new Uint32Array([1, 2, 3, 4, 5]), 3);
            expect(() => fsVector.setLimit(-10)).toThrowError("Limit out of bounds");
            expect(() => fsVector.setLimit(10)).toThrowError("Limit out of bounds");
        })
    });
})
