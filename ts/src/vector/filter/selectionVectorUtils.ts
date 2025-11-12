import { type SelectionVector } from "./selectionVector";
import { FlatSelectionVector } from "./flatSelectionVector";
import type BitVector from "../flat/bitVector";

export function createSelectionVector(size: number) {
    const selectionVector = new Array(size);
    for (let i = 0; i < size; i++) {
        selectionVector[i] = i;
    }
    return new FlatSelectionVector(selectionVector);
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
