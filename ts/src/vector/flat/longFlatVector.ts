import { FixedSizeVector } from "../fixedSizeVector";

export class LongFlatVector extends FixedSizeVector<BigInt64Array | BigUint64Array, bigint> {
    protected getValueFromBuffer(index: number): bigint {
        return this.dataBuffer[index];
    }
}
