import { describe, it, expect } from "vitest";
import ZOrderCurve from "./zOrderCurve";

describe("ZOrderCurve", () => {
    it("decode", () => {
        const expectedIndex = 38865244;
        const expectedVertex = { x: 3358, y: 4130 };
        const zCurve = new ZOrderCurve(288, 4150);

        const actualIndex = zCurve.encode(expectedVertex);
        const actualVertex = zCurve.decode(actualIndex);

        expect(actualIndex).toEqual(expectedIndex);
        expect(actualVertex).toEqual(expectedVertex);
    });
});
