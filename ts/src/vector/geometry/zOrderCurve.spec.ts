import { describe, it, expect } from "vitest";
import { decodeZOrderCurve } from "./zOrderCurve";

describe("decodeZOrderCurve", () => {
    it("should decode z-order curve", () => {
        const encodedValue = 38865244;
        const numBits = 13;
        const coordinateShift = 0;
        const expectedDecodedValue = { x: 3358, y: 4130 };

        expect(expectedDecodedValue).toEqual(decodeZOrderCurve(encodedValue, numBits, coordinateShift));
    });

    it("should decode the example value of wikipedia", () => {
        const encodedValue = 2479;
        const numBits = 6;
        const coordinateShift = 0;
        const expectedDecodedValue = { x: 19, y: 47 };

        expect(expectedDecodedValue).toEqual(decodeZOrderCurve(encodedValue, numBits, coordinateShift));
    });
});
