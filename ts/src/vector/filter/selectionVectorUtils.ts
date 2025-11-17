import { type SelectionVector } from "./selectionVector";
import { FlatSelectionVector } from "./flatSelectionVector";
import type BitVector from "../flat/bitVector";
import { SequenceSelectionVector } from "./sequenceSelectionVector";

export function createSelectionVector(size: number) {
    return new SequenceSelectionVector(0, 1, size);
}

/**
 * Creates a selection vector containing indices of non-null values.
 * @param size - The total number of elements to consider
 * @param nullabilityBuffer - Optional bit vector where 1=not null, 0=null. If undefined/null, all values are considered non-null.
 */
export function createNullableSelectionVector(size: number, nullabilityBuffer?: BitVector): SelectionVector {
    const selectionVector = [];
    for (let i = 0; i < size; i++) {
        // Include index if no nullability buffer (all non-null) OR if bit is set (non-null)
        if (!nullabilityBuffer || nullabilityBuffer.get(i)) {
            selectionVector.push(i);
        }
    }
    return new FlatSelectionVector(selectionVector);
}

/**
 * Filters an existing selection vector to include only non-null values.
 * @param selectionVector - The input selection vector to filter
 * @param nullabilityBuffer - Optional bit vector where 1=not null, 0=null. If undefined/null, all values are considered non-null.
 */
export function updateNullableSelectionVector(selectionVector: SelectionVector, nullabilityBuffer?: BitVector): SelectionVector {
    const filteredIndices = [];
    for (let i = 0; i < selectionVector.limit; i++) {
        const vectorIndex = selectionVector.getIndex(i);
        // Include index if no nullability buffer (all non-null) OR if bit is set (non-null)
        if (!nullabilityBuffer || nullabilityBuffer.get(vectorIndex)) {
            filteredIndices.push(vectorIndex);
        }
    }
    return new FlatSelectionVector(filteredIndices);
}
