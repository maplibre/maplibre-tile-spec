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

    let decodedLength = 0;
    for (let i = 0; i < compressedData.length; i++) {
        const symbolIndex = compressedData[i];
        if (symbolIndex === 255) {
            decodedLength++;
            i++;
        } else {
            decodedLength += symbolLengths[symbolIndex];
        }
    }

    const decodedData = new Uint8Array(decodedLength);
    let decodedOffset = 0;
    for (let i = 0; i < compressedData.length; i++) {
        const symbolIndex = compressedData[i];
        if (symbolIndex === 255) {
            decodedData[decodedOffset++] = compressedData[++i];
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

const CHECKPOINT_INTERVAL = 1 << 14;
// Sparse reads avoid expanding the dictionary. Dense consumers switch to the
// two-pass sequential decoder once sparse scans have examined the compressed
// input twice, making the decision proportional to the dictionary size.
const FULL_DECODE_SCAN_MULTIPLIER = 2;

export class FsstDecoder {
    private readonly symbolOffsets: Uint32Array;
    private readonly compressedCheckpoints: number[] = [0];
    private readonly decodedCheckpoints: number[] = [0];
    private indexedCompressedOffset = 0;
    private indexedDecodedOffset = 0;
    private nextCheckpoint = CHECKPOINT_INTERVAL;
    private sparseBytesScanned = 0;
    private decodedData?: Uint8Array;

    constructor(
        private readonly symbols: Uint8Array,
        private readonly symbolLengths: Uint32Array,
        private readonly compressedData: Uint8Array,
    ) {
        this.symbolOffsets = new Uint32Array(symbolLengths.length);
        for (let i = 1; i < symbolLengths.length; i++) {
            this.symbolOffsets[i] = this.symbolOffsets[i - 1] + symbolLengths[i - 1];
        }
    }

    decodeRange(start: number, end: number): Uint8Array {
        if (this.decodedData) return this.decodedData.subarray(start, end);
        if (this.sparseBytesScanned >= this.compressedData.length * FULL_DECODE_SCAN_MULTIPLIER) {
            return this.decode().subarray(start, end);
        }
        this.sparseBytesScanned += this.indexThrough(start);

        let low = 0;
        let high = this.decodedCheckpoints.length;
        while (low + 1 < high) {
            const middle = (low + high) >> 1;
            if (this.decodedCheckpoints[middle] <= start) low = middle;
            else high = middle;
        }

        const decodedData = new Uint8Array(end - start);
        let compressedOffset = this.compressedCheckpoints[low];
        const rangeStart = compressedOffset;
        let decodedOffset = this.decodedCheckpoints[low];
        let outputOffset = 0;
        while (compressedOffset < this.compressedData.length && decodedOffset < end) {
            const symbolIndex = this.compressedData[compressedOffset++];
            if (symbolIndex === 255) {
                const value = this.compressedData[compressedOffset++];
                if (decodedOffset >= start) decodedData[outputOffset++] = value;
                decodedOffset++;
            } else {
                const symbolLength = this.symbolLengths[symbolIndex];
                const symbolOffset = this.symbolOffsets[symbolIndex];
                const copyStart = Math.max(start - decodedOffset, 0);
                const copyEnd = Math.min(end - decodedOffset, symbolLength);
                for (let i = copyStart; i < copyEnd; i++) {
                    decodedData[outputOffset++] = this.symbols[symbolOffset + i];
                }
                decodedOffset += symbolLength;
            }
        }
        this.sparseBytesScanned += compressedOffset - rangeStart;

        return outputOffset === decodedData.length ? decodedData : decodedData.subarray(0, outputOffset);
    }

    decode(): Uint8Array {
        return (this.decodedData ??= decodeFsst(this.symbols, this.symbolLengths, this.compressedData));
    }

    private indexThrough(targetDecodedOffset: number): number {
        const initialCompressedOffset = this.indexedCompressedOffset;
        let compressedOffset = this.indexedCompressedOffset;
        let decodedOffset = this.indexedDecodedOffset;
        let nextCheckpoint = this.nextCheckpoint;
        const compressedData = this.compressedData;
        const symbolLengths = this.symbolLengths;
        const compressedCheckpoints = this.compressedCheckpoints;
        const decodedCheckpoints = this.decodedCheckpoints;

        while (compressedOffset < compressedData.length && decodedOffset < targetDecodedOffset) {
            const symbolIndex = compressedData[compressedOffset++];
            if (symbolIndex === 255) {
                decodedOffset++;
                compressedOffset++;
            } else {
                decodedOffset += symbolLengths[symbolIndex];
            }

            if (decodedOffset >= nextCheckpoint) {
                compressedCheckpoints.push(compressedOffset);
                decodedCheckpoints.push(decodedOffset);
                nextCheckpoint = decodedOffset + CHECKPOINT_INTERVAL;
            }
        }

        this.indexedCompressedOffset = compressedOffset;
        this.indexedDecodedOffset = decodedOffset;
        this.nextCheckpoint = nextCheckpoint;
        return compressedOffset - initialCompressedOffset;
    }
}
