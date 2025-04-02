import {ConstVector} from "./constVector";
import BitVector from "../flat/bitVector";
import {SelectionVector} from "../filter/selectionVector";
import {FlatSelectionVector} from "../filter/flatSelectionVector";

export class IntConstVector extends ConstVector<Int32Array, number> {

    public constructor (name: string, value: number,  sizeOrNullabilityBuffer : number | BitVector) {
        super(name, Int32Array.of(value), sizeOrNullabilityBuffer);
    }

    filter(value: number): SelectionVector {
        //TODO: create also different SelectionVectors -> Const, Sequence and Flat
        const vectorValue = this.dataBuffer[0];
        if(vectorValue !== value){
            return new FlatSelectionVector([]);
        }

        return this.createSelectionVector();
    }

    match(values: number[]): SelectionVector {
        const vectorValue = this.dataBuffer[0];
        if(!values.includes(vectorValue)){
            return new FlatSelectionVector([]);
        }

        return this.createSelectionVector();
    }

    filterSelected(value: number, selectionVector: SelectionVector): void {
        const vectorValue = this.dataBuffer[0];
        if(vectorValue !== value){
            selectionVector.setLimit(0);
            return;
        }

        this.updateSelectionVector(selectionVector);
    }

    matchSelected(values: number[], selectionVector: SelectionVector): void {
        const vectorValue = this.dataBuffer[0];
        if(!values.includes(vectorValue)){
            selectionVector.setLimit(0);
            return;
        }

        this.updateSelectionVector(selectionVector);
    }

    protected getValueFromBuffer(index: number): number {
        return this.dataBuffer[0];
    }

    greaterThanOrEqualTo(testValue: number): SelectionVector {
        //TODO: handle bitVector?
        const vectorValue = this.dataBuffer[0];
        if(vectorValue >= testValue){
            return this.createSelectionVector();
        }

        return new FlatSelectionVector([]);

    }

    greaterThanOrEqualToSelected(value: number, selectionVector: SelectionVector): void {
        const vectorValue = this.dataBuffer[0];
        if(vectorValue >= value){
            this.updateSelectionVector(selectionVector);
            return;
        }

        selectionVector.setLimit(0);
    }

    smallerThanOrEqualTo(value: number): SelectionVector {
        const vectorValue = this.dataBuffer[0];
        if(vectorValue <= value){
            return this.createSelectionVector();
        }

        return new FlatSelectionVector([]);
    }

    smallerThanOrEqualToSelected(value: number, selectionVector: SelectionVector): void {
        const vectorValue = this.dataBuffer[0];
        if(vectorValue <= value){
            this.updateSelectionVector(selectionVector);
        }

        selectionVector.setLimit(0);
    }

    filterNotEqual(value: number): SelectionVector {
        const vectorValue = this.dataBuffer[0];
        if(vectorValue !== value){
            return this.createSelectionVector();
        }

        return new FlatSelectionVector([]);
    }

    filterNotEqualSelected(value: number, selectionVector: SelectionVector): void {
        const vectorValue = this.dataBuffer[0];
        if(vectorValue !== value){
            this.updateSelectionVector(selectionVector);
            return;
        }

        selectionVector.setLimit(0);
    }

    noneMatch(values: number[]): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    noneMatchSelected(values: number[], selectionVector: SelectionVector): void {
        throw new Error("Not implemented yet.");
    }

}
