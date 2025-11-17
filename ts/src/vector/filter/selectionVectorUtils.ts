import { type SelectionVector } from "./selectionVector";
import { FlatSelectionVector } from "./flatSelectionVector";
import type BitVector from "../flat/bitVector";
import { SequenceSelectionVector } from "./sequenceSelectionVector";

export function createSelectionVector(size: number) {
    return new SequenceSelectionVector(0, 1, size);
}

export function createNullableSelectionVector(size: number, nullabilityBuffer: BitVector) {
    const selectionVector = [];
    for (let i = 0; i < size; i++) {
        if (nullabilityBuffer?.get(i)) {
            selectionVector.push(i);
        }
    }
    return new FlatSelectionVector(selectionVector);
}

/** Returns new selection vector with only indices where nullability is true. */
export function updateNullableSelectionVector(selectionVector: SelectionVector, nullabilityBuffer: BitVector): SelectionVector {
    const filteredIndices = [];
    for (let i = 0; i < selectionVector.limit; i++) {
        const vectorIndex = selectionVector.getIndex(i);
        if (!nullabilityBuffer || nullabilityBuffer.get(vectorIndex)) {
            filteredIndices.push(vectorIndex);
        }
    }
    return new FlatSelectionVector(filteredIndices);
}
