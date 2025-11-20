import { VariableSizeVector } from "../variableSizeVector";
import type BitVector from "./bitVector";
import { decodeString } from "../../decoding/decodingUtils";
import { type SelectionVector } from "../filter/selectionVector";
import { FlatSelectionVector } from "../filter/flatSelectionVector";

export class StringFlatVector extends VariableSizeVector<Uint8Array, string> {
    private readonly textEncoder: TextEncoder;

    constructor(name: string, offsetBuffer: Int32Array, dataBuffer: Uint8Array, nullabilityBuffer?: BitVector) {
        super(name, offsetBuffer, dataBuffer, nullabilityBuffer ?? offsetBuffer.length - 1);
        this.textEncoder = new TextEncoder();
    }

    protected getValueFromBuffer(index: number): string {
        const start = this.offsetBuffer[index];
        const end = this.offsetBuffer[index + 1];
        return decodeString(this.dataBuffer, start, end);
    }

    filter(testValue: string): SelectionVector {
        const selectionVector = new Uint32Array(this._size);
        let index = 0;
        const predicateUtf8 = this.textEncoder.encode(testValue);

        for (let i = 0; i < this.size; i++) {
            const length = this.offsetBuffer[i + 1] - this.offsetBuffer[i];
            if (
                (!this.nullabilityBuffer || this.nullabilityBuffer.get(i)) &&
                length === predicateUtf8.length &&
                this.dataBuffer
                    .subarray(this.offsetBuffer[i], this.offsetBuffer[i + 1])
                    .every((val, idx) => val === predicateUtf8[idx])
            ) {
                selectionVector[index] = i;
                index++;
            }
        }

        return new FlatSelectionVector(selectionVector);
    }

    match(testValues: string[]): SelectionVector {
        const selectionVector = new Uint32Array(this._size);
        let index = 0;
        const numPredicates = testValues.length;
        const testValuesUtf8 = testValues.map((v) => this.textEncoder.encode(v));
        for (let i = 0; i < this.size; i++) {
            const valueLength = this.offsetBuffer[i + 1] - this.offsetBuffer[i];
            const valueUtf8 = this.dataBuffer.subarray(this.offsetBuffer[i], this.offsetBuffer[i + 1]);

            for (let j = 0; j < numPredicates; j++) {
                //TODO: get rid of every
                if (
                    (!this.nullabilityBuffer || this.nullabilityBuffer.get(i)) &&
                    valueLength === testValuesUtf8[j].length &&
                    valueUtf8.every((val, idx) => val === testValuesUtf8[j][idx])
                ) {
                    if(!selectionVector.includes(i)){
                        selectionVector[index] = i;
                        index++;
                    }
                }
            }
        }

        return new FlatSelectionVector(selectionVector);
    }

    filterSelected(value: string, selectionVector: SelectionVector): void {
        const predicateUtf8 = this.textEncoder.encode(value);
        const vector = selectionVector.selectionValues();
        let limit = 0;
        for (let i = 0; i < selectionVector.limit; i++) {
            const index = vector[i];
            const length = this.offsetBuffer[index + 1] - this.offsetBuffer[index];
            //TODO: get rid of every
            if (
                (!this.nullabilityBuffer || this.nullabilityBuffer.get(index)) &&
                length === predicateUtf8.length &&
                this.dataBuffer
                    .subarray(this.offsetBuffer[index], this.offsetBuffer[index + 1])
                    .every((val, idx) => val === predicateUtf8[idx])
            ) {
                vector[limit++] = index;
            }
        }
        selectionVector.setLimit(limit);
    }

    matchSelected(values: string[], selectionVector: SelectionVector): void {
        //TODO: get rid of map?
        const testValuesUtf8 = values.map((v) => this.textEncoder.encode(v));

        //TODO: sort and use binary search?
        const vector = selectionVector.selectionValues();
        let limit = 0;
        for (let i = 0; i < selectionVector.limit; i++) {
            const index = vector[i];
            const value = this.dataBuffer.subarray(this.offsetBuffer[index], this.offsetBuffer[index + 1]);
            for (const testValue of testValuesUtf8) {
                //TODO: get rid of every
                if (
                    (!this.nullabilityBuffer || this.nullabilityBuffer.get(index)) &&
                    value.length === testValue.length &&
                    value.every((val, idx) => val === testValue[idx])
                ) {
                    vector[limit++] = index;
                    break;
                }
            }
        }

        selectionVector.setLimit(limit);
    }

