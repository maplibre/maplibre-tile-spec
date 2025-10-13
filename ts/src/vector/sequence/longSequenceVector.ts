import {SequenceVector} from "./sequenceVector";
import {SelectionVector} from "../filter/selectionVector";
import {FlatSelectionVector} from "../filter/flatSelectionVector";

export class LongSequenceVector extends SequenceVector<BigInt64Array, bigint> {

    public constructor(name: string, baseValue: bigint, delta: bigint, size: number) {
        super(name, BigInt64Array.of(baseValue), delta, size);
    }

    filter(value: bigint): SelectionVector {
        const index = (value - this.dataBuffer[0]) / this.delta;
        const sequenceValue = this.dataBuffer[0] + index * this.delta;
        if(value ===  sequenceValue){
            return new FlatSelectionVector([Number(index)])
        }

        const vector = sequenceValue? [Number(index)] : [];
        return new FlatSelectionVector(vector);
    }

    match(values: bigint[]): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    filterSelected(value: bigint, selectionVector: SelectionVector): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    matchSelected(values: bigint[], selectionVector: SelectionVector): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    protected getValueFromBuffer(index: number): bigint {
        return this.dataBuffer[0] + BigInt(index) * this.delta;
    }

    filterNotEqual(value: bigint): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    filterNotEqualSelected(value: bigint, selectionVector: SelectionVector): void {
        throw new Error("Not implemented yet.");
    }

    greaterThanOrEqualTo(value: bigint): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    greaterThanOrEqualToSelected(value: bigint, selectionVector: SelectionVector): void {
        throw new Error("Not implemented yet.");
    }

    noneMatch(values: bigint[]): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    noneMatchSelected(values: bigint[], selectionVector: SelectionVector): void {
        throw new Error("Not implemented yet.");
    }

    smallerThanOrEqualTo(value: bigint): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    smallerThanOrEqualToSelected(value: bigint, selectionVector: SelectionVector): void {
        throw new Error("Not implemented yet.");
    }

}
