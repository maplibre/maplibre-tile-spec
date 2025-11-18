import type BitVector from "./bitVector";
import Vector from "../vector";
import { type SelectionVector } from "../filter/selectionVector";

export class BooleanFlatVector extends Vector<Uint8Array, boolean> {
    private readonly dataVector: BitVector;

    constructor(name: string, dataVector: BitVector, sizeOrNullabilityBuffer: number | BitVector) {
        super(name, dataVector.getBuffer(), sizeOrNullabilityBuffer);
        this.dataVector = dataVector;
    }

    protected getValueFromBuffer(index: number): boolean {
        return this.dataVector.get(index);
    }
    override greaterThanOrEqualTo(value: boolean): SelectionVector {
        throw new Error("Method not available for type boolean");
    }
    override smallerThanOrEqualTo(value: boolean): SelectionVector {
        throw new Error("Method not available for type boolean");
    }
    override greaterThanOrEqualToSelected(value: boolean, selectionVector: SelectionVector): void {
        throw new Error("Method not available for type boolean");
    }
    override smallerThanOrEqualToSelected(value: boolean, selectionVector: SelectionVector): void {
        throw new Error("Method not available for type boolean");
    }
}
