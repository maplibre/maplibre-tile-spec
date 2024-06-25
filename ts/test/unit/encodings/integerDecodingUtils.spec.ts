import varint from "varint";
import {
    decodeVarintInt32,
    decodeVarintFloat64,
    decodeZigZagFloat64
} from "../../../src/encodings/integerDecodingUtils";
import IntWrapper from "../../../src/encodings/intWrapper";
import {zigZagEncode64} from "bytebuffer";

const numValues = 200_000;
const randomValues = new Int32Array(numValues);
const maxValue = 2 ** 30;
for(let i = 0; i < randomValues.length; i++) {
    randomValues[i] = Math.floor(Math.random() * maxValue);
}
const randomVarintValues = varintEncode(randomValues);


describe("IntegerDecodingUtils", () => {
    describe("decodeVarint", () => {
        it("should return valid decoded values", () => {
            const actualValues = decodeVarintInt32(randomVarintValues, new IntWrapper(0),
                randomValues.length);

            expect(actualValues).toEqual(randomValues);
        });
    });
    describe("decodeVarintLongToFloat64", () => {
        it("should return valid decoded values", () => {
            const value = 2** 40;
            const varintEncoded = varintEncodeNum(value);

            const actualValues = decodeVarintFloat64(varintEncoded, 1, new IntWrapper(0));

            expect(actualValues[0]).toEqual(value);
        });
    });
    describe("decodeZigZagFloat64", () => {
        it("should return valid decoded values for zigZag Varint decoding", () => {
            const value = 2** 35;
            const zigZagValue = value * 2;
            const varintEncoded = varintEncodeNum(zigZagValue);

            const actualValues = decodeVarintFloat64(varintEncoded, 1, new IntWrapper(0));
            decodeZigZagFloat64(actualValues);

            expect(actualValues[0]).toEqual(value);
        });
        it("should return valid decoded values for zigZag Varint decoding", () => {
            const value = 3298190;
            const zigZagValue = value << 1;
            const varintEncoded = varintEncodeNum(zigZagValue);

            const actualValues = decodeVarintFloat64(varintEncoded, 1, new IntWrapper(0));
            decodeZigZagFloat64(actualValues);

            expect(actualValues[0]).toEqual(value);
        });
    });
});

function varintEncode(values: Int32Array){
    const varintValues = [];
    for(let i = 0; i < values.length; i++){
        const v = varint.encode(values[i]);
        varintValues.push(...v);
    }

    return new Uint8Array(varintValues);
}

function varintEncodeNum(value: number){
    const v = varint.encode(value);
    return new Uint8Array(v);
}


