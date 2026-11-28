import { FixedSizeVector } from "../fixedSizeVector.js";

export class DoubleFlatVector extends FixedSizeVector<Float64Array, number> {
    protected getValueFromBuffer(index: number): number {
        return this.dataBuffer[index];
    }
}
