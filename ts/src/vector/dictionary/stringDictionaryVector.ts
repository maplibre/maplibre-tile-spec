import {VariableSizeVector} from "../variableSizeVector";
import BitVector from "../flat/bitVector";
import {decodeString} from "../../encodings/decodingUtils";
import {SelectionVector} from "../filter/selectionVector";
import {FlatSelectionVector} from "../filter/flatSelectionVector";
import {createSelectionVector} from "../filter/selectionVectorUtils";


export class StringDictionaryVector extends VariableSizeVector<Uint8Array, string>{
    private readonly textEncoder: TextEncoder;

    constructor(name: string, private readonly indexBuffer: Int32Array, offsetBuffer: Int32Array, dictionaryBuffer: Uint8Array,
                       nullabilityBuffer?: BitVector) {
        super(name, offsetBuffer, dictionaryBuffer, nullabilityBuffer ?? indexBuffer.length);
        this.indexBuffer = indexBuffer;
        this.textEncoder = new TextEncoder();
    }

    protected getValueFromBuffer(index: number): string {
        const offset = this.indexBuffer[index];
        const start = this.offsetBuffer[offset];
        const end = this.offsetBuffer[offset + 1];
        return decodeString(this.dataBuffer, start, end);
    }

    filter(testValue: string): SelectionVector {
        const selectionVector = [];
        const testValueUtf8 = this.textEncoder.encode(testValue);
        const testValueDictionaryIndex = this.findDictionaryIndex(testValueUtf8);

        if(testValueDictionaryIndex === -1){
            return new FlatSelectionVector([]);
        }

        for(let i = 0; i < this.indexBuffer.length; i++){
            if((!this.nullabilityBuffer || (this.nullabilityBuffer.get(i))) && this.indexBuffer[i]  === testValueDictionaryIndex){
                selectionVector.push(i);
            }
        }

        return new FlatSelectionVector(selectionVector);
    }

    match(testValues: string[]): SelectionVector {
        const selectionVector = [];
        const testValuesDictionaryIndices = testValues.map(v => this.findDictionaryIndex(this.textEncoder.encode(v)))
            .filter(i => i !== -1);

        if(testValuesDictionaryIndices.length === 0){
            return new FlatSelectionVector([]);
        }

        //TODO: sort and use binary search?
        for(let i = 0; i < this.size; i++){
            const valueDictionaryIndex = this.indexBuffer[i];
            if((!this.nullabilityBuffer || (this.nullabilityBuffer.get(i)))
                && testValuesDictionaryIndices.includes(valueDictionaryIndex)){
                selectionVector.push(i);
            }
        }

        return new FlatSelectionVector(selectionVector);
    }

    filterSelected(testValue: string, selectionVector: SelectionVector): void {
        const testValueUtf8 = this.textEncoder.encode(testValue);
        const testValueDictionaryIndex = this.findDictionaryIndex(testValueUtf8);

        if(testValueDictionaryIndex === -1){
            selectionVector.setLimit(0);
            return;
        }

        const vector = selectionVector.selectionValues();
        let limit = 0;
        for(let i = 0; i < selectionVector.limit; i++){
            const featureIndex = vector[i];
            if((!this.nullabilityBuffer || (this.nullabilityBuffer.get(featureIndex))) &&
                this.indexBuffer[featureIndex] === testValueDictionaryIndex){
                vector[limit++] = featureIndex;
            }
        }

        selectionVector.setLimit(limit);
    }

    matchSelected(testValues: string[], selectionVector: SelectionVector): void{
        const testValuesDictionaryIndices = testValues.map(v =>
            this.findDictionaryIndex(this.textEncoder.encode(v))).filter(i => i !== -1);

        if(testValuesDictionaryIndices.length === 0){
            selectionVector.setLimit(0);
            return;
        }

        //TODO: sort and use binary search?
        const vector = selectionVector.selectionValues();
        let limit = 0;
        for(let i = 0; i < selectionVector.limit; i++){
            const featureIndex = vector[i];
            if((!this.nullabilityBuffer || (this.nullabilityBuffer.get(featureIndex))) &&
                testValuesDictionaryIndices.includes(this.indexBuffer[featureIndex])){
                vector[limit++] = featureIndex;
            }
        }

        selectionVector.setLimit(limit);
    }

    filterNotEqual(testValue: string): SelectionVector {
        const selectionVector = [];
        const testValueUtf8 = this.textEncoder.encode(testValue);
        const testValueDictionaryIndex = this.findDictionaryIndex(testValueUtf8);

        if(testValueDictionaryIndex === -1){
            return createSelectionVector(this.size);
        }

        for(let i = 0; i < this.indexBuffer.length; i++){
            if((this.nullabilityBuffer && !this.nullabilityBuffer.get(i)) || this.indexBuffer[i] !== testValueDictionaryIndex){
                selectionVector.push(i);
            }
        }

        return new FlatSelectionVector(selectionVector);
    }

