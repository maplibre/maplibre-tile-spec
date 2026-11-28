import type BitVector from "../flat/bitVector.js";
import Vector from "../vector.js";

export class Int32ConstVector extends Vector<Int32Array | Uint32Array, number> {
    public constructor(name: string, value: number, sizeOrNullabilityBuffer: number | BitVector, isSigned: boolean) {
        super(name, isSigned ? Int32Array.of(value) : Uint32Array.of(value), sizeOrNullabilityBuffer);
    }

    protected getValueFromBuffer(_index: number): number {
        return this.dataBuffer[0];
    }
}
