import type BitVector from "./flat/bitVector";

export default abstract class Vector<T extends ArrayBufferView = ArrayBufferView, K = unknown> {
    protected nullabilityBuffer: BitVector | null;
    protected _size: number;

    constructor(
        private readonly _name: string,
        protected readonly dataBuffer: T,
        sizeOrNullabilityBuffer: number | BitVector,
    ) {
        if (typeof sizeOrNullabilityBuffer === "number") {
            this._size = sizeOrNullabilityBuffer;
        } else {
            this.nullabilityBuffer = sizeOrNullabilityBuffer;
            this._size = sizeOrNullabilityBuffer.size();
        }
    }

    getValue(index: number): K | null {
        return this.nullabilityBuffer && !this.nullabilityBuffer.get(index) ? null : this.getValueFromBuffer(index);
    }

    has(index: number): boolean {
        return (this.nullabilityBuffer && this.nullabilityBuffer.get(index)) || !this.nullabilityBuffer;
    }

    get name(): string {
        return this._name;
    }

    get size(): number {
        return this._size;
    }

    protected abstract getValueFromBuffer(index: number): K;
}
