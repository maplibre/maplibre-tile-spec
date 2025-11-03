import { SequenceVector } from "./sequenceVector";

export class IntSequenceVector extends SequenceVector<Int32Array, number> {
    public constructor(name: string, baseValue: number, delta: number, size: number) {
        super(name, Int32Array.of(baseValue), delta, size);
    }
    protected getValueFromBuffer(index: number): number {
        return this.dataBuffer[0] + index * this.delta;
    }
}
