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

export function updateSelectionVector(selectionVector: SelectionVector, nullabilityBuffer: BitVector) {
    const buffer = selectionVector.selectionValues();
    let limit = 0;
    for (let i = 0; i < selectionVector.limit; i++) {
        if (!nullabilityBuffer || nullabilityBuffer.get(i)) {
            buffer[limit++] = buffer[i];
        }
    }

    selectionVector.setLimit(limit);
}

export function updateNullableSelectionVector(selectionVector: SelectionVector, nullabilityBuffer: BitVector) {
    const buffer = selectionVector.selectionValues();
    let limit = 0;
    for (let i = 0; i < selectionVector.limit; i++) {
        if (!nullabilityBuffer || nullabilityBuffer.get(i)) {
            buffer[limit++] = buffer[i];
        }
    }
    selectionVector.setLimit(limit);
}
