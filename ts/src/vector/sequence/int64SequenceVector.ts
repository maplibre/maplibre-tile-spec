import { SequenceVector } from "./sequenceVector";

export class Int64SequenceVector extends SequenceVector<BigInt64Array | BigUint64Array, bigint> {
    public constructor(name: string, baseValue: bigint, delta: bigint, size: number, isSigned: boolean) {
        super(name, isSigned ? BigInt64Array.of(baseValue) : BigUint64Array.of(baseValue), delta, size);
    }

    protected getValueFromBuffer(index: number): bigint {
        return this.dataBuffer[0] + BigInt(index) * this.delta;
    }
}
