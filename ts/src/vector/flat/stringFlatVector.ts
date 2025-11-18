import { VariableSizeVector } from "../variableSizeVector";
import type BitVector from "./bitVector";
import { decodeString } from "../../decoding/decodingUtils";
import { type SelectionVector } from "../filter/selectionVector";

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

    override filter(value: string): SelectionVector {
        throw new Error("Method not implemented");
    }
    override filterNotEqual(value: string): SelectionVector {
        throw new Error("Method not implemented");
    }
    override match(values: string[]): SelectionVector {
        throw new Error("Method not implemented");
    }
    override noneMatch(values: string[]): SelectionVector {
        throw new Error("Method not implemented");
    }
    override filterSelected(value: string, selectionVector: SelectionVector): void {
        throw new Error("Method not implemented");
    }
    override filterNotEqualSelected(value: string, selectionVector: SelectionVector): void {
        throw new Error("Method not implemented");
    }
    override matchSelected(values: string[], selectionVector: SelectionVector): void {
        throw new Error("Method not implemented");
    }
    override noneMatchSelected(values: string[], selectionVector: SelectionVector): void {
        throw new Error("Method not implemented");
    }
    override greaterThanOrEqualTo(value: string): SelectionVector {
        throw new Error("Method not available for type string");
    }
    override smallerThanOrEqualTo(value: string): SelectionVector {
        throw new Error("Method not available for type string");
    }
    override greaterThanOrEqualToSelected(value: string, selectionVector: SelectionVector): void {
        throw new Error("Method not available for type string");
    }
    override smallerThanOrEqualToSelected(value: string, selectionVector: SelectionVector): void {
        throw new Error("Method not available for type string");
    }
}
