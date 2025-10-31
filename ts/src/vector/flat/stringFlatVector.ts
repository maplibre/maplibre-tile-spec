import { VariableSizeVector } from "../variableSizeVector";
import type BitVector from "./bitVector";
import { decodeString } from "../../decoding/decodingUtils";

export class StringFlatVector extends VariableSizeVector<Uint8Array, string> {
    private readonly textEncoder: TextEncoder;

    constructor(name: string, offsetBuffer: Int32Array, dataBuffer: Uint8Array, nullabilityBuffer?: BitVector) {
        super(name, offsetBuffer, dataBuffer, nullabilityBuffer ?? offsetBuffer.length - 1);
        this.textEncoder = new TextEncoder();
    }

    protected getValueFromBuffer(index: number): string {
        const start = this.offsetBuffer[index];
        const end = this.offsetBuffer[index + 1];
        return decodeString(this.dataBuffer, start, end);
    }
}
