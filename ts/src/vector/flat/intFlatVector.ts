import { FixedSizeVector } from "../fixedSizeVector";

export class IntFlatVector extends FixedSizeVector<Int32Array, number> {
    protected getValueFromBuffer(index: number): number {
        return this.dataBuffer[index];
    }
}
