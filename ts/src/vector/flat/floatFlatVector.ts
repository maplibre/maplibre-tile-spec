import { FixedSizeVector } from "../fixedSizeVector";

export class FloatFlatVector extends FixedSizeVector<Float32Array, number> {
    protected getValueFromBuffer(index: number): number {
        return this.dataBuffer[index];
    }
}
