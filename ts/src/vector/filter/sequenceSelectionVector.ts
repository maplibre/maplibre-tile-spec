import { type SelectionVector } from "./selectionVector";

// Memory-efficient selection vector for sequential ranges (e.g., [0,1,2,3,...,n])
// Stores only baseValue, delta, and size instead of materializing the full array
export class SequenceSelectionVector implements SelectionVector {
    private _baseValue: number;
    private _delta: number;
    private _limit: number;
    private _capacity: number;
    private _materializedArray: number[] | null = null;

    constructor(baseValue: number, delta: number, size: number) {
        this._baseValue = baseValue;
        this._delta = delta;
        this._limit = size;
        this._capacity = size;
    }
    get limit(): any {
        return this._limit;
    }
    get capacity(): any {
        return this._capacity;
    }

    selectionValues(): number[] {
        if (!this._materializedArray) {
            this._materializedArray = this.materialize();
        }
        return this._materializedArray;
    }
    private materialize(): number[] {
        const arr = new Array<number>(this._capacity);
        for (let i = 0; i < this._capacity; i++) {
            arr[i] = this._baseValue + i * this._delta;
        }
        return arr;
    }

    getIndex(index: number): number {
        if (index >= this._limit || index < 0) {
            throw new RangeError("Index out of bounds");
        }
        if (this._materializedArray) {
            return this._materializedArray[index];
        }
        return this._baseValue + index * this._delta;
    }
    setIndex(index: number, value: number): void {
        if (index >= this._limit || index < 0) {
            throw new RangeError("Index out of bounds");
        }
        if (!this._materializedArray) {
            this._materializedArray = this.materialize();
        }
        this._materializedArray[index] = value;
    }

    setLimit(limit: number): void {
        this._limit = limit;
    }
}
