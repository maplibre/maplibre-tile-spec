import type BitVector from "./flat/bitVector";
import Vector from "./vector";

export abstract class VariableSizeVector<T extends ArrayBufferView, K> extends Vector<T, K> {
    protected constructor(
        name: string,
        protected offsetBuffer: Uint32Array,
        dataBuffer: T,
        sizeOrNullabilityBuffer: number | BitVector,
    ) {
        super(name, dataBuffer, sizeOrNullabilityBuffer);
    }
}
