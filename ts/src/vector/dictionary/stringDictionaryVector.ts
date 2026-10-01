import { VariableSizeVector } from "../variableSizeVector";
import type BitVector from "../flat/bitVector";
import { decodeString } from "../../decoding/decodingUtils";

export class StringDictionaryVector extends VariableSizeVector<Uint8Array, string> {
    constructor(
        name: string,
        private readonly indexBuffer: Uint32Array,
        offsetBuffer: Uint32Array,
        dictionaryBuffer: Uint8Array,
        nullabilityBuffer?: BitVector,
    ) {
        super(name, offsetBuffer, dictionaryBuffer, nullabilityBuffer ?? indexBuffer.length);
        this.indexBuffer = indexBuffer;
    }

    protected getValueFromBuffer(index: number): string {
        const offset = this.indexBuffer[index];
        const start = this.offsetBuffer[offset];
        const end = this.offsetBuffer[offset + 1];
        return decodeString(this.dataBuffer, start, end);
    }
}
