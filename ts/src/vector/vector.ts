import type BitVector from "./flat/bitVector";
import { type SelectionVector } from "./filter/selectionVector";
import { FlatSelectionVector } from "./filter/flatSelectionVector";

export default abstract class Vector<T extends ArrayBufferView = ArrayBufferView, K = unknown> {
    protected nullabilityBuffer: BitVector | null;
    protected _size: number;

    constructor(
        private readonly _name: string,
        protected readonly dataBuffer: T,
        sizeOrNullabilityBuffer: number | BitVector,
    ) {
        if (typeof sizeOrNullabilityBuffer === "number") {
            this._size = sizeOrNullabilityBuffer;
        } else {
            this.nullabilityBuffer = sizeOrNullabilityBuffer;
            this._size = sizeOrNullabilityBuffer.size();
        }
    }

    getValue(index: number): K | null {
        return this.nullabilityBuffer && !this.nullabilityBuffer.get(index) ? null : this.getValueFromBuffer(index);
    }

    has(index: number): boolean {
        return (this.nullabilityBuffer && this.nullabilityBuffer.get(index)) || !this.nullabilityBuffer;
    }

    get name(): string {
        return this._name;
    }

    get size(): number {
        return this._size;
    }

    presentValues(): SelectionVector {
        const selectionVector = [];
        for (let i = 0; i < this.size; i++) {
            if (this.has(i)) {
                selectionVector.push(i);
            }
        }
        return new FlatSelectionVector(selectionVector);
    }

    presentValuesSelected(selectionVector: SelectionVector): SelectionVector {
        let limit = 0;
        const vector = selectionVector.selectionValues();
        for (let i = 0; i < selectionVector.limit; i++) {
            const index = vector[i];
            if (this.has(index)) {
                vector[limit++] = index;
            }
        }

        selectionVector.setLimit(limit);
        return selectionVector;
    }

    nullableValues(): SelectionVector {
        const selectionVector = [];
        for (let i = 0; i < this.size; i++) {
            if (!this.has(i)) {
                selectionVector.push(i);
            }
        }
        return new FlatSelectionVector(selectionVector);
    }

    nullableValuesSelected(selectionVector: SelectionVector): SelectionVector {
        let limit = 0;
        const vector = selectionVector.selectionValues();
        for (let i = 0; i < selectionVector.limit; i++) {
            const index = vector[i];
            if (!this.has(index)) {
                vector[limit++] = index;
            }
        }

        selectionVector.setLimit(limit);
        return selectionVector;
    }
}
