import type BitVector from "./flat/bitVector";
import { type SelectionVector } from "./filter/selectionVector";
import {
    presentValues,
    presentValuesSelected,
    nullableValues,
    nullableValuesSelected,
    filter,
    filterSelected,
    filterNotEqual,
    filterNotEqualSelected,
    match,
    matchSelected,
    noneMatch,
    noneMatchSelected
} from "./utils";

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
        }
        return this.nullabilityBuffer ? this.nullabilityBuffer.get(index) : true;
    }

    get name(): string {
        return this._name;
    }

    get size(): number {
        return this._size;
    }

    protected abstract getValueFromBuffer(index: number): K;

    presentValues(): SelectionVector {
        return presentValues(this);
    }

    presentValuesSelected(selectionVector: SelectionVector): SelectionVector {
        return presentValuesSelected(this, selectionVector);
    }

    nullableValues(): SelectionVector {
        return nullableValues(this);
    }

    nullableValuesSelected(selectionVector: SelectionVector): SelectionVector {
        return nullableValuesSelected(this, selectionVector);
    }

    filter(value: K): SelectionVector {
        return filter(this, value);
    }

    filterSelected(value: K, selectionVector: SelectionVector): void {
        filterSelected(this, value, selectionVector);
    }

    filterNotEqual(value: K): SelectionVector {
        return filterNotEqual(this, value);
    }

    filterNotEqualSelected(value: K, selectionVector: SelectionVector): void {
        filterNotEqualSelected(this, value, selectionVector);
    }

    match(values: K[]): SelectionVector {
        return match(this, values);
    }

    matchSelected(values: K[], selectionVector: SelectionVector): void {
        matchSelected(this, values, selectionVector);
    }

    noneMatch(values: K[]): SelectionVector {
        return noneMatch(this, values);
    }

    noneMatchSelected(values: K[], selectionVector: SelectionVector): void {
        noneMatchSelected(this, values, selectionVector);
    }
}
