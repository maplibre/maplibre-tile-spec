import { type SelectionVector } from "../filter/selectionVector";
import { SequenceVector } from "./sequenceVector";
import { FlatSelectionVector } from "../filter/flatSelectionVector";

export class IntSequenceVector extends SequenceVector<Int32Array, number> {
    public constructor(name: string, baseValue: number, delta: number, size: number) {
        super(name, Int32Array.of(baseValue), delta, size);
    }
    protected getValueFromBuffer(index: number): number {
        return this.dataBuffer[0] + index * this.delta;
    }
    filter(value: number): SelectionVector {
        const index = (value - this.dataBuffer[0]) / this.delta;
        const sequenceValue = this.dataBuffer[0] + index * this.delta;
        if (value === sequenceValue) {
            return new FlatSelectionVector([index]);
        }

        const vector = sequenceValue ? [index] : [];
        return new FlatSelectionVector(vector);
    }

    filterNotEqual(value: number): SelectionVector {
        throw new Error("Method 'filterNotEqual' not implemented.");
    }

    match(values: number[]): SelectionVector {
        throw new Error("Method 'match' not implemented.");
    }

    noneMatch(values: number[]): SelectionVector {
        throw new Error("Method 'noneMatch' not implemented.");
    }

    filterSelected(value: number, selectionVector: SelectionVector): void {
        throw new Error("Method 'filterSelected' not implemented.");
    }

    filterNotEqualSelected(value: number, selectionVector: SelectionVector): void {
        throw new Error("Method 'filterNotEqualSelected' not implemented.");
    }

    matchSelected(values: number[], selectionVector: SelectionVector): void {
        throw new Error("Method 'matchSelected' not implemented.");
    }

    noneMatchSelected(values: number[], selectionVector: SelectionVector): void {
        throw new Error("Method 'noneMatchSelected' not implemented.");
    }

    greaterThanOrEqualTo(value: number): SelectionVector {
        throw new Error("Method 'greaterThanOrEqualTo' not implemented.");
    }

    smallerThanOrEqualTo(value: number): SelectionVector {
        throw new Error("Method 'smallerThanOrEqualTo' not implemented.");
    }

    greaterThanOrEqualToSelected(value: number, selectionVector: SelectionVector): void {
        throw new Error("Method 'greaterThanOrEqualToSelected' not implemented.");
    }

    smallerThanOrEqualToSelected(value: number, selectionVector: SelectionVector): void {
        throw new Error("Method 'smallerThanOrEqualToSelected' not implemented.");
    }
}
