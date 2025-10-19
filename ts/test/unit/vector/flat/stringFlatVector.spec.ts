import { describe, it, expect, beforeEach } from "vitest";
import { StringFlatVector } from "../../../../src/vector/flat/stringFlatVector";

describe("StringFlatVector", () => {
    let values: string[];
    let offsetBuffer: Int32Array;
    let dataBuffer: Uint8Array;
    let stringFlatVector: StringFlatVector;

    beforeEach(() => {
        values = ["test", "test2", "test1", "test", "test2"];

        dataBuffer = new Uint8Array(values.reduce((p, c) => p + c.length, 0));
        const encoder = new TextEncoder();
        let offset = 0;
        offsetBuffer = new Int32Array(values.length + 1);
        offsetBuffer[0] = 0;
        let i = 1;
        for (const value of values) {
            const data = encoder.encode(value);
            dataBuffer.set(data, offset);
            offset += data.length;
            offsetBuffer[i++] = offset;
        }

        stringFlatVector = new StringFlatVector("test", offsetBuffer, dataBuffer, null);
    });

    describe("getValue", () => {
        it("should return correct string value", () => {
            const result = stringFlatVector.getValue(1);
            expect(result).toEqual(values[1]);
        });
    });

    describe("filter", () => {
        it("should return correct indices for a given predicate", () => {
            const value = values[0];
            const result = stringFlatVector.filter(value);
            expect(result.selectionValues()).toEqual([0, 3]);
        });
    });

    describe("filterIn", () => {
        it("should return correct indices for a given predicate", () => {
            const result = stringFlatVector.match([values[0], values[2]]);
            expect(result.selectionValues()).toEqual([0, 2, 3]);
        });
    });
});