    filterNotEqualSelected(testValue: string, selectionVector: SelectionVector): void {
        const testValueUtf8 = this.textEncoder.encode(testValue);
        const testValueDictionaryIndex = this.findDictionaryIndex(testValueUtf8);

        if(testValueDictionaryIndex === -1){
            return;
        }

        const vector = selectionVector.selectionValues();
        let limit = 0;
        for(let i = 0; i < selectionVector.limit; i++){
            const featureIndex = vector[i];
            if((this.nullabilityBuffer && !this.nullabilityBuffer.get(featureIndex)) || this.indexBuffer[featureIndex] !== testValueDictionaryIndex){
                vector[limit++] = featureIndex;
            }
        }

        selectionVector.setLimit(limit);
    }

    noneMatch(testValues: string[]): SelectionVector {
        const testValuesDictionaryIndices = testValues.map(v => this.findDictionaryIndex(this.textEncoder.encode(v)))
            .filter(i => i !== -1);

        if(testValuesDictionaryIndices.length === 0){
            return createSelectionVector(this.size);
        }

        //TODO: sort and use binary search?
        for(let i = 0; i < this.size; i++){
            const valueDictionaryIndex = this.indexBuffer[i];

            const selectionVector = [];
            if((this.nullabilityBuffer && !this.nullabilityBuffer.get(i)) || !testValuesDictionaryIndices.includes(valueDictionaryIndex)){
                selectionVector.push(i);
            }
        }

        return new FlatSelectionVector([]);
    }

    noneMatchSelected(testValues: string[], selectionVector: SelectionVector): void {
        const testValuesDictionaryIndices =
            testValues.map(v => this.findDictionaryIndex(this.textEncoder.encode(v))).filter(i => i !== -1);

        if(testValuesDictionaryIndices.length === 0){
            return;
        }

        //TODO: sort and use binary search?
        const vector = selectionVector.selectionValues();
        let limit = 0;
        for(let i = 0; i < selectionVector.limit; i++){
            const featureIndex = vector[i];
            if((this.nullabilityBuffer && !this.nullabilityBuffer.get(featureIndex)) || !testValuesDictionaryIndices.includes(this.indexBuffer[featureIndex])){
                vector[limit++] = featureIndex;
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

    private findDictionaryIndex(testValue: Uint8Array): number {
        const testValueLength = testValue.length;

        //TODO: use binary search if sorted
        for (let i = 1; i <= this.size; i++) {
            const valueLength = this.offsetBuffer[i] - this.offsetBuffer[i - 1];
            if (valueLength !== testValueLength) {
                continue;
            }

            const value = this.dataBuffer.subarray(this.offsetBuffer[i - 1], this.offsetBuffer[i]);
            //TODO: get rid of every
            if (value.every((val, idx) => val === testValue[idx])) {
                return i - 1;
            }
        }

        return -1;
    }

    /*filterSelected2(testValue: string, vector2: StringDictionaryVector, testValue2: string): number[] {
        const selectionVector = [];
        const testValueUtf8 = this.textEncoder.encode(testValue);
        const testValueDictionaryIndex = this.findDictionaryIndex(testValueUtf8);
        const testValue2Utf8 = this.textEncoder.encode(testValue2);
        const testValue2DictionaryIndex = vector2.findDictionaryIndex(testValue2Utf8);

        if(testValueDictionaryIndex === -1 || testValue2DictionaryIndex === -1){
            return [];
        }

        for(let i = 0; i < this.indexBuffer.length; i++){
            if(this.nullabilityBuffer.get(i) && this.indexBuffer[i]  === testValueDictionaryIndex &&
                vector2.nullabilityBuffer.get(i) && vector2.indexBuffer[i] === testValue2DictionaryIndex){
                selectionVector.push(i);
            }
        }

        return selectionVector;
    }*/

    /*filter(index: number, testValue: string): number[] {
        const selectionVector = [];
        const testValueUtf8 = this.textEncoder.encode(testValue);
        const testValueDictionaryIndex = this.findDictionaryIndex(testValueUtf8);

        if(testValueDictionaryIndex === -1){
            return [];
        }

        for(let i = 0; i < this.indexBuffer.length; i++){
            if(this.nullabilityBuffer.get(i) && this.indexBuffer[i]  === testValueDictionaryIndex){
                selectionVector.push(i);
            }
        }

        return selectionVector;
    }*/
}
