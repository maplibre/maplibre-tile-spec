
export default class TopologyVector {

    //TODO: refactor to use unsigned integers
    constructor(
        private _geometryOffsets: Int32Array,
        private _partOffsets: Int32Array,
        private _ringOffsets: Int32Array
    ) {
    }

    get geometryOffsets(): Int32Array {
        return this._geometryOffsets;
    }

    get partOffsets(): Int32Array {
        return this._partOffsets;
    }

    get ringOffsets(): Int32Array {
        return this._ringOffsets;
    }
}
