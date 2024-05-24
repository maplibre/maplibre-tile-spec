import { FsstDecoder } from "./fsstDecoder";

let symbols = new Uint8Array([65, 65, 0, 65, 69, 100, 67, 102, 66]);
let symbolLengths = [2, 1, 1, 1, 1, 1, 1, 1];
let compressedData = new Uint8Array([
    0, 0, 0, 2, 7, 7, 7, 0, 2, 5, 5, 4, 4, 4, 4, 4, 3, 3, 3, 3, 3, 3, 6, 6, 6, 3, 3, 3, 3, 0, 0, 2, 4, 4, 4, 4, 5, 5,
]);

let decodedData = FsstDecoder.decode(symbols, symbolLengths, compressedData);

let decoder = new TextDecoder("utf-8");
let decodedDataString = decoder.decode(decodedData);

console.log(decodedDataString);
