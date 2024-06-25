import {FixedSizeVector} from "../fixedSizeVector";
import BitVector from "./bitVector";
import {SelectionVector} from "../filter/selectionVector";
import {FlatSelectionVector} from "../filter/flatSelectionVector";

export class LongFlatVector extends FixedSizeVector<BigInt64Array, bigint>{

    constructor(
        name: string,
        dataBuffer: BigInt64Array,
        sizeOrNullabilityBuffer : number | BitVector){
        super(name, dataBuffer, sizeOrNullabilityBuffer);
    }

    protected getValueFromBuffer(index: number): bigint {
        return this.dataBuffer[index];
    }

    filter(value: bigint): SelectionVector {
        const selectionVector = [];
        for (let i = 0; i < this.dataBuffer.length; i++) {
            if ((!this.nullabilityBuffer || this.nullabilityBuffer.get(i)) && this.dataBuffer[i] === value) {
                selectionVector.push(i);
            }
        }

        return new FlatSelectionVector(selectionVector);
    }

    match(values: bigint[]): SelectionVector {
        const selectionVector = [];
        for (let i = 0; i < this.dataBuffer.length; i++) {
            for(let j = 0; j < values.length; j++){
                if ((!this.nullabilityBuffer || this.nullabilityBuffer.get(i)) && this.dataBuffer[i] === values[j]) {
                    selectionVector.push(i);
                }
            }
        }

        return new FlatSelectionVector(selectionVector);
    }

    //TODO: fix -> values in style are number not bigint
    filterSelected(testValue: bigint, selectionVector: SelectionVector): void {
        let limit = 0;
        const vector = selectionVector.selectionValues();
        for (let i = 0; i < selectionVector.limit; i++) {
            const index = vector[i];
            if ((!this.nullabilityBuffer || this.nullabilityBuffer.get(i)) && this.dataBuffer[index] === testValue) {
                vector[limit++] = index;
            }
        }

        selectionVector.setLimit(limit);
    }

    matchSelected(testValues: bigint[], selectionVector: SelectionVector): void {
        let limit = 0;
        const vector = selectionVector.selectionValues();
        for (let i = 0; i < selectionVector.limit; i++) {
            const index = vector[i];
            if ((!this.nullabilityBuffer || this.nullabilityBuffer.get(i)) && testValues.includes(this.dataBuffer[index])) {
                vector[limit++] = index;
            }
        }

        selectionVector.setLimit(limit);
    }

    greaterThanOrEqualTo(value: bigint): SelectionVector {
        const selectionVector = [];
        for (let i = 0; i < this.dataBuffer.length; i++) {
            if ((!this.nullabilityBuffer || this.nullabilityBuffer.get(i)) && this.dataBuffer[i] >= value) {
                selectionVector.push(i);
            }
        }

        return new FlatSelectionVector(selectionVector);
    }

    greaterThanOrEqualToSelected(testValue: bigint, selectionVector: SelectionVector): void {
        let limit = 0;
        const vector = selectionVector.selectionValues();
        for (let i = 0; i < selectionVector.limit; i++) {
            const index = vector[i];
            if ((!this.nullabilityBuffer || this.nullabilityBuffer.get(i)) && this.dataBuffer[index] >= testValue) {
                vector[limit++] = index;
            }
        }

        selectionVector.setLimit(limit);
    }

    smallerThanOrEqualTo(value: bigint): SelectionVector {
        const selectionVector = [];
        for (let i = 0; i < this.dataBuffer.length; i++) {
            if ((!this.nullabilityBuffer || this.nullabilityBuffer.get(i)) && this.dataBuffer[i] <= value) {
                selectionVector.push(i);
            }
        }

        return new FlatSelectionVector(selectionVector);
    }

    smallerThanOrEqualToSelected(testValue: bigint, selectionVector: SelectionVector): void {
        let limit = 0;
        const vector = selectionVector.selectionValues();
        for (let i = 0; i < selectionVector.limit; i++) {
            const index = vector[i];
            if ((!this.nullabilityBuffer || this.nullabilityBuffer.get(i)) && this.dataBuffer[index] <= testValue) {
                vector[limit++] = index;
            }
        }

        selectionVector.setLimit(limit);
    }

    filterNotEqual(value: bigint): SelectionVector {
        const selectionVector = [];
        for (let i = 0; i < this.dataBuffer.length; i++) {
            if ((!this.nullabilityBuffer || this.nullabilityBuffer.get(i)) && this.dataBuffer[i] !== value) {
                selectionVector.push(i);
            }
        }

        return new FlatSelectionVector(selectionVector);
    }

    filterNotEqualSelected(testValue: bigint, selectionVector: SelectionVector): void {
        let limit = 0;
        const vector = selectionVector.selectionValues();
        for (let i = 0; i < selectionVector.limit; i++) {
            const index = vector[i];
            if ((!this.nullabilityBuffer || this.nullabilityBuffer.get(i)) && this.dataBuffer[index] !== testValue) {
                vector[limit++] = index;
            }
        }

        selectionVector.setLimit(limit);
    }

    noneMatch(values: bigint[]): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    noneMatchSelected(values: bigint[], selectionVector: SelectionVector): void {
        throw new Error("Not implemented yet.");
    }

}

