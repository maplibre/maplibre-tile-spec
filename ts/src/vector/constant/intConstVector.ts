import type BitVector from "../flat/bitVector";
import { type SelectionVector } from "../filter/selectionVector";
import { FlatSelectionVector } from "../filter/flatSelectionVector";
import Vector from "../vector";
import {
    createSelectionVector,
    createNullableSelectionVector,
    updateNullableSelectionVector,
} from "../filter/selectionVectorUtils";

export class IntConstVector extends Vector<Int32Array, number> {
    public constructor(name: string, value: number, sizeOrNullabilityBuffer: number | BitVector) {
        super(name, Int32Array.of(value), sizeOrNullabilityBuffer);
    }

    filter(value: number): SelectionVector {
        //TODO: create also different SelectionVectors -> Const, Sequence and Flat
        const vectorValue = this.dataBuffer[0];
        if (vectorValue !== value) {
            return new FlatSelectionVector([]);
        }

        return this.nullabilityBuffer
            ? createNullableSelectionVector(this.size, this.nullabilityBuffer)
            : createSelectionVector(this.size);
    }

    match(values: number[]): SelectionVector {
        const vectorValue = this.dataBuffer[0];
        if (!values.includes(vectorValue)) {
            return new FlatSelectionVector([]);
        }

        return this.nullabilityBuffer
            ? createNullableSelectionVector(this.size, this.nullabilityBuffer)
            : createSelectionVector(this.size);
    }

    filterSelected(value: number, selectionVector: SelectionVector): void {
        const vectorValue = this.dataBuffer[0];
        if (vectorValue !== value) {
            selectionVector.setLimit(0);
            return;
        }

        updateNullableSelectionVector(selectionVector, this.nullabilityBuffer);
    }

    matchSelected(values: number[], selectionVector: SelectionVector): void {
        const vectorValue = this.dataBuffer[0];
        if (!values.includes(vectorValue)) {
            selectionVector.setLimit(0);
            return;
        }

        updateNullableSelectionVector(selectionVector, this.nullabilityBuffer);
    }

    protected getValueFromBuffer(index: number): number {
        return this.dataBuffer[0];
    }

    greaterThanOrEqualTo(testValue: number): SelectionVector {
        if (this.dataBuffer[0] < testValue) {
            return new FlatSelectionVector([]);
        }

        return this.nullabilityBuffer
            ? createNullableSelectionVector(this.size, this.nullabilityBuffer)
            : createSelectionVector(this.size);
    }

    greaterThanOrEqualToSelected(value: number, selectionVector: SelectionVector): void {
        if (this.dataBuffer[0] >= value) {
            updateNullableSelectionVector(selectionVector, this.nullabilityBuffer);
            return;
        }

        selectionVector.setLimit(0);
    }

    smallerThanOrEqualTo(value: number): SelectionVector {
        if (this.dataBuffer[0] > value) {
            return new FlatSelectionVector([]);
        }

        return this.nullabilityBuffer
            ? createNullableSelectionVector(this.size, this.nullabilityBuffer)
            : createSelectionVector(this.size);
    }

    smallerThanOrEqualToSelected(value: number, selectionVector: SelectionVector): void {
        if (this.dataBuffer[0] <= value) {
            updateNullableSelectionVector(selectionVector, this.nullabilityBuffer);
            return;
        }

        selectionVector.setLimit(0);
    }

    filterNotEqual(value: number): SelectionVector {
        return this.dataBuffer[0] !== value ? createSelectionVector(this.size) : new FlatSelectionVector([]);
    }

    filterNotEqualSelected(value: number, selectionVector: SelectionVector): void {
        if (this.dataBuffer[0] !== value) {
            return;
        }

        selectionVector.setLimit(0);
    }

    noneMatch(values: number[]): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    noneMatchSelected(values: number[], selectionVector: SelectionVector): void {
        throw new Error("Not implemented yet.");
    }
}
