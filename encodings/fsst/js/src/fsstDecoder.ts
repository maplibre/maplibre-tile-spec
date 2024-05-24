export class FsstDecoder {
    static decode(symbols: Uint8Array, symbolLengths: number[], compressedData: Uint8Array): Uint8Array {
        let decodedData: number[] = [];
        let symbolOffsets: number[] = new Array(symbolLengths.length).fill(0);

        for (let i = 1; i < symbolLengths.length; i++) {
            symbolOffsets[i] = symbolOffsets[i - 1] + symbolLengths[i - 1];
        }

        for (let i = 0; i < compressedData.length; i++) {
            let symbolIndex = compressedData[i];
            if (symbolIndex === 255) {
                decodedData.push(compressedData[++i]);
            } else {
                let symbolLength = symbolLengths[symbolIndex];
                let symbolOffset = symbolOffsets[symbolIndex];
                let symbolBytes = symbols.slice(symbolOffset, symbolOffset + symbolLength);
                decodedData.push(...symbolBytes);
            }
        }
        return new Uint8Array(decodedData);
    }
}
