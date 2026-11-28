import { FixedSizeVector } from "../fixedSizeVector.js";

export class Int64FlatVector extends FixedSizeVector<BigInt64Array | BigUint64Array, bigint> {
    protected getValueFromBuffer(index: number): bigint {
        return this.dataBuffer[index];
    }
}
