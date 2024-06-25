import {ConstVector} from "./constVector";
import BitVector from "../flat/bitVector";
import {SelectionVector} from "../filter/selectionVector";
import {FlatSelectionVector} from "../filter/flatSelectionVector";

export class LongConstVector extends ConstVector<BigInt64Array, bigint> {

    public constructor (name: string, value: bigint, sizeOrNullabilityBuffer : number | BitVector) {
        super(name, BigInt64Array.of(value), sizeOrNullabilityBuffer);
    }

    filter(value: bigint): SelectionVector{
        const vectorValue = this.dataBuffer[0];
        if(vectorValue !== value){
            return new FlatSelectionVector([]);
        }

        return this.createSelectionVector();
    }

    match(values: bigint[]): SelectionVector {
        const vectorValue = this.dataBuffer[0];
        if(!values.includes(vectorValue)){
            return new FlatSelectionVector([]);
        }

        return this.createSelectionVector();
    }

    filterSelected(value: bigint, selectionVector: SelectionVector): void {
        const vectorValue = this.dataBuffer[0];
        if(vectorValue !== value){
            selectionVector.setLimit(0);
            return;
        }

        this.updateSelectionVector(selectionVector);
    }

    matchSelected(values: bigint[], selectionVector: SelectionVector): void {
        const vectorValue = this.dataBuffer[0];
        if(!values.includes(vectorValue)){
            selectionVector.setLimit(0);
            return;
        }

        this.updateSelectionVector(selectionVector);
    }

    protected getValueFromBuffer(index: number): bigint{
        return this.dataBuffer[0];
    }

    greaterThanOrEqualTo(value: bigint): SelectionVector {
        const vectorValue = this.dataBuffer[0];
        if(vectorValue >= value){
            return this.createSelectionVector();
        }

        return new FlatSelectionVector([]);
    }

    greaterThanOrEqualToSelected(value: bigint, selectionVector: SelectionVector): void {
        const vectorValue = this.dataBuffer[0];
        if(vectorValue >= value){
            this.updateSelectionVector(selectionVector);
            return;
        }

        selectionVector.setLimit(0);
    }

    smallerThanOrEqualTo(value: bigint): SelectionVector {
        const vectorValue = this.dataBuffer[0];
        if(vectorValue <= value){
            return this.createSelectionVector();
        }

        return new FlatSelectionVector([]);
    }

    smallerThanOrEqualToSelected(value: bigint, selectionVector: SelectionVector): void {
        const vectorValue = this.dataBuffer[0];
        if(vectorValue <= value){
            this.updateSelectionVector(selectionVector);
            return;
        }

        selectionVector.setLimit(0);
    }

    filterNotEqual(value: bigint): SelectionVector {
        const vectorValue = this.dataBuffer[0];
        if(vectorValue != value){
            return this.createSelectionVector();
        }

        return new FlatSelectionVector([]);
    }

    filterNotEqualSelected(value: bigint, selectionVector: SelectionVector): void {
        const vectorValue = this.dataBuffer[0];
        if(vectorValue != value){
            this.updateSelectionVector(selectionVector);
            return;
        }

        selectionVector.setLimit(0);
    }

    noneMatch(values: bigint[]): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    noneMatchSelected(values: bigint[], selectionVector: SelectionVector): void {
        throw new Error("Not implemented yet.");
    }

}


