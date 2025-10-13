import {StringDictionaryVector} from "../../../../src/vector/dictionary/stringDictionaryVector";

describe("StringDictionaryVector", () => {
    let values: string[];
    let indexBuffer: Int32Array;
    let offsetBuffer: Int32Array;
    let dataBuffer: Uint8Array;
    let stringDictionaryVector: StringDictionaryVector;

    beforeEach(() => {
        values = ["test", "test2", "test1", "test", "test2"];
        const dictionary = [...new Set(values)];

        indexBuffer = new Int32Array([0, 1, 2, 0, 1]);
        dataBuffer = new Uint8Array(dictionary.reduce((p, c) => p + c.length, 0));
        const encoder = new TextEncoder();
        let offset = 0;
        offsetBuffer = new Int32Array(values.length + 1);
        offsetBuffer[0] = 0;
        let i = 1;
        for(const value of dictionary){
            const data = encoder.encode(value);
            dataBuffer.set(data, offset);
            offset += data.length;
            offsetBuffer[i++] = offset;
        }

        stringDictionaryVector = new StringDictionaryVector("test", indexBuffer, offsetBuffer, dataBuffer, null);
    });

    describe("getValue", () => {
        it("should return correct string value", () => {
            const result = stringDictionaryVector.getValue(1);
            expect(result).toEqual(values[1]);
        });
    });

    describe("filter", () => {
        it("should return correct indices for a given predicate", () => {
            const value = values[0];
            const result = stringDictionaryVector.filter(value);
            expect(result.selectionValues()).toEqual([0, 3]);
        });
    });

    describe("filterIn", () => {
        it("should return correct indices for a given predicate", () => {
            const result = stringDictionaryVector.match([values[0], values[2]]);
            expect(result.selectionValues()).toEqual([0, 2, 3]);
        });
    });

});
