import { FixedSizeVector } from "../fixedSizeVector";

export class Int32FlatVector extends FixedSizeVector<Int32Array | Uint32Array, number> {
    protected getValueFromBuffer(index: number): number {
        return this.dataBuffer[index];
    }
}
