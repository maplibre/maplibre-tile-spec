import { VariableSizeVector } from "../variableSizeVector";
import type BitVector from "../flat/bitVector";
import { FsstDecoder } from "../../decoding/fsstDecoder";
import { decodeString } from "../../decoding/decodingUtils";

const dictionaryDecoders = new WeakMap<Uint8Array, WeakMap<Uint8Array, WeakMap<Uint32Array, FsstDecoder>>>();
const MIN_DENSE_VALUE_CACHE_SIZE = 256;
const DENSE_VALUE_CACHE_RATIO = 4;
type DecodedValueCache = Map<number, string> | Array<string | undefined>;

export class StringFsstDictionaryVector extends VariableSizeVector<Uint8Array, string> {
    // TODO: extend from StringVector
    private symbolLengthBuffer: Uint32Array;
    private dictionaryDecoder: FsstDecoder;
    private decodedValues?: DecodedValueCache;

    constructor(
        name: string,
        private readonly indexBuffer: Uint32Array,
        offsetBuffer: Uint32Array,
        dictionaryBuffer: Uint8Array,
        private readonly symbolOffsetBuffer: Uint32Array,
        private readonly symbolTableBuffer: Uint8Array,
        nullabilityBuffer: BitVector,
    ) {
        super(name, offsetBuffer, dictionaryBuffer, nullabilityBuffer ?? indexBuffer.length);
    }

    protected getValueFromBuffer(index: number): string {
        if (this.dictionaryDecoder == null) {
            const decodersForData = dictionaryDecoders.get(this.dataBuffer);
            const decodersForSymbols = decodersForData?.get(this.symbolTableBuffer);
            const cachedDecoder = decodersForSymbols?.get(this.symbolOffsetBuffer);
            if (cachedDecoder) {
                this.dictionaryDecoder = cachedDecoder;
            } else {
                // TODO: change FsstEncoder to take offsets instead of length to get rid of this conversion
                this.symbolLengthBuffer = this.offsetToLengthBuffer(this.symbolOffsetBuffer);
                this.dictionaryDecoder = new FsstDecoder(
                    this.symbolTableBuffer,
                    this.symbolLengthBuffer,
                    this.dataBuffer,
                );
                const symbolDecoders = decodersForSymbols ?? new WeakMap<Uint32Array, FsstDecoder>();
                symbolDecoders.set(this.symbolOffsetBuffer, this.dictionaryDecoder);
                if (!decodersForSymbols) {
                    const dataDecoders =
                        decodersForData ?? new WeakMap<Uint8Array, WeakMap<Uint32Array, FsstDecoder>>();
                    dataDecoders.set(this.symbolTableBuffer, symbolDecoders);
                    if (!decodersForData) dictionaryDecoders.set(this.dataBuffer, dataDecoders);
                }
            }
        }

        const offset = this.indexBuffer[index];
        const cached =
            this.decodedValues instanceof Map ? this.decodedValues.get(offset) : this.decodedValues?.[offset];
        if (cached !== undefined) return cached;
        const start = this.offsetBuffer[offset];
        const end = this.offsetBuffer[offset + 1];
        const decodedString = this.dictionaryDecoder.decodeRange(start, end);
        const value = decodeString(decodedString, 0, decodedString.length);
        this.cacheDecodedValue(offset, value);
        return value;
    }

    private cacheDecodedValue(index: number, value: string): void {
        let decodedValues = this.decodedValues;
        if (Array.isArray(decodedValues)) {
            decodedValues[index] = value;
            return;
        }

        if (!decodedValues) {
            decodedValues = new Map<number, string>();
            this.decodedValues = decodedValues;
        }
        decodedValues.set(index, value);
        const denseCacheThreshold = Math.max(
            MIN_DENSE_VALUE_CACHE_SIZE,
            Math.ceil((this.offsetBuffer.length - 1) / DENSE_VALUE_CACHE_RATIO),
        );
        if (decodedValues.size >= denseCacheThreshold) {
            const denseValues = new Array<string | undefined>(this.offsetBuffer.length - 1);
            for (const [cachedIndex, cachedValue] of decodedValues) denseValues[cachedIndex] = cachedValue;
            this.decodedValues = denseValues;
        }
    }

    // TODO: get rid of that conversion
    private offsetToLengthBuffer(offsetBuffer: Uint32Array): Uint32Array {
        const lengthBuffer = new Uint32Array(offsetBuffer.length - 1);
        let previousOffset = offsetBuffer[0];
        for (let i = 1; i < offsetBuffer.length; i++) {
            const offset = offsetBuffer[i];
            lengthBuffer[i - 1] = offset - previousOffset;
            previousOffset = offset;
        }

        return lengthBuffer;
    }
}
