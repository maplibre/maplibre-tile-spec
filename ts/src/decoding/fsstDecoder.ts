/**
 * Calculates the exact output size before decoding. This allows one final
 * `Uint8Array` allocation and avoids growing a JavaScript number array and
 * copying it into a typed array afterward. Traversing the compressed data
 * twice is always worthwhile here because it avoids those larger temporary
 * allocations.
 */
function getDecodedLength(symbolLengths: Uint32Array, compressedData: Uint8Array): number {
    let decodedLength = 0;
    for (let i = 0; i < compressedData.length; i++) {
        const symbolIndex = compressedData[i];
        if (symbolIndex === 255) {
            decodedLength++;
            i++; // Skip the literal byte following the escape marker.
        } else {
            decodedLength += symbolLengths[symbolIndex];
        }
    }

    return decodedLength;
}

/**
 * Decode FSST compressed data
 *
 * @param symbols           Array of symbols, where each symbol can be between 1 and 8 bytes
 * @param symbolLengths     Array of symbol lengths, length of each symbol in symbols array
 * @param compressedData    FSST Compressed data, where each entry is an index to the symbols array
 * @returns                 Decoded data as Uint8Array
 */
export function decodeFsst(symbols: Uint8Array, symbolLengths: Uint32Array, compressedData: Uint8Array): Uint8Array {
    const symbolOffsets = new Uint32Array(symbolLengths.length);

    for (let i = 1; i < symbolLengths.length; i++) {
        symbolOffsets[i] = symbolOffsets[i - 1] + symbolLengths[i - 1];
    }

    const decodedData = new Uint8Array(getDecodedLength(symbolLengths, compressedData));
    let decodedOffset = 0;
    for (let i = 0; i < compressedData.length; i++) {
        const symbolIndex = compressedData[i];
        if (symbolIndex === 255) {
            i++;
            decodedData[decodedOffset++] = compressedData[i];
        } else {
            let symbolLength = symbolLengths[symbolIndex];
            let symbolOffset = symbolOffsets[symbolIndex];
            while (symbolLength-- > 0) {
                decodedData[decodedOffset++] = symbols[symbolOffset++];
            }
        }
    }

    return decodedData;
}
