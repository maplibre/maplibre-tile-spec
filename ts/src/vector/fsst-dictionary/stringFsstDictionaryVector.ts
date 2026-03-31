import { VariableSizeVector } from "../variableSizeVector";
import type BitVector from "../flat/bitVector";
import { decodeFsst } from "../../decoding/fsstDecoder";
import { decodeString } from "../../decoding/decodingUtils";

export class StringFsstDictionaryVector extends VariableSizeVector<Uint8Array, string> {
    // TODO: extend from StringVector
    private symbolLengthBuffer: Uint32Array;
    private decodedDictionary: Uint8Array;

    constructor(
        name: string,
        private readonly indexBuffer: Uint32Array,
        offsetBuffer: Uint32Array,
        dictionaryBuffer: Uint8Array,
        private readonly symbolOffsetBuffer: Uint32Array,
        private readonly symbolTableBuffer: Uint8Array,
        nullabilityBuffer: BitVector,
    ) {
        super(name, offsetBuffer, dictionaryBuffer, nullabilityBuffer);
    }

    protected getValueFromBuffer(index: number): string {
        if (this.decodedDictionary == null) {
            if (this.symbolLengthBuffer == null) {
                // TODO: change FsstEncoder to take offsets instead of length to get rid of this conversion
                this.symbolLengthBuffer = this.offsetToLengthBuffer(this.symbolOffsetBuffer);
            }

            this.decodedDictionary = decodeFsst(this.symbolTableBuffer, this.symbolLengthBuffer, this.dataBuffer);
        }

        const offset = this.indexBuffer[index];
        const start = this.offsetBuffer[offset];
        const end = this.offsetBuffer[offset + 1];
        return decodeString(this.decodedDictionary, start, end);
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
