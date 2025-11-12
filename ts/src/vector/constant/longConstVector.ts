import { type SelectionVector } from "../filter/selectionVector";
import type BitVector from "../flat/bitVector";
import Vector from "../vector";
import { FlatSelectionVector } from "../filter/flatSelectionVector";
import {
    createNullableSelectionVector,
    createSelectionVector,
    updateNullableSelectionVector,
} from "../filter/selectionVectorUtils";

export class LongConstVector extends Vector<BigInt64Array, bigint> {
    filter(value: bigint): SelectionVector {
        const vectorValue = this.dataBuffer[0];
        if (vectorValue !== value) {
            return new FlatSelectionVector([]);
        }

        return createNullableSelectionVector(this.size, this.nullabilityBuffer);
    }

    match(values: bigint[]): SelectionVector {
        const vectorValue = this.dataBuffer[0];
        if (!values.includes(vectorValue)) {
            return new FlatSelectionVector([]);
        }

        return createNullableSelectionVector(this.size, this.nullabilityBuffer);
    }

    filterSelected(value: bigint, selectionVector: SelectionVector): void {
        const vectorValue = this.dataBuffer[0];
        if (vectorValue !== value) {
            selectionVector.setLimit(0);
            return;
        }

        updateNullableSelectionVector(selectionVector, this.nullabilityBuffer);
    }

    matchSelected(values: bigint[], selectionVector: SelectionVector): void {
        const vectorValue = this.dataBuffer[0];
        if (!values.includes(vectorValue)) {
            selectionVector.setLimit(0);
            return;
        }

        updateNullableSelectionVector(selectionVector, this.nullabilityBuffer);
    }

    greaterThanOrEqualTo(value: bigint): SelectionVector {
        return this.dataBuffer[0] >= value
            ? createNullableSelectionVector(this.size, this.nullabilityBuffer)
            : new FlatSelectionVector([]);
    }

    greaterThanOrEqualToSelected(value: bigint, selectionVector: SelectionVector): void {
        if (this.dataBuffer[0] >= value) {
            updateNullableSelectionVector(selectionVector, this.nullabilityBuffer);
            return;
        }

        selectionVector.setLimit(0);
    }

    smallerThanOrEqualTo(value: bigint): SelectionVector {
        return this.dataBuffer[0] <= value
            ? createNullableSelectionVector(this.size, this.nullabilityBuffer)
            : new FlatSelectionVector([]);
    }

    smallerThanOrEqualToSelected(value: bigint, selectionVector: SelectionVector): void {
        if (this.dataBuffer[0] <= value) {
            updateNullableSelectionVector(selectionVector, this.nullabilityBuffer);
            return;
        }

        selectionVector.setLimit(0);
    }

    filterNotEqual(value: bigint): SelectionVector {
        return this.dataBuffer[0] !== value ? createSelectionVector(this.size) : new FlatSelectionVector([]);
    }

    filterNotEqualSelected(value: bigint, selectionVector: SelectionVector): void {
        if (this.dataBuffer[0] !== value) {
            return;
        }

        selectionVector.setLimit(0);
    }

    noneMatch(values: bigint[]): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    noneMatchSelected(values: bigint[], selectionVector: SelectionVector): void {
        throw new Error("Not implemented yet.");
    }

    public constructor(name: string, value: bigint, sizeOrNullabilityBuffer: number | BitVector) {
        super(name, BigInt64Array.of(value), sizeOrNullabilityBuffer);
    }

    protected getValueFromBuffer(index: number): bigint {
        return this.dataBuffer[0];
    }
}
