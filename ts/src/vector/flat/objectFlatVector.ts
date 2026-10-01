import Vector from "../vector";
import type BitVector from "./bitVector";

/**
 * Holds already-decoded values of arbitrary shape, one per feature.
 *
 * Unlike the other vectors there is no packed buffer to index into: nested property (MAP) columns
 * decode to plain JavaScript maps, arrays and scalars, so the values are kept as-is. Features
 * without a value are marked absent in the nullability buffer, so `has` reports them as missing and
 * `getValue` returns `null`.
 */
export class ObjectFlatVector extends Vector<Uint8Array, unknown> {
    constructor(
        name: string,
        private readonly values: unknown[],
        nullabilityBuffer?: BitVector,
    ) {
        super(name, new Uint8Array(0), nullabilityBuffer ?? values.length);
    }

    protected getValueFromBuffer(index: number): unknown {
        return this.values[index];
    }
}
