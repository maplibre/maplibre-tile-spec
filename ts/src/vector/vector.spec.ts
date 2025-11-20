import { describe, it, expect } from "vitest";
import { IntFlatVector } from "./flat/intFlatVector";
import BitVector from "./flat/bitVector";


function createVector(values: number[], name = "test"): IntFlatVector {
    const data = new Int32Array(values);
    return new IntFlatVector(name, data, values.length);
}

function createNullableVector(values: number[], nullBits: number, name = "test"): IntFlatVector {
    const data = new Int32Array(values);
    const nullability = new Uint8Array([nullBits]);
    const bitVector = new BitVector(nullability, values.length);
    return new IntFlatVector(name, data, bitVector);
}

// int is used for base testing since it is the simplest datatype. Edge cases are tested separately in the according vector classes
describe("BaseVector tests", () => {
    it("should be a placehoder for future tests", () => {
        expect(1).toStrictEqual(1);
    })
});
