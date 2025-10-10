import BitVector from "./flat/bitVector";
import Vector from "./vector";

export abstract class FixedSizeVector<T extends ArrayBuffer, K> extends Vector<T, K> {
    protected constructor(name: string, dataBuffer: T, sizeOrNullabilityBuffer: number | BitVector) {
        super(name, dataBuffer, sizeOrNullabilityBuffer);
    }
}
