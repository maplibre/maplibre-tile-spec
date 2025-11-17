import {beforeEach, describe, it, expect} from "vitest";
import {FlatSelectionVector} from "./flatSelectionVector";

describe("flatSelectionVector", () => {
    let vector: number[];
    let fsVector: FlatSelectionVector;

    beforeEach(() => {
        vector = [0, 1, 999999999999, -28, 36];
        fsVector = new FlatSelectionVector(vector);
    });

    describe("getIndex Test", () => {
        it("Should return value from Index", () => {
            expect(fsVector.getIndex(0)).toBe(0);
            expect(fsVector.getIndex(2)).toBe(999999999999);
            expect(fsVector.getIndex(3)).toBe(-28);
        });
        it("Should return Index out of bounds", () => {
            expect(() => fsVector.getIndex(80)).toThrowError("Index out of bounds");
            expect(() => fsVector.getIndex(-36)).toThrowError("Index out of bounds")
        });
    });
    describe("setIndex Test", () => {
        it("Should set value on Index", () => {
            fsVector.setIndex(0, 25);
            fsVector.setIndex(2, -48);
            fsVector.setIndex(3, 1000000000000001);

            expect(fsVector.getIndex(0)).toBe(25);
            expect(fsVector.getIndex(2)).toBe(-48);
            expect(fsVector.getIndex(3)).toBe(1000000000000001);
        });
        it("Should return Index out of bounds", () => {
            expect(() => fsVector.setIndex(-1, 0)).toThrowError("Index out of bounds");
            expect(() => fsVector.setIndex(25, 52)).toThrowError("Index out of bounds");
        })
    });
    describe("setLimit Test", () => {
        it("Should set limit", () => {
            fsVector.setLimit(250);
            expect(fsVector.limit).toBe(250);
            fsVector.setLimit(0);
            expect(fsVector.limit).toBe(0);
            fsVector.setLimit(-125);
            expect(fsVector.limit).toBe(-125);
        })
    });
    describe("selectionValues Test", () => {
        it("Should return selectionVector", () => {
            expect(fsVector.selectionValues()).toBe(vector)

            const emptyFsVector = new FlatSelectionVector([]);
            expect(emptyFsVector.selectionValues()).toStrictEqual([])
        });
    });
    describe("get capacity Test", () => {
        it("Should return capacity", () => {
            expect(fsVector.capacity).toBe(vector.length);

            const emptyFsVector = new FlatSelectionVector([]);
            expect(emptyFsVector.capacity).toBe(0)
        });
    });
    describe("get limit Test", () => {
        it("Should return limit", () => {
            expect(fsVector.limit).toBe(vector.length);

            const emptyFsVector = new FlatSelectionVector([]);
            expect(emptyFsVector.limit).toBe(0);

            const limitFsVector = new FlatSelectionVector([], 52);
            expect(limitFsVector.limit).toBe(52);
        });
    })
})
