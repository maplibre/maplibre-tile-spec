import { FixedSizeVector } from "../fixedSizeVector";
import { type SelectionVector } from "../filter/selectionVector";
import { FlatSelectionVector } from "../filter/flatSelectionVector";

export class IntFlatVector extends FixedSizeVector<Int32Array, number> {
    protected getValueFromBuffer(index: number): number {
        return this.dataBuffer[index];
    }

    filter(value: number): SelectionVector {
        const selectionVector = [];
        for (let i = 0; i < this.dataBuffer.length; i++) {
            if ((!this.nullabilityBuffer || this.nullabilityBuffer.get(i)) && this.getValue(i) === value) {
                selectionVector.push(i);
            }
        }

        return new FlatSelectionVector(selectionVector);
    }

    match(values: number[]): SelectionVector {
        const selectionVector = [];

        for (let i = 0; i < this.dataBuffer.length; i++) {
            for (let j = 0; j < values.length; j++) {
                if ((!this.nullabilityBuffer || this.nullabilityBuffer.get(i)) && this.getValue(i) === values[j]) {
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

    filterNotEqual(value: number): SelectionVector {
        const selectionVector = [];
        for (let i = 0; i < this.dataBuffer.length; i++) {
            if ((this.nullabilityBuffer && !this.nullabilityBuffer.get(i)) || this.dataBuffer[i] !== value) {
                selectionVector.push(i);
            }
        }

        return new FlatSelectionVector(selectionVector);
    }

    filterNotEqualSelected(testValue: number, selectionVector: SelectionVector): void {
        let limit = 0;
        const vector = selectionVector.selectionValues();
        for (let i = 0; i < selectionVector.limit; i++) {
            const index = vector[i];
            if ((this.nullabilityBuffer && !this.nullabilityBuffer.get(i)) || this.dataBuffer[index] !== testValue) {
                vector[limit++] = index;
            }
        }

        selectionVector.setLimit(limit);
    }
}
