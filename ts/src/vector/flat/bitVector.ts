export default class BitVector {
    private readonly values: Uint8Array;
    private readonly _size: number;

    /**
     * @param values The byte buffer containing the bit values in least-significant bit (LSB)
     *     numbering
     */
    constructor(values: Uint8Array, size: number) {
        this.values = values;
        this._size = size;
    }

    get(index: number): boolean {
        const byteIndex = Math.floor(index / 8);
        const bitIndex = index % 8;
        const b = this.values[byteIndex];
        return ((b >> bitIndex) & 1) === 1;
    }

    set(index: number, value: boolean): void{
        //TODO: refactor -> improve quick and dirty solution
        const byteIndex = Math.floor(index / 8);
        const bitIndex = index % 8;
        this.values[byteIndex] = this.values[byteIndex] | ((value ? 1 : 0) << bitIndex);
    }

    getInt(index: number): number {
        const byteIndex = Math.floor(index / 8);
        const bitIndex = index % 8;
        const b = this.values[byteIndex];
        return (b >> bitIndex) & 1;
    }

    size(): number {
        return this._size;
    }

    getBuffer(): Uint8Array {
        return this.values;
    }
}
