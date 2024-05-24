import { decodeFsst } from "./decodeFsst";

let symbols: Uint8Array = new Uint8Array([65, 65, 0, 65, 69, 100, 67, 102, 66]);
let symbolLengths: number[] = [2, 1, 1, 1, 1, 1, 1, 1];
let compressedData: Uint8Array = new Uint8Array([
    0, 0, 0, 2, 7, 7, 7, 0, 2, 5, 5, 4, 4, 4, 4, 4, 3, 3, 3, 3, 3, 3, 6, 6, 6, 3, 3, 3, 3, 0, 0, 2, 4, 4, 4, 4, 5, 5,
]);

let decodedData: Uint8Array = decodeFsst(symbols, symbolLengths, compressedData);

let textDecoder: TextDecoder = new TextDecoder("utf-8");
let decodedDataString: string = textDecoder.decode(decodedData);

console.log(decodedDataString);
