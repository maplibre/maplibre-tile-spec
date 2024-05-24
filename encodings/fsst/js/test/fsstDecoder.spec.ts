import { FsstDecoder } from "../src/fsstDecoder";

describe("FsstDecoder", () => {
    const expectedOutput: string = "AAAAAAABBBAAACCdddddEEEEEEfffEEEEAAAAAddddCC";
    const symbols = new Uint8Array([65, 65, 0, 65, 69, 100, 67, 102, 66]);
    const symbolLengths = [2, 1, 1, 1, 1, 1, 1, 1];
    const compressedData = new Uint8Array([
        0, 0, 0, 2, 7, 7, 7, 0, 2, 5, 5, 4, 4, 4, 4, 4, 3, 3, 3, 3, 3, 3, 6, 6, 6, 3, 3, 3, 3, 0, 0, 2, 4, 4, 4, 4, 5,
        5,
    ]);

    it("decodes compressed data correctly", () => {
        const decodedData = FsstDecoder.decode(symbols, symbolLengths, compressedData);
        const decoder = new TextDecoder("utf-8");
        const decodedDataString = decoder.decode(decodedData);
        expect(decodedDataString).toStrictEqual(expectedOutput);
    });

    it("returns an empty array when compressed data is empty", () => {
        const decodedData = FsstDecoder.decode(symbols, symbolLengths, new Uint8Array([]));
        expect(decodedData).toEqual(new Uint8Array([]));
    });
});
