import { type SelectionVector } from "../filter/selectionVector";
import { SequenceVector } from "./sequenceVector";
import { FlatSelectionVector } from "../filter/flatSelectionVector";

export class LongSequenceVector extends SequenceVector<BigInt64Array, bigint> {
    public constructor(name: string, baseValue: bigint, delta: bigint, size: number) {
        super(name, BigInt64Array.of(baseValue), delta, size);
    }

    protected getValueFromBuffer(index: number): bigint {
        return this.dataBuffer[0] + BigInt(index) * this.delta;
    }

    filter(value: bigint): SelectionVector {
        const index = (value - this.dataBuffer[0]) / this.delta;
        const sequenceValue = this.dataBuffer[0] + index * this.delta;
        if (value === sequenceValue) {
            return new FlatSelectionVector([Number(index)]);
        }

        const vector = sequenceValue ? [Number(index)] : [];
        return new FlatSelectionVector(vector);
    }

    filterNotEqual(value: bigint): SelectionVector {
        throw new Error("Method not implemented.");
    }
    match(values: bigint[]): SelectionVector {
        throw new Error("Method not implemented.");
    }
    noneMatch(values: bigint[]): SelectionVector {
        throw new Error("Method not implemented.");
    }
    filterSelected(value: bigint, selectionVector: SelectionVector): void {
        throw new Error("Method not implemented.");
    }
    filterNotEqualSelected(value: bigint, selectionVector: SelectionVector): void {
        throw new Error("Method not implemented.");
    }
    matchSelected(values: bigint[], selectionVector: SelectionVector): void {
        throw new Error("Method not implemented.");
    }
    noneMatchSelected(values: bigint[], selectionVector: SelectionVector): void {
        throw new Error("Method not implemented.");
    }
    greaterThanOrEqualTo(value: bigint): SelectionVector {
        throw new Error("Method not implemented.");
    }
    smallerThanOrEqualTo(value: bigint): SelectionVector {
        throw new Error("Method not implemented.");
    }
    greaterThanOrEqualToSelected(value: bigint, selectionVector: SelectionVector): void {
        throw new Error("Method not implemented.");
    }
    smallerThanOrEqualToSelected(value: bigint, selectionVector: SelectionVector): void {
        throw new Error("Method not implemented.");
    }
}
