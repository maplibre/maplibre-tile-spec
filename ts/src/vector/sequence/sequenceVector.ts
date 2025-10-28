import Vector from "../vector";

export abstract class SequenceVector<T extends ArrayBufferView, K> extends Vector<T, K> {
    protected readonly delta: K;

    protected constructor(name: string, baseValueBuffer: T, delta: K, size: number) {
        super(name, baseValueBuffer, size);
        this.delta = delta;
    }
}
