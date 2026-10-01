/**
 * Create symbol table from string array
 *
 * @param symbolStrings     Array of symbol strings
 * @returns                 Symbol table buffer and lengths
 */
export function createSymbolTable(symbolStrings: string[]): { symbols: Uint8Array; symbolLengths: Uint32Array } {
    const textEncoder = new TextEncoder();
    const symbolBuffers = symbolStrings.map((s) => textEncoder.encode(s));
    const symbolLengths = new Uint32Array(symbolBuffers.map((b) => b.length));
    const totalLength = symbolBuffers.reduce((sum, b) => sum + b.length, 0);
    const symbols = new Uint8Array(totalLength);

    let offset = 0;
    for (const buffer of symbolBuffers) {
        symbols.set(buffer, offset);
        offset += buffer.length;
    }

    return { symbols, symbolLengths };
}

/**
 * Encode data using FSST compression with pre-defined symbol table
 * Encoder requires pre-defined symbol table. Real FSST learns optimal symbols from data. This
 * implementation is for testing decoder only.
 *
 * @param symbols           Array of symbols, where each symbol can be between 1 and 8 bytes
 * @param symbolLengths     Array of symbol lengths, length of each symbol in symbols array
 * @param uncompressedData  Data to compress
 * @returns                 FSST compressed data, where each entry is an index to the symbols array
 */
export function encodeFsst(symbols: Uint8Array, symbolLengths: Uint32Array, uncompressedData: Uint8Array): Uint8Array {
    if (uncompressedData.length === 0) {
        return new Uint8Array(0);
    }

    // Calculate symbol offsets (cumulative sum of lengths)
    const symbolOffsets: number[] = new Array(symbolLengths.length).fill(0);
    for (let i = 1; i < symbolLengths.length; i++) {
        symbolOffsets[i] = symbolOffsets[i - 1] + symbolLengths[i - 1];
    }

    const result: number[] = [];
    let pos = 0;

    while (pos < uncompressedData.length) {
        let bestSymbolIndex = -1;
        let bestSymbolLength = 0;

        // Try to find longest matching symbol at current position
        for (let symbolIndex = 0; symbolIndex < symbolLengths.length; symbolIndex++) {
            const symbolLength = symbolLengths[symbolIndex];
            const symbolOffset = symbolOffsets[symbolIndex];

            // Check if symbol could fit and is longer than current best
            if (pos + symbolLength <= uncompressedData.length && symbolLength > bestSymbolLength) {
                // Check if bytes match
                let matches = true;
                for (let i = 0; i < symbolLength; i++) {
                    if (symbols[symbolOffset + i] !== uncompressedData[pos + i]) {
                        matches = false;
                        break;
                    }
                }

                if (matches) {
                    bestSymbolIndex = symbolIndex;
                    bestSymbolLength = symbolLength;
                }
            }
        }

        if (bestSymbolIndex !== -1) {
            // Found a matching symbol
            result.push(bestSymbolIndex);
            pos += bestSymbolLength;
        } else {
            // No match - emit escape sequence (255 followed by literal byte)
            result.push(255);
            result.push(uncompressedData[pos]);
            pos++;
        }
    }

    return new Uint8Array(result);
}
