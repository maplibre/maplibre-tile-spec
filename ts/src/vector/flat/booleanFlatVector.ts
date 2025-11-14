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

    filter(value: boolean): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    match(values: boolean[]): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    filterSelected(value: boolean, selectionVector: SelectionVector): void {
        throw new Error("Not implemented yet.");
    }

    matchSelected(values: boolean[], selectionVector: SelectionVector): void {
        throw new Error("Not implemented yet.");
    }

    filterNotEqual(value: boolean): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    filterNotEqualSelected(value: boolean, selectionVector: SelectionVector): void {
        throw new Error("Not implemented yet.");
    }

    greaterThanOrEqualTo(value: boolean): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    greaterThanOrEqualToSelected(value: boolean, selectionVector: SelectionVector): void {
        throw new Error("Not implemented yet.");
    }

    noneMatch(values: boolean[]): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    noneMatchSelected(values: boolean[], selectionVector: SelectionVector): void {
        throw new Error("Not implemented yet.");
    }

    smallerThanOrEqualTo(value: boolean): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    smallerThanOrEqualToSelected(value: boolean, selectionVector: SelectionVector): void {
        throw new Error("Not implemented yet.");
    }
}
