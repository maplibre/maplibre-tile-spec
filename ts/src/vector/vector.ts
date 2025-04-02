import BitVector from "./flat/bitVector";
import {SelectionVector} from "./filter/selectionVector";

export default abstract class Vector<T extends ArrayBuffer = ArrayBuffer, K = unknown> {
    protected nullabilityBuffer: BitVector | null;
    protected _size: number;

    /*protected constructor(name: string, dataBuffer: T, size: number);
    protected constructor(name: string, dataBuffer: T, nullabilityBuffer: BitVector);*/
    protected constructor(private readonly _name: string, protected readonly dataBuffer: T, sizeOrNullabilityBuffer : number | BitVector) {
        if(typeof sizeOrNullabilityBuffer === "number"){
            this._size = sizeOrNullabilityBuffer;
        }
        else{
            this.nullabilityBuffer = sizeOrNullabilityBuffer;
            this._size = sizeOrNullabilityBuffer.size();
        }
    }

    getValue(index: number): K | null {
        return (this.nullabilityBuffer && !this.nullabilityBuffer.get(index))? null :
            this.getValueFromBuffer(index);
    }

    get name(): string {
        return this._name;
    }

    get size(): number {
        return this._size;
    }

    protected abstract getValueFromBuffer(index: number): K;

    abstract filter(value: K): SelectionVector;

    abstract filterNotEqual(value: K): SelectionVector;

    abstract match(values: K[]): SelectionVector;

    abstract noneMatch(values: K[]): SelectionVector;

    /* updates the values and limit of the existing SelectionVector in-place */
    abstract filterSelected(value: K, selectionVector: SelectionVector): void;

    abstract filterNotEqualSelected(value: K, selectionVector: SelectionVector): void;

    /* updates the values and limit of the existing SelectionVector in-place */
    abstract matchSelected(values: K[], selectionVector: SelectionVector): void;

    abstract noneMatchSelected(values: K[], selectionVector: SelectionVector): void;

    abstract greaterThanOrEqualTo(value: K): SelectionVector;

    abstract smallerThanOrEqualTo(value: K): SelectionVector;

    abstract greaterThanOrEqualToSelected(value: K, selectionVector: SelectionVector): void;

    abstract smallerThanOrEqualToSelected(value: K, selectionVector: SelectionVector): void;
}
