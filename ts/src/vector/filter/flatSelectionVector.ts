import {SelectionVector} from "./selectionVector";


export class FlatSelectionVector implements SelectionVector{
    /**
     * @param _limit In write mode the limit of a Buffer is the limit of how much data you can write into the buffer.
     * In write mode the limit is equal to the capacity of the Buffer.
     */
    constructor(private _selectionVector: number[], private _limit?: number){
        if(!this._limit){
            this._limit = this._selectionVector.length;
        }
    }

    getIndex(index: number): number {
        if(index >= this._limit){
            throw new Error("Index out of bounds");
        }

        return this._selectionVector[index];
    }

    setIndex(index: number, value: number): void {
        if(index >= this._limit){
            throw new Error("Index out of bounds");
        }

        this._selectionVector[index] = value;
    }

    setLimit(limit: number): void {
        this._limit = limit;
    }

    selectionValues(): number[]{
        return this._selectionVector;
    }

    get capacity() {
        return this._selectionVector.length;
    }

    get limit() {
        return this._limit;
    }

}