import {beforeEach, describe, it, expect} from "vitest";
import {FlatSelectionVector} from "./flatSelectionVector";

let vector: number[];
let FSVector: FlatSelectionVector;

beforeEach(() => {
    vector = [0, 1, 999999999999, -28, 36];
    FSVector = new FlatSelectionVector(vector);
});

describe("flatSelectionVector", () => {
    describe("getIndex Test", () => {
        it("Should return value from Index", () => {
            expect(FSVector.getIndex(0)).toBe(0);
            expect(FSVector.getIndex(2)).toBe(999999999999);
            expect(FSVector.getIndex(3)).toBe(-28);
        });
        it("Should return Index out of bounds", () => {
            expect(() => FSVector.getIndex(80)).toThrowError("Index out of bounds");
            expect(() => FSVector.getIndex(-36)).toThrowError("Index out of bounds")
        });
    });
    describe("setIndex Test", () => {
        it("Should set value on Index", () => {
            FSVector.setIndex(0, 25);
            FSVector.setIndex(2, -48);
            FSVector.setIndex(3, 1000000000000001);

            expect(FSVector.getIndex(0)).toBe(25);
            expect(FSVector.getIndex(2)).toBe(-48);
            expect(FSVector.getIndex(3)).toBe(1000000000000001);
        });
        it("Should return Index out of bounds", () => {
            expect(() => FSVector.setIndex(-1, 0)).toThrowError("Index out of bounds");
            expect(() => FSVector.setIndex(25, 52)).toThrowError("Index out of bounds");
        })
    });
    describe("setLimit Test", () => {
        it("Should set limit", () => {
            FSVector.setLimit(250);
            expect(FSVector.limit).toBe(250);
            FSVector.setLimit(0);
            expect(FSVector.limit).toBe(0);
            FSVector.setLimit(-125);
            expect(FSVector.limit).toBe(-125);
        })
    });
    describe("selectionValues Test", () => {
        it("Should return selectionVector", () => {
            expect(FSVector.selectionValues()).toBe(vector)

            const EmptyFSVector = new FlatSelectionVector([]);
            expect(EmptyFSVector.selectionValues()).toStrictEqual([])
        });
    });
    describe("get capacity Test", () => {
        it("Should return capacity", () => {
            expect(FSVector.capacity).toBe(vector.length);

            const EmptyFSVector = new FlatSelectionVector([]);
            expect(EmptyFSVector.capacity).toBe(0)
        });
    });
    describe("get limit Test", () => {
        it("Should return limit", () => {
            expect(FSVector.limit).toBe(vector.length);

            const EmptyFSVector = new FlatSelectionVector([]);
            expect(EmptyFSVector.limit).toBe(0);

            const LimitFSVector = new FlatSelectionVector([], 52);
            expect(LimitFSVector.limit).toBe(52);
        });
    })
})
