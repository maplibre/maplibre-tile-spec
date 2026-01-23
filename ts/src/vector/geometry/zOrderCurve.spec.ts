import { describe, it, expect } from "vitest";
import { decodeZOrderCurve } from "./zOrderCurve";
import { encodeZOrderCurve } from "../../encoding/geometryEncoder";

describe("zOrderCurve", () => {
    it("should encode and decode z-order curve", () => {
        const x = 3358;
        const y = 4130;
        const numBits = 13;
        const coordinateShift = 0;

        const encoded = encodeZOrderCurve(x, y, numBits, coordinateShift);
        expect(encoded).toBe(38865244);

        const decoded = decodeZOrderCurve(encoded, numBits, coordinateShift);
        expect(decoded).toEqual({ x, y });
    });

    it("should handle coordinate shift", () => {
        const x = -50;
        const y = 30;
        const numBits = 8;
        const coordinateShift = 100;

        const encoded = encodeZOrderCurve(x, y, numBits, coordinateShift);
        const decoded = decodeZOrderCurve(encoded, numBits, coordinateShift);
        expect(decoded).toEqual({ x, y });
    });
});
