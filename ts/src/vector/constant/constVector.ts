import BitVector from "../flat/bitVector";
import Vector from "../vector";
import {SelectionVector} from "../filter/selectionVector";
import {FlatSelectionVector} from "../filter/flatSelectionVector";


export abstract class ConstVector<T extends ArrayBuffer, K> extends Vector<T, K> {

    protected constructor(name: string, buffer: T, sizeOrNullabilityBuffer: number | BitVector){
        super(name, buffer, sizeOrNullabilityBuffer);
    }

    protected updateSelectionVector(selectionVector: SelectionVector) {
        const buffer = selectionVector.selectionValues();
        let limit = 0;
        for (let i = 0; i < selectionVector.limit; i++) {
            if (!this.nullabilityBuffer || this.nullabilityBuffer.get(i)) {
                buffer[limit++] = buffer[i];
            }
        }

        selectionVector.setLimit(limit);
    }

    protected createSelectionVector() {
        const selectionVector = [];
        for (let i = 0; i < this.size; i++) {
            if (!this.nullabilityBuffer || this.nullabilityBuffer.get(i)) {
                selectionVector.push(i);
            }
        }

        return new FlatSelectionVector(selectionVector);
    }
}
