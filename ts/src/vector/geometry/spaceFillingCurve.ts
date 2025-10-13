
export default abstract class SpaceFillingCurve {
    protected tileExtent: number;
    protected _numBits: number;
    protected _coordinateShift: number;
    private readonly minBound: number;
    private readonly maxBound: number;

    constructor(minVertexValue: number, maxVertexValue: number) {
        // TODO: fix tile buffer problem
        this._coordinateShift = minVertexValue < 0 ? Math.abs(minVertexValue) : 0;
        this.tileExtent = maxVertexValue + this._coordinateShift;
        this._numBits = Math.ceil(Math.log2(this.tileExtent));
        this.minBound = minVertexValue;
        this.maxBound = maxVertexValue;
    }

    protected validateCoordinates(vertex: { x: number, y: number }): void {
        // TODO: also check for int overflow as we are limiting the sfc ids to max int size
        if (vertex.x < this.minBound || vertex.y < this.minBound || vertex.x > this.maxBound || vertex.y > this.maxBound) {
            throw new Error("The specified tile buffer size is currently not supported.");
        }
    }

    abstract encode(vertex: { x: number, y: number }): number;

    abstract decode(mortonCode: number): { x: number, y: number };

    numBits(): number {
        return this._numBits;
    }

    coordinateShift(): number {
        return this._coordinateShift;
    }
}
