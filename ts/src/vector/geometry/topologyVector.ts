export default class TopologyVector {
    constructor(
        private _geometryOffsets: Uint32Array,
        private _partOffsets: Uint32Array,
        private _ringOffsets: Uint32Array,
    ) {}

    get geometryOffsets(): Uint32Array {
        return this._geometryOffsets;
    }

    get partOffsets(): Uint32Array {
        return this._partOffsets;
    }

    get ringOffsets(): Uint32Array {
        return this._ringOffsets;
    }
}
