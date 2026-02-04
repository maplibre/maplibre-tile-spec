import { VariableSizeVector } from "../variableSizeVector";
import type BitVector from "../flat/bitVector";
import { decodeString } from "../../decoding/decodingUtils";

export class StringDictionaryVector extends VariableSizeVector<Uint8Array, string> {
    private readonly textEncoder: TextEncoder;

    constructor(
        name: string,
        private readonly indexBuffer: Int32Array,
        offsetBuffer: Uint32Array,
        dictionaryBuffer: Uint8Array,
        nullabilityBuffer?: BitVector,
    ) {
        super(name, offsetBuffer, dictionaryBuffer, nullabilityBuffer ?? indexBuffer.length);
        this.indexBuffer = indexBuffer;
        this.textEncoder = new TextEncoder();
    }

    protected getValueFromBuffer(index: number): string {
        const offset = this.indexBuffer[index];
        const start = this.offsetBuffer[offset];
        const end = this.offsetBuffer[offset + 1];
        return decodeString(this.dataBuffer, start, end);
    }
}
