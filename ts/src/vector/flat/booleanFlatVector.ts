import type BitVector from "./bitVector.js";
import Vector from "../vector.js";

export class BooleanFlatVector extends Vector<Uint8Array, boolean> {
    private readonly dataVector: BitVector;

    constructor(name: string, dataVector: BitVector, sizeOrNullabilityBuffer: number | BitVector) {
        super(name, dataVector.getBuffer(), sizeOrNullabilityBuffer);
        this.dataVector = dataVector;
    }

    protected getValueFromBuffer(index: number): boolean {
        return this.dataVector.get(index);
    }
}
