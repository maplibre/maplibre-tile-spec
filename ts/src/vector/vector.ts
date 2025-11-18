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
        if (index < 0 || index >= this._size) {
            throw new RangeError("Index out of bounds");
        }
        return this.nullabilityBuffer && !this.nullabilityBuffer.get(index) ? null : this.getValueFromBuffer(index);
    }

    has(index: number): boolean {
        if (index < 0 || index >= this._size) {
            return false;
        } if (!this.nullabilityBuffer) {
            return true;
        }
        return this.nullabilityBuffer.get(index);
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

    protected abstract getValueFromBuffer(index: number): K;

    filter(value: K): SelectionVector {
        const selectionVector = [];
        for (let i = 0; i < this._size; i++) {
            if (this.has(i) && this.getValue(i) === value) {
                selectionVector.push(i);
            }
        }
        return new FlatSelectionVector(selectionVector);
    }

    filterNotEqual(value: K): SelectionVector {
        const selectionVector = [];
        for (let i = 0; i < this._size; i++) {
            if (!this.has(i) || this.getValue(i) !== value) {
                selectionVector.push(i);
            }
        }
        return new FlatSelectionVector(selectionVector);
    }

    match(values: K[]): SelectionVector {
        const selectionVector = [];
        for (let i = 0; i < this._size; i++) {
            if (!this.has(i)) continue;
            const value = this.getValue(i);
            const matchCount = values.filter(v => v === value).length;
            selectionVector.push(...Array(matchCount).fill(i));
        }
        return new FlatSelectionVector(selectionVector);
    }


    noneMatch(values: K[]): SelectionVector {
        const selectionVector = [];
        for (let i = 0; i < this._size; i++) {
            if (this.has(i) && !values.includes(this.getValue(i))) {
                selectionVector.push(i);
            }
        }
        return new FlatSelectionVector(selectionVector);
    }

    /* updates the values and limit of the existing SelectionVector in-place */
    filterSelected(value: K, selectionVector: SelectionVector): void {
        let limit = 0;
        const vector = selectionVector.selectionValues();
        for (let i = 0; i < selectionVector.limit; i++) {
            const index = vector[i];
            if (this.has(index) && this.getValue(index) === value) {
                vector[limit++] = index;
            }
        }
        selectionVector.setLimit(limit);
    }

    filterNotEqualSelected(value: K, selectionVector: SelectionVector): void {
        let limit = 0;
        const vector = selectionVector.selectionValues();
        for (let i = 0; i < selectionVector.limit; i++) {
            const index = vector[i];
            if (!this.has(index) || this.getValue(index) !== value) {
                vector[limit++] = index;
            }
        }
        selectionVector.setLimit(limit);
    }

    /* updates the values and limit of the existing SelectionVector in-place */
    matchSelected(values: K[], selectionVector: SelectionVector): void {
        let limit = 0;
        const vector = selectionVector.selectionValues();
        for (let i = 0; i < selectionVector.limit; i++) {
            const index = vector[i];
            if (!this.has(index)) continue;
            const value = this.getValue(index);
            const matchCount = values.filter(v => v === value).length;
            for (let k = 0; k < matchCount; k++) {
                vector[limit++] = index;
            }
        }
        selectionVector.setLimit(limit);
    }

    noneMatchSelected(values: K[], selectionVector: SelectionVector): void {
        let limit = 0;
        const vector = selectionVector.selectionValues();
        for (let i = 0; i < selectionVector.limit; i++) {
            const index = vector[i];
            if (this.has(index) && !values.includes(this.getValue(index))) {
                vector[limit++] = index;
            }
        }
        selectionVector.setLimit(limit);
    }

    greaterThanOrEqualTo(value: K): SelectionVector {
        const selectionVector = [];
        for (let i = 0; i < this._size; i++) {
            if (this.has(i) && this.getValue(i) >= value) {
                selectionVector.push(i);
            }
        }
        return new FlatSelectionVector(selectionVector);
    }

    smallerThanOrEqualTo(value: K): SelectionVector {
        const selectionVector = [];
        for (let i = 0; i < this._size; i++) {
            if (this.has(i) && this.getValue(i) <= value) {
                selectionVector.push(i);
            }
        }
        return new FlatSelectionVector(selectionVector);
    }

    greaterThanOrEqualToSelected(value: K, selectionVector: SelectionVector): void {
        let limit = 0;
        const vector = selectionVector.selectionValues();
        for (let i = 0; i < selectionVector.limit; i++) {
            const index = vector[i];
            if (this.has(index) && this.getValue(index) >= value) {
                vector[limit++] = index;
            }
        }
        selectionVector.setLimit(limit);
    }

    smallerThanOrEqualToSelected(value: K, selectionVector: SelectionVector): void {
        let limit = 0;
        const vector = selectionVector.selectionValues();
        for (let i = 0; i < selectionVector.limit; i++) {
            const index = vector[i];
            if (this.has(index) && this.getValue(index) <= value) {
                vector[limit++] = index;
            }
        }
        selectionVector.setLimit(limit);
    }
}
