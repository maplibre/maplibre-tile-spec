import {
    decodeZigZagVarint,
    decodeVarint,
    isBitSet,
    decodeUint32Rle,
    decodeInt64Rle,
    decodeString,
} from "../../../src/decoder/decodingUtils";

describe("decode", () => {
    describe("decodeVarint", () => {
        it("should decode unsigned int with 1 byte", () => {
            const value = 10;
            const varintBuffer = new Uint8Array([10]);

            const [actualValue, numBytes] = decodeVarint(varintBuffer);

            expect(actualValue).toEqual(value);
            expect(numBytes).toEqual(1);
        });

        it("should decode unsigned int with 4 bytes", () => {
            /* 100 10000000 10000000 10000000 */
            const value = 8388608;
            const varintBuffer = new Uint8Array([0x80, 0x80, 0x80, 4]);

            const [actualValue, numBytes] = decodeVarint(varintBuffer);

            expect(actualValue).toEqual(value);
            expect(numBytes).toEqual(4);
        });

        it("should decode unsigned int with 4 bytes and buffer offset", () => {
            /* 100 10000000 10000000 10000000 */
            const value = 8388608;
            const varintBuffer = new Uint8Array([0x80, 0x80, 0x80, 0x80, 0x80, 4]);

            const [actualValue, offset] = decodeVarint(varintBuffer, 2);

            expect(actualValue).toEqual(value);
            expect(offset).toEqual(6);
        });

        it("should decode unsigned int with 7 bytes and buffer offset", () => {
            /* 100 10000000 10000000 10000000 10000000 10000000 10000000 */
            const value = 17592186044416;
            const varintBuffer = new Uint8Array([0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 4]);

            const [actualValue, offset] = decodeVarint(varintBuffer, 2);

            expect(actualValue).toEqual(value);
            expect(offset).toEqual(9);
        });
    });

    //TODO: check why failing?
    describe("decodeZigZagVarint", () => {
        it("should decode unsigned int with 1 byte", () => {
            /* ZigZag Varint -> 100 10011011 */
            const value = -270;
            const varintBuffer = new Uint8Array([155, 4]);

            const [actualValue, numBytes] = decodeZigZagVarint(varintBuffer);

            expect(actualValue).toEqual(value);
            expect(numBytes).toEqual(2);
        });
    });

    describe("decodeRle", () => {
        it("should decode runs", () => {
            const expectedValues = [1, 2, 3, 4, 5, 1, 2, 3, 4, 5];
            const rleBuffer = new Uint8Array([2, 1, 1, 2, 1, 1]);

            const [actualValues, newOffset] = decodeUint32Rle(rleBuffer, expectedValues.length, 0);

            expect(convertBigInt64ArrayToNumberArray(actualValues)).toEqual(expectedValues);
            expect(newOffset).toEqual(rleBuffer.length);
        });

        it.skip("should decode literals and runs in combination", () => {
            const run1Values = Array.from(new Array(100).keys())
                .map(() => 7)
                .reverse();
            const literalsValues = [2, 3, 6, 7, 11];
            const run2Values = [...Array(51).keys()].reverse();
            run2Values.pop();
            const run1 = [0x61, 0x00, 0xe];
            const literals = [0xfb, ...[0x02, 0x03, 0x06, 0x07, 0xb].map((i) => (i >> 31) ^ (i << 1))];
            const run2 = [0x2f, ...[-1, 0x32].map((i) => (i >> 31) ^ (i << 1))];
            const encodedRleValues = new Uint8Array([...run1, ...literals, ...run2]);
            const numValues = run1Values.length + literalsValues.length + run2Values.length;

            //const [values, newOffset] = decodeInt64Rle(encodedRleValues, numValues, 0);
            //const [values, newOffset] = decodeUint32Rle(encodedRleValues, numValues, 0);
            const [values, newOffset] = decodeRleTest(encodedRleValues, numValues, true, 0);

            expect(convertBigInt64ArrayToNumberArray(values)).toEqual([
                ...run1Values,
                ...literalsValues,
                ...run2Values,
            ]);
            expect(newOffset).toEqual(12);
        });
    });

    describe("isSet", () => {
        it("should decode BitSet", () => {
            const buffer = new Uint8Array([0, 2]);

            expect(isBitSet(buffer, 9)).toBeTruthy();
            expect(isBitSet(buffer, 8)).toBeFalsy();
        });
    });

    describe("decodeString", () => {
        it("should decode short string", () => {
            const expectedValue = "Test";
            const buffer = new TextEncoder().encode(expectedValue);
            const offset = 0;
            const actualValue = decodeString(buffer, offset, buffer.length);
            expect(actualValue).toEqual(expectedValue);
        });

        it("should decode longer string", () => {
            const expectedValue = "Test String that is longer that 12 bytes";
            const buffer = new TextEncoder().encode(expectedValue);
            const offset = 0;
            const actualValue = decodeString(buffer, offset, buffer.length);
            expect(actualValue).toEqual(expectedValue);
        });

        it("should decode short string with offset", () => {
            const expectedValue = "Test";
            const utf8EncodedValue = new TextEncoder().encode(expectedValue);
            const buffer = new Uint8Array([0, 0, expectedValue.length, ...utf8EncodedValue]);

            const offset = 3;
            expect(buffer.length).toEqual(expectedValue.length + offset);
            const endOffset = buffer.length;
            const actualValue = decodeString(buffer, offset, endOffset);
            expect(actualValue).toEqual(expectedValue);
        });

        it("should decode longer string with offset", () => {
            const expectedValue = "Test String that is longer that 12 bytes";
            const utf8EncodedValue = new TextEncoder().encode(expectedValue);
            const buffer = new Uint8Array([0, 0, expectedValue.length, ...utf8EncodedValue]);

            const offset = 3;
            expect(buffer.length).toEqual(expectedValue.length + offset);
            const endOffset = buffer.length;
            const actualValue = decodeString(buffer, offset, endOffset);
            expect(actualValue).toEqual(expectedValue);
        });
    });
});

function convertBigInt64ArrayToNumberArray(bigIntArray): number[] {
    const numberArray = [];
    for (let i = 0; i < bigIntArray.length; i++) {
        const numberValue = Number(bigIntArray[i]);
        numberArray.push(numberValue);
    }
    return numberArray;
}

export function decodeRleTest(
    buffer: Uint8Array,
    numValues: number,
    signed = true,
    offset = 0,
): [values: BigInt64Array, offset: number] {
    const values = new BigInt64Array(numValues);

    let valuesCounter = 0;
    while (valuesCounter < numValues) {
        const header = buffer[offset++];

        /* Runs */
        if (header <= 0x7f) {
            const numRuns = header + 3;
            //TODO: get rid of that DataView construction
            const delta = new DataView(buffer.buffer).getInt8(offset++);
            //TODO: get rid of that branch
            const [firstValue, newOffset] = signed ? decodeZigZagVarint(buffer, offset) : decodeVarint(buffer, offset);
            offset = newOffset;

            for (let i = 0; i < numRuns; i++) {
                values[valuesCounter++] = BigInt(firstValue + i * delta);
            }
        } else {
            /* Literals */
            const numLiterals = 256 - header;
            for (let i = 0; i < numLiterals; i++) {
                const [value, newOffset] = signed ? decodeZigZagVarint(buffer, offset) : decodeVarint(buffer, offset);
                values[valuesCounter++] = BigInt(value);
                offset = newOffset;
            }
        }
    }

    return [values, offset];
}
