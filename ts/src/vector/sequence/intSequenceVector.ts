import { SequenceVector } from "./sequenceVector";
import { type SelectionVector } from "../filter/selectionVector";
import { FlatSelectionVector } from "../filter/flatSelectionVector";

export class IntSequenceVector extends SequenceVector<Int32Array, number> {
    public constructor(name: string, baseValue: number, delta: number, size: number) {
        super(name, Int32Array.of(baseValue), delta, size);
    }

    filter(value: number): SelectionVector {
        const index = (value - this.dataBuffer[0]) / this.delta;
        if (Number.isInteger(index) && index >= 0 && index < this.size) {
            const sequenceValue = this.dataBuffer[0] + index * this.delta;
            if (value === sequenceValue) {
                return new FlatSelectionVector([index]);
            }
        }
        return new FlatSelectionVector([]);
    }

    match(values: number[]): SelectionVector {
        /*const baseValue = this.dataBuffer[0];
        const sequenceValues = new Array(values.length);
        let i = 0;
        for(const value of sequenceValues){
            const index = (value - baseValue) / this.delta;
            const sequenceValue = baseValue + index * this.delta;
            sequenceValues[i++] = sequenceValue;
        }

        if(value ===  sequenceValue){
            return new FlatSelectionVector([index])
        }

        const vector = sequenceValue? [index] : [];
        return new FlatSelectionVector(vector);*/
        throw new Error("Not implemented yet.");
    }

    filterSelected(value: number, selectionVector: SelectionVector): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    matchSelected(values: number[], selectionVector: SelectionVector): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    protected getValueFromBuffer(index: number): number {
        return this.dataBuffer[0] + index * this.delta;
    }

    filterNotEqual(value: number): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    filterNotEqualSelected(value: number, selectionVector: SelectionVector): void {
        throw new Error("Not implemented yet.");
    }

    greaterThanOrEqualTo(value: number): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    greaterThanOrEqualToSelected(value: number, selectionVector: SelectionVector): void {
        throw new Error("Not implemented yet.");
    }

    noneMatch(values: number[]): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    noneMatchSelected(values: number[], selectionVector: SelectionVector): void {
        throw new Error("Not implemented yet.");
    }

    smallerThanOrEqualTo(value: number): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    smallerThanOrEqualToSelected(value: number, selectionVector: SelectionVector): void {
        throw new Error("Not implemented yet.");
    }
}
