/**
 * Decode FSST compressed data
 *
 * @param symbols           Array of symbols, where each symbol can be between 1 and 8 bytes
 * @param symbolLengths     Array of symbol lengths, length of each symbol in symbols array
 * @param compressedData    FSST Compressed data, where each entry is an index to the symbols array
 * @returns                 Decoded data as Uint8Array
 */
export function decodeFsst(symbols: Uint8Array, symbolLengths: number[], compressedData: Uint8Array): Uint8Array {
    let decodedData: number[] = [];
    let symbolOffsets: number[] = new Array(symbolLengths.length).fill(0);

    for (let i = 1; i < symbolLengths.length; i++) {
        symbolOffsets[i] = symbolOffsets[i - 1] + symbolLengths[i - 1];
    }

    for (let i = 0; i < compressedData.length; i++) {
        if (compressedData[i] === 255) {
            decodedData.push(compressedData[++i]);
        } else {
            for (let j = 0; j < symbolLengths[compressedData[i]]; j++) {
                decodedData.push(symbols[symbolOffsets[compressedData[i]] + j]);
            }
        }
    }
    return new Uint8Array(decodedData);
}