    filterNotEqual(testValue: string): SelectionVector {
        const selectionVector = new Uint32Array(this._size);
        let index = 0;
        const predicateUtf8 = this.textEncoder.encode(testValue);
        for (let i = 0; i < this.size; i++) {
            const length = this.offsetBuffer[i + 1] - this.offsetBuffer[i];
            //TODO: get rid of every
            if (
                (!this.nullabilityBuffer || this.nullabilityBuffer.get(i)) &&
                (length !== predicateUtf8.length ||
                    !this.dataBuffer
                        .subarray(this.offsetBuffer[i], this.offsetBuffer[i + 1])
                        .every((val, idx) => val === predicateUtf8[idx]))
            ) {
                selectionVector[index] = i;
                index++;
            }
        }

        return new FlatSelectionVector(selectionVector);
    }

    filterNotEqualSelected(value: string, selectionVector: SelectionVector): void {
        const predicateUtf8 = this.textEncoder.encode(value);
        const vector = selectionVector.selectionValues();
        let limit = 0;
        for (let i = 0; i < selectionVector.limit; i++) {
            const index = vector[i];
            const length = this.offsetBuffer[index + 1] - this.offsetBuffer[index];
            //TODO: get rid of every
            if (
                (!this.nullabilityBuffer || this.nullabilityBuffer.get(index)) &&
                (length !== predicateUtf8.length ||
                    !this.dataBuffer
                        .subarray(this.offsetBuffer[index], this.offsetBuffer[index + 1])
                        .every((val, idx) => val === predicateUtf8[idx]))
            ) {
                vector[limit++] = index;
            }
        }

        selectionVector.setLimit(limit);
    }

    noneMatch(testValues: string[]): SelectionVector {
        const selectionVector = new Uint32Array(this._size);
        let index = 0;
        const testValuesUtf8 = testValues.map((v) => this.textEncoder.encode(v));

        for (let i = 0; i < this.size; i++) {
            const valueLength = this.offsetBuffer[i + 1] - this.offsetBuffer[i];
            const valueUtf8 = this.dataBuffer.subarray(this.offsetBuffer[i], this.offsetBuffer[i + 1]);

            let matches = false;
            for (let j = 0; j < testValuesUtf8.length; j++) {
                if (
                    (!this.nullabilityBuffer || this.nullabilityBuffer.get(i)) &&
                    valueLength === testValuesUtf8[j].length &&
                    valueUtf8.every((val, idx) => val === testValuesUtf8[j][idx])
                ) {
                    matches = true;
                    break;
                }
            }

            if (!matches && (!this.nullabilityBuffer || this.nullabilityBuffer.get(i))) {
                selectionVector[index] = i;
                index++;
            }
        }

        return new FlatSelectionVector(selectionVector);
    }

    noneMatchSelected(values: string[], selectionVector: SelectionVector): void {
        //TODO: get rid of map?
        const testValuesUtf8 = values.map((v) => this.textEncoder.encode(v));

        //TODO: sort and use binary search?
        const vector = selectionVector.selectionValues();
        let limit = 0;
        for (let i = 0; i < selectionVector.limit; i++) {
            const index = vector[i];
            const value = this.dataBuffer.subarray(this.offsetBuffer[index], this.offsetBuffer[index + 1]);
            let matches = false;
            for (let j = 0; j < testValuesUtf8.length; j++) {
                //TODO: get rid of every
                if (
                    (!this.nullabilityBuffer || this.nullabilityBuffer.get(index)) &&
                    value.length === testValuesUtf8[j].length &&
                    value.every((val, idx) => val === testValuesUtf8[j][idx])
                ) {
                    matches = true;
                    break;
                }
            }
            if (!matches && (!this.nullabilityBuffer || this.nullabilityBuffer.get(index))) {
                vector[limit++] = index;
            }
        }

        selectionVector.setLimit(limit);
    }

    greaterThanOrEqualTo(value: string): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    greaterThanOrEqualToSelected(value: string, selectionVector: SelectionVector): void {
        throw new Error("Not implemented yet.");
    }

    smallerThanOrEqualTo(value: string): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    smallerThanOrEqualToSelected(value: string, selectionVector: SelectionVector): void {
        throw new Error("Not implemented yet.");
    }
}
