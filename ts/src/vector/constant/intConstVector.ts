import type BitVector from "../flat/bitVector";
import Vector from "../vector";

export class IntConstVector extends Vector<Int32Array, number> {
    public constructor(name: string, value: number, sizeOrNullabilityBuffer: number | BitVector) {
        super(name, Int32Array.of(value), sizeOrNullabilityBuffer);
    }

    protected getValueFromBuffer(index: number): number {
        return this.dataBuffer[0];
    }
}
