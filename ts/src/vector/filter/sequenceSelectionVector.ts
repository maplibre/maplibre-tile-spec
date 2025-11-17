import { type SelectionVector } from "./selectionVector";

/**
 * Memory-efficient SelectionVector for arithmetic sequences (base + index * delta).
 * Calculates values on-demand, only materializes when modified.
 */
export class SequenceSelectionVector implements SelectionVector {
    private _materializedArray: number[] | null = null;

    constructor(
        private readonly _baseValue: number,
        private readonly _delta: number,
        private _limit: number,
        private readonly _capacity: number = _limit,
    ) {}

    /** @inheritdoc */
    get limit(): number {
        return this._limit;
    }

    /** @inheritdoc */
    get capacity(): number {
        return this._capacity;
    }

    /** @inheritdoc */
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

    /** @inheritdoc */
    getIndex(index: number): number {
        if (index >= this._limit || index < 0) {
            throw new RangeError("Index out of bounds");
        }
        if (this._materializedArray) {
            return this._materializedArray[index];
        }
        return this._baseValue + index * this._delta;
    }

    /** @inheritdoc */
    setIndex(index: number, value: number): void {
        if (index >= this._limit || index < 0) {
            throw new RangeError("Index out of bounds");
        }
        if (!this._materializedArray) {
            this._materializedArray = this.materialize();
        }
        this._materializedArray[index] = value;
    }

    /** @inheritdoc */
    setLimit(limit: number): void {
        if(limit < 0 || limit > this.capacity) {
            throw new RangeError("Limit out of bounds");
        }
        this._limit = limit;
    }
}
