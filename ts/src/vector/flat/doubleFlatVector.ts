import { FixedSizeVector } from "../fixedSizeVector";
import type BitVector from "./bitVector";
import { type SelectionVector } from "../filter/selectionVector";
import { FlatSelectionVector } from "../filter/flatSelectionVector";
import { IntVector } from "../intVector";

export class DoubleFlatVector extends FixedSizeVector<Float64Array, number> {
    protected getValueFromBuffer(index: number): number {
        return this.dataBuffer[index];
    }

    filter(testValue: number): SelectionVector {
        const selectionVector = [];
        for (let i = 0; i < this.dataBuffer.length; i++) {
            if ((!this.nullabilityBuffer || this.nullabilityBuffer.get(i)) && this.dataBuffer[i] === testValue) {
                selectionVector.push(i);
            }
        }

        return new FlatSelectionVector(selectionVector);
    }

    match(testValues: number[]): SelectionVector {
        const selectionVector = [];
        for (let i = 0; i < this.dataBuffer.length; i++) {
            for (let j = 0; j < testValues.length; j++) {
                if (
                    (!this.nullabilityBuffer || this.nullabilityBuffer.get(i)) &&
                    this.dataBuffer[i] === testValues[j]
                ) {
                    selectionVector.push(i);
                }
            }
        }

        return new FlatSelectionVector(selectionVector);
    }

    filterSelected(testValue: number, selectionVector: SelectionVector): void {
        let limit = 0;
        const vector = selectionVector.selectionValues();
        for (let i = 0; i < selectionVector.limit; i++) {
            const index = vector[i];
            if (
                (!this.nullabilityBuffer || this.nullabilityBuffer.get(index)) &&
                this.dataBuffer[index] === testValue
            ) {
                vector[limit++] = index;
            }
        }

        selectionVector.setLimit(limit);
    }

    matchSelected(testValues: number[], selectionVector: SelectionVector): void {
        let limit = 0;
        const vector = selectionVector.selectionValues();
        for (let i = 0; i < selectionVector.limit; i++) {
            const index = vector[i];
            if (
                (!this.nullabilityBuffer || this.nullabilityBuffer.get(index)) &&
                testValues.includes(this.dataBuffer[index])
            ) {
                vector[limit++] = index;
            }
        }

        selectionVector.setLimit(limit);
    }

    greaterThanOrEqualTo(value: number): SelectionVector {
        const selectionVector = [];
        for (let i = 0; i < this.dataBuffer.length; i++) {
            if ((!this.nullabilityBuffer || this.nullabilityBuffer.get(i)) && this.dataBuffer[i] >= value) {
                selectionVector.push(i);
            }
        }

        return new FlatSelectionVector(selectionVector);
    }

    greaterThanOrEqualToSelected(testValue: number, selectionVector: SelectionVector): void {
        let limit = 0;
        const vector = selectionVector.selectionValues();
        for (let i = 0; i < selectionVector.limit; i++) {
            const index = vector[i];
            if ((!this.nullabilityBuffer || this.nullabilityBuffer.get(index)) && this.dataBuffer[index] >= testValue) {
                vector[limit++] = index;
            }
        }

        selectionVector.setLimit(limit);
    }

    smallerThanOrEqualTo(value: number): SelectionVector {
        const selectionVector = [];
        for (let i = 0; i < this.dataBuffer.length; i++) {
            if ((!this.nullabilityBuffer || this.nullabilityBuffer.get(i)) && this.dataBuffer[i] <= value) {
                selectionVector.push(i);
            }
        }

        return new FlatSelectionVector(selectionVector);
    }

    smallerThanOrEqualToSelected(testValue: number, selectionVector: SelectionVector): void {
        let limit = 0;
        const vector = selectionVector.selectionValues();
        for (let i = 0; i < selectionVector.limit; i++) {
            const index = vector[i];
            if ((!this.nullabilityBuffer || this.nullabilityBuffer.get(index)) && this.dataBuffer[index] <= testValue) {
                vector[limit++] = index;
            }
        }

        selectionVector.setLimit(limit);
    }

    noneMatch(values: number[]): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    noneMatchSelected(values: number[], selectionVector: SelectionVector): void {
        throw new Error("Not implemented yet.");
    }

    filterNotEqual(value: number): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    filterNotEqualSelected(value: number, selectionVector: SelectionVector): void {
        throw new Error("Not implemented yet.");
    }
}
