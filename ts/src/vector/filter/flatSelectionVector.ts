import { type SelectionVector } from "./selectionVector";

/**
 * Array-based SelectionVector for non-sequential selections.
 * Stores indices explicitly, suitable for irregular patterns and frequent modifications.
 */
export class FlatSelectionVector implements SelectionVector {
    /**
     * @param _selectionVector
     * @param _limit In write mode the limit of a Buffer is the limit of how much data you can write into the buffer.
     * In write mode the limit is equal to the capacity of the Buffer.
     */
    constructor(
        private _selectionVector: number[],
        private _limit?: number,
    ) {
        if (!this._limit) {
            this._limit = this._selectionVector.length;
        }
    }

    /** @inheritdoc */
    getIndex(index: number): number {
        if (index >= this._limit || index < 0) {
            throw new RangeError("Index out of bounds");
        }

        return this._selectionVector[index];
    }

    /** @inheritdoc */
    setIndex(index: number, value: number): void {
        if (index >= this._limit || index < 0) {
            throw new RangeError("Index out of bounds");
        }

        this._selectionVector[index] = value;
    }

    /** @inheritdoc */
    setLimit(limit: number): void {
        this._limit = limit;
    }

    /** @inheritdoc */
    selectionValues(): number[] {
        return this._selectionVector;
    }

    /** @inheritdoc */
    get capacity() {
        return this._selectionVector.length;
    }

    /** @inheritdoc */
    get limit() {
        return this._limit;
    }
}
