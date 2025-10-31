import { SequenceVector } from "./sequenceVector";

export class LongSequenceVector extends SequenceVector<BigInt64Array, bigint> {
    public constructor(name: string, baseValue: bigint, delta: bigint, size: number) {
        super(name, BigInt64Array.of(baseValue), delta, size);
    }

    protected getValueFromBuffer(index: number): bigint {
        return this.dataBuffer[0] + BigInt(index) * this.delta;
    }
}
