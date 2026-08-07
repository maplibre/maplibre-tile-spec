import Vector from "../vector";

/**
 * Holds already-decoded values of arbitrary shape, one per feature.
 *
 * Unlike the other vectors there is no packed buffer to index into: nested property (MAP) columns
 * decode to plain JavaScript maps, arrays and scalars, so the values are kept as-is. A `null` entry
 * means the property is absent for that feature.
 */
export class ObjectFlatVector extends Vector<Uint8Array, unknown> {
    constructor(
        name: string,
        private readonly values: unknown[],
    ) {
        super(name, new Uint8Array(0), values.length);
    }

    protected getValueFromBuffer(index: number): unknown {
        return this.values[index];
    }
}
