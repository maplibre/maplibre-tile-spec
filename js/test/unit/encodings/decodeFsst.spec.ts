import { decodeFsst } from "../../../src/encodings/fsst";

describe("DecodeFsst", (): void => {
    const expectedOutput: string = "AAAAAAABBBAAACCdddddEEEEEEfffEEEEAAAAAddddCC";
    const symbols: Uint8Array = new Uint8Array([65, 65, 0, 65, 69, 100, 67, 102, 66]);
    const symbolLengths: number[] = [2, 1, 1, 1, 1, 1, 1, 1];
    const compressedData: Uint8Array = new Uint8Array([
        0, 0, 0, 2, 7, 7, 7, 0, 2, 5, 5, 4, 4, 4, 4, 4, 3, 3, 3, 3, 3, 3, 6, 6, 6, 3, 3, 3, 3, 0, 0, 2, 4, 4, 4, 4, 5,
        5,
    ]);

    it("decodes compressed data correctly", (): void => {
        const decodedData: Uint8Array = decodeFsst(symbols, symbolLengths, compressedData);
        const texDecoder: TextDecoder = new TextDecoder("utf-8");
        const decodedDataString: string = texDecoder.decode(decodedData);
        expect(decodedDataString).toStrictEqual(expectedOutput);
    });

    it("returns an empty array when compressed data is empty", (): void => {
        const decodedData: Uint8Array = decodeFsst(symbols, symbolLengths, new Uint8Array([]));
        expect(decodedData).toEqual(new Uint8Array([]));
    });
});
